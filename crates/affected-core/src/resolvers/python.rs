use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::debug;

use crate::resolvers::{file_to_package, Resolver};
use crate::types::{Ecosystem, Package, PackageId, ProjectGraph};

pub struct PythonResolver;

/// Which Python tooling is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonTooling {
    Generic,
    Poetry,
    Uv,
}

#[derive(Deserialize)]
struct PyProjectToml {
    project: Option<ProjectSection>,
    tool: Option<ToolSection>,
}

#[derive(Deserialize)]
struct ProjectSection {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ToolSection {
    poetry: Option<PoetrySection>,
    uv: Option<UvSection>,
}

#[derive(Deserialize)]
struct PoetrySection {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<toml::Value>,
}

#[derive(Deserialize)]
struct UvSection {
    workspace: Option<UvWorkspaceSection>,
}

#[derive(Deserialize)]
struct UvWorkspaceSection {
    members: Option<Vec<String>>,
}

impl PythonResolver {
    /// Detect which Python tooling is in use at the given root.
    fn detect_tooling(root: &Path) -> PythonTooling {
        let root_pyproject = root.join("pyproject.toml");
        if root_pyproject.exists() {
            if let Ok(content) = std::fs::read_to_string(&root_pyproject) {
                if content.contains("[tool.poetry]") {
                    debug!("Python tooling: Poetry");
                    return PythonTooling::Poetry;
                }
                if content.contains("[tool.uv.workspace]") {
                    debug!("Python tooling: uv");
                    return PythonTooling::Uv;
                }
            }
        }
        debug!("Python tooling: Generic");
        PythonTooling::Generic
    }
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
        let tooling = Self::detect_tooling(root);

        match tooling {
            PythonTooling::Poetry => self.resolve_poetry(root),
            PythonTooling::Uv => self.resolve_uv(root),
            PythonTooling::Generic => self.resolve_generic(root),
        }
    }

    fn package_for_file(&self, graph: &ProjectGraph, file: &Path) -> Option<PackageId> {
        file_to_package(graph, file)
    }

    fn test_command(&self, package_id: &PackageId) -> Vec<String> {
        // We need the root pyproject.toml to detect tooling, but we don't have
        // root info here. We use the generic command; users can override via config.
        // The find_affected_with_options path can use config overrides.
        vec![
            "python".into(),
            "-m".into(),
            "pytest".into(),
            package_id.0.clone(),
        ]
    }
}

impl PythonResolver {
    /// Resolve a Poetry-based project.
    fn resolve_poetry(&self, root: &Path) -> Result<ProjectGraph> {
        debug!("Resolving Poetry project at {}", root.display());

        // Find all pyproject.toml files
        let pkg_tomls = self.find_pyproject_tomls(root);

        let mut packages = HashMap::new();
        let mut name_to_id = HashMap::new();

        for toml_path in &pkg_tomls {
            let content = std::fs::read_to_string(toml_path)
                .with_context(|| format!("Failed to read {}", toml_path.display()))?;
            let pyproject: PyProjectToml = toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", toml_path.display()))?;

            // Poetry uses [tool.poetry] for name/version
            let name = pyproject
                .tool
                .as_ref()
                .and_then(|t| t.poetry.as_ref())
                .and_then(|p| p.name.clone())
                .or_else(|| pyproject.project.as_ref().and_then(|p| p.name.clone()));

            let name = match name {
                Some(n) => n,
                None => continue,
            };

            let version = pyproject
                .tool
                .as_ref()
                .and_then(|t| t.poetry.as_ref())
                .and_then(|p| p.version.clone())
                .or_else(|| pyproject.project.as_ref().and_then(|p| p.version.clone()));

            let pkg_dir = toml_path.parent().unwrap_or(root).to_path_buf();
            let pkg_id = PackageId(name.clone());
            name_to_id.insert(normalize_python_name(&name), pkg_id.clone());

            debug!("Poetry: discovered package '{}'", name);

            packages.insert(
                pkg_id.clone(),
                Package {
                    id: pkg_id,
                    name: name.clone(),
                    version,
                    path: pkg_dir,
                    manifest_path: toml_path.clone(),
                },
            );
        }

        // Build edges from Poetry dependencies
        let mut edges = Vec::new();
        let workspace_names: HashSet<String> = name_to_id.keys().cloned().collect();

        for toml_path in &pkg_tomls {
            let content = std::fs::read_to_string(toml_path)?;
            let pyproject: PyProjectToml = toml::from_str(&content)?;

            let from_name = pyproject
                .tool
                .as_ref()
                .and_then(|t| t.poetry.as_ref())
                .and_then(|p| p.name.clone())
                .or_else(|| pyproject.project.as_ref().and_then(|p| p.name.clone()));

            let from_name = match from_name {
                Some(n) => n,
                None => continue,
            };

            // Parse Poetry-style dependencies: {path = "../pkg-b", develop = true}
            if let Some(deps_value) = pyproject
                .tool
                .as_ref()
                .and_then(|t| t.poetry.as_ref())
                .and_then(|p| p.dependencies.as_ref())
            {
                if let Some(deps_table) = deps_value.as_table() {
                    for (dep_name, _dep_spec) in deps_table {
                        let normalized = normalize_python_name(dep_name);
                        if workspace_names.contains(&normalized) {
                            if let Some(to_id) = name_to_id.get(&normalized) {
                                edges.push((PackageId(from_name.clone()), to_id.clone()));
                            }
                        }
                    }
                }
            }

            // Also check standard PEP 621 dependencies
            if let Some(deps) = pyproject
                .project
                .as_ref()
                .and_then(|p| p.dependencies.as_ref())
            {
                for dep_str in deps {
                    let dep_name = parse_pep508_name(dep_str);
                    let normalized = normalize_python_name(&dep_name);
                    if workspace_names.contains(&normalized) {
                        if let Some(to_id) = name_to_id.get(&normalized) {
                            edges.push((PackageId(from_name.clone()), to_id.clone()));
                        }
                    }
                }
            }
        }

        edges.sort();
        edges.dedup();

        Ok(ProjectGraph {
            packages,
            edges,
            root: root.to_path_buf(),
        })
    }

    /// Resolve a uv workspace project.
    fn resolve_uv(&self, root: &Path) -> Result<ProjectGraph> {
        debug!("Resolving uv workspace at {}", root.display());

        let root_content = std::fs::read_to_string(root.join("pyproject.toml"))
            .context("Failed to read root pyproject.toml")?;
        let root_pyproject: PyProjectToml =
            toml::from_str(&root_content).context("Failed to parse root pyproject.toml")?;

        // Get workspace member globs from [tool.uv.workspace]
        let member_globs = root_pyproject
            .tool
            .as_ref()
            .and_then(|t| t.uv.as_ref())
            .and_then(|u| u.workspace.as_ref())
            .and_then(|w| w.members.clone())
            .unwrap_or_default();

        debug!("uv workspace member globs: {:?}", member_globs);

        // Expand member globs to find pyproject.toml files
        let mut pkg_tomls = Vec::new();
        for pattern in &member_globs {
            let full_pattern = root.join(pattern).join("pyproject.toml");
            if let Ok(paths) = glob::glob(full_pattern.to_str().unwrap_or("")) {
                for entry in paths.filter_map(|p| p.ok()) {
                    pkg_tomls.push(entry);
                }
            }
        }

        // Also include the root pyproject.toml if it has a [project] section
        if root_pyproject
            .project
            .as_ref()
            .and_then(|p| p.name.as_ref())
            .is_some()
        {
            pkg_tomls.push(root.join("pyproject.toml"));
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

            debug!("uv: discovered package '{}'", name);

            packages.insert(
                pkg_id.clone(),
                Package {
                    id: pkg_id,
                    name: name.clone(),
                    version: pyproject.project.as_ref().and_then(|p| p.version.clone()),
                    path: pkg_dir,
                    manifest_path: toml_path.clone(),
                },
            );
        }

        // Build edges from declared dependencies
        let mut edges = Vec::new();
        let workspace_names: HashSet<String> = name_to_id.keys().cloned().collect();

        for toml_path in &pkg_tomls {
            let content = std::fs::read_to_string(toml_path)?;
            let pyproject: PyProjectToml = toml::from_str(&content)?;

            let from_name = match pyproject.project.as_ref().and_then(|p| p.name.as_ref()) {
                Some(n) => n.clone(),
                None => continue,
            };

            if let Some(deps) = pyproject
                .project
                .as_ref()
                .and_then(|p| p.dependencies.as_ref())
            {
                for dep_str in deps {
                    let dep_name = parse_pep508_name(dep_str);
                    let normalized = normalize_python_name(&dep_name);
                    if workspace_names.contains(&normalized) {
                        if let Some(to_id) = name_to_id.get(&normalized) {
                            edges.push((PackageId(from_name.clone()), to_id.clone()));
                        }
                    }
                }
            }
        }

        edges.sort();
        edges.dedup();

        Ok(ProjectGraph {
            packages,
            edges,
            root: root.to_path_buf(),
        })
    }

    /// Resolve a generic Python monorepo (no Poetry or uv).
    fn resolve_generic(&self, root: &Path) -> Result<ProjectGraph> {
        debug!("Resolving generic Python project at {}", root.display());

        let pkg_tomls = self.find_pyproject_tomls(root);

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
                    version: pyproject.project.as_ref().and_then(|p| p.version.clone()),
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

            if let Some(deps) = pyproject
                .project
                .as_ref()
                .and_then(|p| p.dependencies.as_ref())
            {
                for dep_str in deps {
                    let dep_name = parse_pep508_name(dep_str);
                    let normalized = normalize_python_name(&dep_name);
                    if workspace_names.contains(&normalized) {
                        if let Some(to_id) = name_to_id.get(&normalized) {
                            edges.push((PackageId(from_name.clone()), to_id.clone()));
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

    /// Find all pyproject.toml files in subdirectories (up to 2 levels deep).
    fn find_pyproject_tomls(&self, root: &Path) -> Vec<std::path::PathBuf> {
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

        pkg_tomls
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_python_name() {
        assert_eq!(normalize_python_name("My-Package"), "my_package");
        assert_eq!(normalize_python_name("simple"), "simple");
        assert_eq!(normalize_python_name("UPPER-CASE"), "upper_case");
        assert_eq!(normalize_python_name("already_snake"), "already_snake");
    }

    #[test]
    fn test_parse_pep508_basic() {
        assert_eq!(parse_pep508_name("requests"), "requests");
        assert_eq!(parse_pep508_name("requests>=2.0"), "requests");
        assert_eq!(parse_pep508_name("my-package>=1.0,<2.0"), "my-package");
        assert_eq!(parse_pep508_name("pkg==1.0.0"), "pkg");
        assert_eq!(parse_pep508_name("my_pkg~=1.0"), "my_pkg");
    }

    #[test]
    fn test_parse_pep508_extras() {
        assert_eq!(parse_pep508_name("package[extra]>=1.0"), "package");
    }

    #[test]
    fn test_scan_python_imports_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("main.py"),
            "import os\nimport json\nfrom pathlib import Path\n",
        )
        .unwrap();

        let imports = scan_python_imports(dir.path());
        assert!(imports.contains("os"));
        assert!(imports.contains("json"));
        assert!(imports.contains("pathlib"));
    }

    #[test]
    fn test_scan_python_imports_multiline() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("app.py"),
            "import foo, bar\nfrom baz.sub import thing\n",
        )
        .unwrap();

        let imports = scan_python_imports(dir.path());
        assert!(imports.contains("foo"));
        assert!(imports.contains("bar"));
        assert!(imports.contains("baz"));
    }

    #[test]
    fn test_scan_python_imports_skips_relative() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mod.py"),
            "from . import sibling\nfrom ..parent import thing\nimport real_dep\n",
        )
        .unwrap();

        let imports = scan_python_imports(dir.path());
        assert!(!imports.contains("sibling"));
        assert!(!imports.contains("parent"));
        assert!(imports.contains("real_dep"));
    }

    #[test]
    fn test_scan_python_imports_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/pkg")).unwrap();
        std::fs::write(dir.path().join("src/pkg/core.py"), "import numpy\n").unwrap();

        let imports = scan_python_imports(dir.path());
        assert!(imports.contains("numpy"));
    }

    #[test]
    fn test_scan_python_imports_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let imports = scan_python_imports(dir.path());
        assert!(imports.is_empty());
    }

    #[test]
    fn test_detect_python_root_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"myapp\"\n",
        )
        .unwrap();

        assert!(PythonResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_no_python() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!PythonResolver.detect(dir.path()));
    }

    #[test]
    fn test_resolve_python_monorepo() {
        let dir = tempfile::tempdir().unwrap();

        // Root pyproject.toml (for detection)
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"root\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        // Package A depends on Package B
        std::fs::create_dir_all(dir.path().join("pkg-a")).unwrap();
        std::fs::write(
            dir.path().join("pkg-a/pyproject.toml"),
            "[project]\nname = \"pkg-a\"\nversion = \"0.1.0\"\ndependencies = [\"pkg-b>=0.1\"]\n",
        )
        .unwrap();

        // Package B
        std::fs::create_dir_all(dir.path().join("pkg-b")).unwrap();
        std::fs::write(
            dir.path().join("pkg-b/pyproject.toml"),
            "[project]\nname = \"pkg-b\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let graph = PythonResolver.resolve(dir.path()).unwrap();

        // Should find root + pkg-a + pkg-b
        assert!(graph.packages.len() >= 2);
        assert!(graph.packages.contains_key(&PackageId("pkg-a".into())));
        assert!(graph.packages.contains_key(&PackageId("pkg-b".into())));

        // pkg-a depends on pkg-b
        assert!(graph
            .edges
            .contains(&(PackageId("pkg-a".into()), PackageId("pkg-b".into()),)));
    }

    #[test]
    fn test_resolve_python_with_import_scanning() {
        let dir = tempfile::tempdir().unwrap();

        // Package A imports package B via code
        std::fs::create_dir_all(dir.path().join("alpha/src")).unwrap();
        std::fs::write(
            dir.path().join("alpha/pyproject.toml"),
            "[project]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("alpha/src/main.py"),
            "import beta\nfrom beta.utils import helper\n",
        )
        .unwrap();

        // Package B
        std::fs::create_dir_all(dir.path().join("beta")).unwrap();
        std::fs::write(
            dir.path().join("beta/pyproject.toml"),
            "[project]\nname = \"beta\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let graph = PythonResolver.resolve(dir.path()).unwrap();

        assert!(graph.packages.contains_key(&PackageId("alpha".into())));
        assert!(graph.packages.contains_key(&PackageId("beta".into())));

        // alpha imports beta
        assert!(graph
            .edges
            .contains(&(PackageId("alpha".into()), PackageId("beta".into()),)));
    }

    #[test]
    fn test_test_command() {
        let cmd = PythonResolver.test_command(&PackageId("my-pkg".into()));
        assert_eq!(cmd, vec!["python", "-m", "pytest", "my-pkg"]);
    }

    #[test]
    fn test_detect_tooling_generic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"myapp\"\n",
        )
        .unwrap();
        assert_eq!(
            PythonResolver::detect_tooling(dir.path()),
            PythonTooling::Generic
        );
    }

    #[test]
    fn test_detect_tooling_poetry() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[tool.poetry]\nname = \"myapp\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(
            PythonResolver::detect_tooling(dir.path()),
            PythonTooling::Poetry
        );
    }

    #[test]
    fn test_detect_tooling_uv() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"root\"\n\n[tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
        )
        .unwrap();
        assert_eq!(
            PythonResolver::detect_tooling(dir.path()),
            PythonTooling::Uv
        );
    }

    #[test]
    fn test_resolve_poetry_project() {
        let dir = tempfile::tempdir().unwrap();

        // Root pyproject.toml with Poetry
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[tool.poetry]\nname = \"root\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        // Package A with Poetry-style deps
        std::fs::create_dir_all(dir.path().join("pkg-a")).unwrap();
        std::fs::write(
            dir.path().join("pkg-a/pyproject.toml"),
            "[tool.poetry]\nname = \"pkg-a\"\nversion = \"0.1.0\"\n\n[tool.poetry.dependencies]\npython = \"^3.9\"\npkg-b = {path = \"../pkg-b\", develop = true}\n",
        )
        .unwrap();

        // Package B
        std::fs::create_dir_all(dir.path().join("pkg-b")).unwrap();
        std::fs::write(
            dir.path().join("pkg-b/pyproject.toml"),
            "[tool.poetry]\nname = \"pkg-b\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let graph = PythonResolver.resolve(dir.path()).unwrap();
        assert!(graph.packages.contains_key(&PackageId("pkg-a".into())));
        assert!(graph.packages.contains_key(&PackageId("pkg-b".into())));

        // pkg-a depends on pkg-b
        assert!(graph
            .edges
            .contains(&(PackageId("pkg-a".into()), PackageId("pkg-b".into()),)));
    }

    #[test]
    fn test_resolve_uv_workspace() {
        let dir = tempfile::tempdir().unwrap();

        // Root pyproject.toml with uv workspace
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n[tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
        )
        .unwrap();

        // Package A depends on Package B
        std::fs::create_dir_all(dir.path().join("packages/pkg-a")).unwrap();
        std::fs::write(
            dir.path().join("packages/pkg-a/pyproject.toml"),
            "[project]\nname = \"pkg-a\"\nversion = \"0.1.0\"\ndependencies = [\"pkg-b>=0.1\"]\n",
        )
        .unwrap();

        // Package B
        std::fs::create_dir_all(dir.path().join("packages/pkg-b")).unwrap();
        std::fs::write(
            dir.path().join("packages/pkg-b/pyproject.toml"),
            "[project]\nname = \"pkg-b\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let graph = PythonResolver.resolve(dir.path()).unwrap();
        assert!(graph.packages.contains_key(&PackageId("pkg-a".into())));
        assert!(graph.packages.contains_key(&PackageId("pkg-b".into())));

        // pkg-a depends on pkg-b
        assert!(graph
            .edges
            .contains(&(PackageId("pkg-a".into()), PackageId("pkg-b".into()),)));
    }
}
