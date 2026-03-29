use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::resolvers::{file_to_package, Resolver};
use crate::types::{Ecosystem, Package, PackageId, ProjectGraph};

pub struct GoResolver;

impl Resolver for GoResolver {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Go
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("go.work").exists() || root.join("go.mod").exists()
    }

    fn resolve(&self, root: &Path) -> Result<ProjectGraph> {
        if root.join("go.work").exists() {
            self.resolve_workspace(root)
        } else {
            self.resolve_single_module(root)
        }
    }

    fn package_for_file(&self, graph: &ProjectGraph, file: &Path) -> Option<PackageId> {
        file_to_package(graph, file)
    }

    fn test_command(&self, package_id: &PackageId) -> Vec<String> {
        vec![
            "go".into(),
            "test".into(),
            format!("./{}/...", package_id.0),
        ]
    }
}

impl GoResolver {
    /// Resolve a Go workspace (go.work file).
    fn resolve_workspace(&self, root: &Path) -> Result<ProjectGraph> {
        let go_work = std::fs::read_to_string(root.join("go.work"))
            .context("Failed to read go.work")?;

        // Parse `use` directives from go.work
        let module_dirs = parse_go_work_uses(&go_work);

        let mut packages = HashMap::new();
        let mut module_path_to_id = HashMap::new();

        for dir_str in &module_dirs {
            let dir = root.join(dir_str);
            let go_mod_path = dir.join("go.mod");
            if !go_mod_path.exists() {
                continue;
            }

            let go_mod = std::fs::read_to_string(&go_mod_path)
                .with_context(|| format!("Failed to read {}", go_mod_path.display()))?;

            let module_path = parse_go_mod_module(&go_mod)
                .with_context(|| format!("No module directive in {}", go_mod_path.display()))?;

            // Use the directory name as the PackageId for simplicity
            let pkg_id = PackageId(dir_str.clone());
            module_path_to_id.insert(module_path.clone(), pkg_id.clone());

            packages.insert(
                pkg_id.clone(),
                Package {
                    id: pkg_id,
                    name: module_path,
                    version: None,
                    path: dir.clone(),
                    manifest_path: go_mod_path,
                },
            );
        }

        // Run `go mod graph` to get dependency edges
        let edges = self.parse_mod_graph(root, &module_path_to_id)?;

        Ok(ProjectGraph {
            packages,
            edges,
            root: root.to_path_buf(),
        })
    }

    /// Resolve a single Go module (just go.mod, no workspace).
    fn resolve_single_module(&self, root: &Path) -> Result<ProjectGraph> {
        let go_mod = std::fs::read_to_string(root.join("go.mod"))
            .context("Failed to read go.mod")?;

        let module_path = parse_go_mod_module(&go_mod)
            .context("No module directive found in go.mod")?;

        let pkg_id = PackageId(".".to_string());
        let mut packages = HashMap::new();
        packages.insert(
            pkg_id.clone(),
            Package {
                id: pkg_id,
                name: module_path,
                version: None,
                path: root.to_path_buf(),
                manifest_path: root.join("go.mod"),
            },
        );

        // Single module has no internal dependency edges
        Ok(ProjectGraph {
            packages,
            edges: vec![],
            root: root.to_path_buf(),
        })
    }

    /// Parse `go mod graph` output and filter to workspace modules.
    fn parse_mod_graph(
        &self,
        root: &Path,
        module_path_to_id: &HashMap<String, PackageId>,
    ) -> Result<Vec<(PackageId, PackageId)>> {
        let output = Command::new("go")
            .args(["mod", "graph"])
            .current_dir(root)
            .output()
            .context("Failed to run 'go mod graph'. Is Go installed?")?;

        if !output.status.success() {
            // Non-fatal: just return no edges
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut edges = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 2 {
                continue;
            }

            // Strip version: "module@v1.0.0" -> "module"
            let from_mod = parts[0].split('@').next().unwrap_or(parts[0]);
            let to_mod = parts[1].split('@').next().unwrap_or(parts[1]);

            if let (Some(from_id), Some(to_id)) = (
                module_path_to_id.get(from_mod),
                module_path_to_id.get(to_mod),
            ) {
                edges.push((from_id.clone(), to_id.clone()));
            }
        }

        Ok(edges)
    }
}

/// Parse `use` directives from go.work content.
fn parse_go_work_uses(content: &str) -> Vec<String> {
    let mut uses = Vec::new();
    let mut in_use_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("use ") && !trimmed.contains('(') {
            // Single-line use: `use ./path`
            let path = trimmed
                .trim_start_matches("use ")
                .trim()
                .trim_matches('.')
                .trim_start_matches('/')
                .to_string();
            if !path.is_empty() {
                uses.push(path);
            } else {
                // Handle `use ./path` -> just `path`
                let raw = trimmed.trim_start_matches("use ").trim();
                let cleaned = raw.trim_start_matches("./").to_string();
                if !cleaned.is_empty() {
                    uses.push(cleaned);
                }
            }
            continue;
        }

        if trimmed == "use (" {
            in_use_block = true;
            continue;
        }

        if in_use_block {
            if trimmed == ")" {
                in_use_block = false;
                continue;
            }
            let path = trimmed.trim_start_matches("./").to_string();
            if !path.is_empty() {
                uses.push(path);
            }
        }
    }

    uses
}

/// Parse the `module` directive from go.mod content.
fn parse_go_mod_module(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("module ") {
            return Some(trimmed.trim_start_matches("module ").trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_go_work() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n").unwrap();
        assert!(GoResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_go_mod() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/foo\n").unwrap();
        assert!(GoResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_no_go_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!GoResolver.detect(dir.path()));
    }

    #[test]
    fn test_parse_go_mod_module_basic() {
        let content = "module example.com/mymod\n\ngo 1.21\n";
        assert_eq!(
            parse_go_mod_module(content),
            Some("example.com/mymod".into())
        );
    }

    #[test]
    fn test_parse_go_mod_module_with_whitespace() {
        let content = "  module   example.com/foo  \n";
        assert_eq!(
            parse_go_mod_module(content),
            Some("example.com/foo".into())
        );
    }

    #[test]
    fn test_parse_go_mod_module_not_found() {
        let content = "go 1.21\nrequire example.com/bar v1.0.0\n";
        assert!(parse_go_mod_module(content).is_none());
    }

    #[test]
    fn test_parse_go_work_uses_block() {
        let content = "go 1.21\n\nuse (\n\t./svc-a\n\t./svc-b\n)\n";
        let uses = parse_go_work_uses(content);
        assert_eq!(uses, vec!["svc-a", "svc-b"]);
    }

    #[test]
    fn test_parse_go_work_uses_single_line() {
        let content = "go 1.21\nuse ./mymod\n";
        let uses = parse_go_work_uses(content);
        assert_eq!(uses, vec!["mymod"]);
    }

    #[test]
    fn test_parse_go_work_uses_empty() {
        let content = "go 1.21\n";
        let uses = parse_go_work_uses(content);
        assert!(uses.is_empty());
    }

    #[test]
    fn test_parse_go_work_uses_mixed() {
        let content = "go 1.21\n\nuse ./standalone\n\nuse (\n\t./a\n\t./b\n)\n";
        let uses = parse_go_work_uses(content);
        assert_eq!(uses.len(), 3);
        assert!(uses.contains(&"standalone".to_string()));
        assert!(uses.contains(&"a".to_string()));
        assert!(uses.contains(&"b".to_string()));
    }

    #[test]
    fn test_resolve_single_module() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/solo\n\ngo 1.21\n",
        )
        .unwrap();

        let graph = GoResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 1);
        let pkg = graph.packages.values().next().unwrap();
        assert_eq!(pkg.name, "example.com/solo");
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_test_command() {
        let cmd = GoResolver.test_command(&PackageId("svc-a".into()));
        assert_eq!(cmd, vec!["go", "test", "./svc-a/..."]);
    }
}
