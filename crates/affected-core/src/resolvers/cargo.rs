use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::resolvers::{file_to_package, Resolver};
use crate::types::{Ecosystem, Package, PackageId, ProjectGraph};

pub struct CargoResolver;

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
    resolve: Option<CargoResolve>,
    workspace_root: String,
}

#[derive(Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
    version: String,
    manifest_path: String,
}

#[derive(Deserialize)]
struct CargoResolve {
    nodes: Vec<CargoNode>,
}

#[derive(Deserialize)]
struct CargoNode {
    id: String,
    dependencies: Vec<String>,
}

impl Resolver for CargoResolver {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Cargo
    }

    fn detect(&self, root: &Path) -> bool {
        let cargo_toml = root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return false;
        }
        std::fs::read_to_string(&cargo_toml)
            .map(|c| c.contains("[workspace]"))
            .unwrap_or(false)
    }

    fn resolve(&self, root: &Path) -> Result<ProjectGraph> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1"])
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .output()
            .context("Failed to run 'cargo metadata'. Is cargo installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo metadata failed: {stderr}");
        }

        let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
            .context("Failed to parse cargo metadata JSON")?;

        let workspace_root = PathBuf::from(&metadata.workspace_root);

        // Collect workspace member IDs for filtering
        let member_ids: HashSet<&str> = metadata
            .workspace_members
            .iter()
            .map(|s| s.as_str())
            .collect();

        // Build packages (only workspace members)
        let mut packages = HashMap::new();
        let mut id_to_name = HashMap::new();

        for pkg in &metadata.packages {
            if member_ids.contains(pkg.id.as_str()) {
                let manifest = PathBuf::from(&pkg.manifest_path);
                let pkg_path = manifest.parent().unwrap_or(&workspace_root).to_path_buf();
                let pkg_id = PackageId(pkg.name.clone());

                id_to_name.insert(pkg.id.clone(), pkg.name.clone());
                packages.insert(
                    pkg_id.clone(),
                    Package {
                        id: pkg_id,
                        name: pkg.name.clone(),
                        version: Some(pkg.version.clone()),
                        path: pkg_path,
                        manifest_path: manifest,
                    },
                );
            }
        }

        // Build edges from resolve graph (only between workspace members)
        let mut edges = Vec::new();
        if let Some(resolve) = &metadata.resolve {
            for node in &resolve.nodes {
                let from_name = match id_to_name.get(&node.id) {
                    Some(n) => n,
                    None => continue,
                };

                for dep_id in &node.dependencies {
                    if let Some(to_name) = id_to_name.get(dep_id) {
                        edges.push((PackageId(from_name.clone()), PackageId(to_name.clone())));
                    }
                }
            }
        }

        Ok(ProjectGraph {
            packages,
            edges,
            root: workspace_root,
        })
    }

    fn package_for_file(&self, graph: &ProjectGraph, file: &Path) -> Option<PackageId> {
        file_to_package(graph, file)
    }

    fn test_command(&self, package_id: &PackageId) -> Vec<String> {
        vec![
            "cargo".into(),
            "test".into(),
            "-p".into(),
            package_id.0.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cargo_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();

        assert!(CargoResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_no_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"solo\"\n",
        )
        .unwrap();

        assert!(!CargoResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_no_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!CargoResolver.detect(dir.path()));
    }

    #[test]
    fn test_test_command_format() {
        let cmd = CargoResolver.test_command(&PackageId("my-crate".into()));
        assert_eq!(cmd, vec!["cargo", "test", "-p", "my-crate"]);
    }

    #[test]
    fn test_resolve_on_self() {
        // Test resolving the `affected` project itself
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();

        if !root.join("Cargo.toml").exists() {
            return; // Skip if not run from the workspace
        }

        let graph = CargoResolver.resolve(root).unwrap();
        assert!(graph.packages.len() >= 2);
        assert!(graph
            .packages
            .contains_key(&PackageId("affected-core".into())));
        assert!(graph
            .packages
            .contains_key(&PackageId("affected-cli".into())));
        // affected-cli depends on affected-core
        assert!(graph.edges.contains(&(
            PackageId("affected-cli".into()),
            PackageId("affected-core".into()),
        )));
    }

    #[test]
    fn test_package_for_file_in_workspace() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();

        if !root.join("Cargo.toml").exists() {
            return;
        }

        let graph = CargoResolver.resolve(root).unwrap();
        let result = CargoResolver
            .package_for_file(&graph, &PathBuf::from("crates/affected-core/src/lib.rs"));
        assert_eq!(result, Some(PackageId("affected-core".into())));
    }
}
