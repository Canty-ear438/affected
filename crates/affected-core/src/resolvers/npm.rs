use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::resolvers::{file_to_package, Resolver};
use crate::types::{Ecosystem, Package, PackageId, ProjectGraph};

pub struct NpmResolver;

#[derive(Deserialize)]
struct RootPackageJson {
    workspaces: Option<WorkspacesField>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WorkspacesField {
    Array(Vec<String>),
    Object { packages: Vec<String> },
}

#[derive(Deserialize)]
struct PackageJson {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
}

impl Resolver for NpmResolver {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Npm
    }

    fn detect(&self, root: &Path) -> bool {
        if root.join("pnpm-workspace.yaml").exists() {
            return true;
        }
        let pkg = root.join("package.json");
        if !pkg.exists() {
            return false;
        }
        std::fs::read_to_string(&pkg)
            .map(|c| c.contains("\"workspaces\""))
            .unwrap_or(false)
    }

    fn resolve(&self, root: &Path) -> Result<ProjectGraph> {
        let workspace_globs = self.find_workspace_globs(root)?;
        let pkg_dirs = self.expand_globs(root, &workspace_globs)?;

        // Parse all workspace packages
        let mut packages = HashMap::new();
        let mut name_to_id = HashMap::new();

        for dir in &pkg_dirs {
            let pkg_json_path = dir.join("package.json");
            if !pkg_json_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&pkg_json_path)
                .with_context(|| format!("Failed to read {}", pkg_json_path.display()))?;
            let pkg: PackageJson = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse {}", pkg_json_path.display()))?;

            let name = match &pkg.name {
                Some(n) => n.clone(),
                None => continue,
            };

            let pkg_id = PackageId(name.clone());
            name_to_id.insert(name.clone(), pkg_id.clone());
            packages.insert(
                pkg_id.clone(),
                Package {
                    id: pkg_id,
                    name: name.clone(),
                    version: pkg.version.clone(),
                    path: dir.clone(),
                    manifest_path: pkg_json_path,
                },
            );
        }

        // Build dependency edges
        let mut edges = Vec::new();
        let workspace_names: std::collections::HashSet<&str> =
            name_to_id.keys().map(|s| s.as_str()).collect();

        for dir in &pkg_dirs {
            let pkg_json_path = dir.join("package.json");
            if !pkg_json_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&pkg_json_path)?;
            let pkg: PackageJson = serde_json::from_str(&content)?;

            let from_name = match &pkg.name {
                Some(n) => n.clone(),
                None => continue,
            };

            // Check both dependencies and devDependencies
            let all_deps: Vec<&str> = pkg
                .dependencies
                .iter()
                .flat_map(|d| d.keys())
                .chain(pkg.dev_dependencies.iter().flat_map(|d| d.keys()))
                .map(|s| s.as_str())
                .collect();

            for dep_name in all_deps {
                if workspace_names.contains(dep_name) {
                    edges.push((
                        PackageId(from_name.clone()),
                        PackageId(dep_name.to_string()),
                    ));
                }
            }
        }

        Ok(ProjectGraph {
            packages,
            edges,
            root: root.to_path_buf(),
        })
    }

    fn package_for_file(&self, graph: &ProjectGraph, file: &Path) -> Option<PackageId> {
        file_to_package(graph, file)
    }

    fn test_command(&self, package_id: &PackageId) -> Vec<String> {
        // Default to npm; users can override via .affected.toml
        vec![
            "npm".into(),
            "test".into(),
            "--workspace".into(),
            package_id.0.clone(),
        ]
    }
}

impl NpmResolver {
    fn find_workspace_globs(&self, root: &Path) -> Result<Vec<String>> {
        // Try pnpm-workspace.yaml first
        let pnpm_path = root.join("pnpm-workspace.yaml");
        if pnpm_path.exists() {
            let content = std::fs::read_to_string(&pnpm_path)?;
            // Simple YAML parsing for the packages field
            // pnpm-workspace.yaml is typically:
            //   packages:
            //     - 'packages/*'
            //     - 'apps/*'
            let mut globs = Vec::new();
            let mut in_packages = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed == "packages:" {
                    in_packages = true;
                    continue;
                }
                if in_packages {
                    if trimmed.starts_with("- ") {
                        let glob = trimmed
                            .trim_start_matches("- ")
                            .trim_matches('\'')
                            .trim_matches('"')
                            .to_string();
                        globs.push(glob);
                    } else if !trimmed.is_empty() {
                        break;
                    }
                }
            }
            if !globs.is_empty() {
                return Ok(globs);
            }
        }

        // Fall back to package.json workspaces
        let pkg_path = root.join("package.json");
        let content = std::fs::read_to_string(&pkg_path)
            .context("No package.json found")?;
        let root_pkg: RootPackageJson = serde_json::from_str(&content)
            .context("Failed to parse root package.json")?;

        match root_pkg.workspaces {
            Some(WorkspacesField::Array(globs)) => Ok(globs),
            Some(WorkspacesField::Object { packages }) => Ok(packages),
            None => anyhow::bail!("No workspaces field found in package.json"),
        }
    }

    fn expand_globs(&self, root: &Path, globs: &[String]) -> Result<Vec<PathBuf>> {
        let mut dirs = Vec::new();

        for pattern in globs {
            let full_pattern = root.join(pattern).join("package.json");
            let pattern_str = full_pattern.to_str().unwrap_or("");

            match glob::glob(pattern_str) {
                Ok(paths) => {
                    for entry in paths.filter_map(|p| p.ok()) {
                        if let Some(parent) = entry.parent() {
                            dirs.push(parent.to_path_buf());
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(dirs)
    }
}
