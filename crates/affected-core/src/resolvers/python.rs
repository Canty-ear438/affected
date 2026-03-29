use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::resolvers::{file_to_package, Resolver};
use crate::types::{Ecosystem, Package, PackageId, ProjectGraph};

pub struct PythonResolver;

#[derive(Deserialize)]
struct PyProjectToml {
    project: Option<ProjectSection>,
}

#[derive(Deserialize)]
struct ProjectSection {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<Vec<String>>,
}

impl Resolver for PythonResolver {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Python
    }

    fn detect(&self, root: &Path) -> bool {
        if root.join("pyproject.toml").exists() {
            return true;
        }
        // Check for multiple pyproject.toml in subdirectories
        let pattern = root.join("*/pyproject.toml");
        if let Ok(paths) = glob::glob(pattern.to_str().unwrap_or("")) {
            return paths.filter_map(|p| p.ok()).count() >= 2;
        }
        false
    }

    fn resolve(&self, root: &Path) -> Result<ProjectGraph> {
        // Find all pyproject.toml files (root + subdirectories)
        let mut pkg_tomls = Vec::new();

        let pattern = root.join("*/pyproject.toml");
        if let Ok(paths) = glob::glob(pattern.to_str().unwrap_or("")) {
            for entry in paths.filter_map(|p| p.ok()) {
                pkg_tomls.push(entry);
            }
        }

        // Also check two levels deep
        let pattern2 = root.join("*/*/pyproject.toml");
        if let Ok(paths) = glob::glob(pattern2.to_str().unwrap_or("")) {
            for entry in paths.filter_map(|p| p.ok()) {
                pkg_tomls.push(entry);
            }
        }

        if pkg_tomls.is_empty() {
            // Single project at root
            let root_toml = root.join("pyproject.toml");
            if root_toml.exists() {
                pkg_tomls.push(root_toml);
            }
        }

        let mut packages = HashMap::new();
        let mut name_to_id = HashMap::new();

        for toml_path in &pkg_tomls {
            let content = std::fs::read_to_string(toml_path)
                .with_context(|| format!("Failed to read {}", toml_path.display()))?;
            let pyproject: PyProjectToml = toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", toml_path.display()))?;

            let name = match pyproject.project.as_ref().and_then(|p| p.name.as_ref()) {
                Some(n) => n.clone(),
                None => continue,
            };

            let pkg_dir = toml_path.parent().unwrap_or(root).to_path_buf();
            let pkg_id = PackageId(name.clone());
            name_to_id.insert(normalize_python_name(&name), pkg_id.clone());

            packages.insert(
                pkg_id.clone(),
                Package {
                    id: pkg_id,
                    name: name.clone(),
                    version: pyproject
                        .project
                        .as_ref()
                        .and_then(|p| p.version.clone()),
                    path: pkg_dir,
                    manifest_path: toml_path.clone(),
                },
            );
        }

        // Build edges from declared dependencies + import scanning
        let mut edges = Vec::new();
        let workspace_names: HashSet<String> = name_to_id.keys().cloned().collect();

        // Strategy 1: Declared dependencies in pyproject.toml
        for toml_path in &pkg_tomls {
            let content = std::fs::read_to_string(toml_path)?;
            let pyproject: PyProjectToml = toml::from_str(&content)?;

            let from_name = match pyproject.project.as_ref().and_then(|p| p.name.as_ref()) {
                Some(n) => n.clone(),
                None => continue,
            };

            if let Some(deps) = pyproject.project.as_ref().and_then(|p| p.dependencies.as_ref()) {
                for dep_str in deps {
                    let dep_name = parse_pep508_name(dep_str);
                    let normalized = normalize_python_name(&dep_name);
                    if workspace_names.contains(&normalized) {
                        if let Some(to_id) = name_to_id.get(&normalized) {
                            edges.push((
                                PackageId(from_name.clone()),
                                to_id.clone(),
                            ));
                        }
                    }
                }
            }
        }

        // Strategy 2: Import scanning
        for (pkg_id, pkg) in &packages {
            let imports = scan_python_imports(&pkg.path);
            for import_name in imports {
                let normalized = normalize_python_name(&import_name);
                if let Some(to_id) = name_to_id.get(&normalized) {
                    if to_id != pkg_id {
                        edges.push((pkg_id.clone(), to_id.clone()));
                    }
                }
            }
        }

        // Deduplicate edges
        edges.sort();
        edges.dedup();

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
        vec![
            "python".into(),
            "-m".into(),
            "pytest".into(),
            package_id.0.clone(),
        ]
    }
}

/// Normalize a Python package name: lowercase, replace hyphens with underscores.
fn normalize_python_name(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

/// Parse the package name from a PEP 508 dependency string.
/// e.g., "my-package>=1.0" -> "my-package"
fn parse_pep508_name(dep: &str) -> String {
    let name: String = dep
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    name
}

/// Scan Python files in a directory for import statements.
/// Returns top-level module names that are imported.
fn scan_python_imports(dir: &Path) -> HashSet<String> {
    let mut imports = HashSet::new();
    let pattern = dir.join("**/*.py");

    let paths = match glob::glob(pattern.to_str().unwrap_or("")) {
        Ok(p) => p,
        Err(_) => return imports,
    };

    for entry in paths.filter_map(|p| p.ok()) {
        let content = match std::fs::read_to_string(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for line in content.lines() {
            let trimmed = line.trim();

            // `import foo` or `import foo.bar`
            if trimmed.starts_with("import ") && !trimmed.contains('(') {
                let rest = trimmed.trim_start_matches("import ").trim();
                // Handle `import foo, bar`
                for part in rest.split(',') {
                    let module = part.trim().split('.').next().unwrap_or("").trim();
                    if !module.is_empty() && module.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        imports.insert(module.to_string());
                    }
                }
            }
            // `from foo import bar` or `from foo.baz import bar`
            else if trimmed.starts_with("from ") && trimmed.contains(" import ") {
                let rest = trimmed.trim_start_matches("from ").trim();
                // Skip relative imports (from . or from ..)
                if rest.starts_with('.') {
                    continue;
                }
                let module = rest.split_whitespace().next().unwrap_or("");
                let top_level = module.split('.').next().unwrap_or("").trim();
                if !top_level.is_empty()
                    && top_level.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    imports.insert(top_level.to_string());
                }
            }
        }
    }

    imports
}
