use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

use crate::resolvers::{file_to_package, Resolver};
use crate::types::{Ecosystem, Package, PackageId, ProjectGraph};

/// GradleResolver detects Gradle multi-project builds via `settings.gradle(.kts)`.
///
/// Uses regex to parse `include` directives and `project(':...')` dependency references.
pub struct GradleResolver;

impl Resolver for GradleResolver {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Gradle
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("settings.gradle").exists() || root.join("settings.gradle.kts").exists()
    }

    fn resolve(&self, root: &Path) -> Result<ProjectGraph> {
        let settings_content = self.read_settings(root)?;
        let module_names = parse_include_directives(&settings_content);

        tracing::debug!(
            "Gradle: found {} included modules: {:?}",
            module_names.len(),
            module_names
        );

        let mut packages = HashMap::new();

        for module_name in &module_names {
            // Gradle module ':foo' maps to directory 'foo'
            // Gradle module ':foo:bar' maps to directory 'foo/bar'
            let dir_path = module_name.replace(':', "/");
            let module_dir = root.join(&dir_path);

            if !module_dir.exists() {
                tracing::debug!(
                    "Gradle: module '{}' directory does not exist, skipping",
                    module_name
                );
                continue;
            }

            // Find the build file for this module
            let build_file = if module_dir.join("build.gradle.kts").exists() {
                module_dir.join("build.gradle.kts")
            } else if module_dir.join("build.gradle").exists() {
                module_dir.join("build.gradle")
            } else {
                tracing::debug!(
                    "Gradle: module '{}' has no build.gradle(.kts), skipping",
                    module_name
                );
                continue;
            };

            let pkg_id = PackageId(module_name.clone());
            packages.insert(
                pkg_id.clone(),
                Package {
                    id: pkg_id,
                    name: module_name.clone(),
                    version: None,
                    path: module_dir,
                    manifest_path: build_file,
                },
            );
        }

        // Build dependency edges by scanning build.gradle(.kts) for project(':...') references
        let mut edges = Vec::new();
        let module_set: std::collections::HashSet<&str> =
            module_names.iter().map(|s| s.as_str()).collect();

        for (pkg_id, pkg) in &packages {
            let build_content = std::fs::read_to_string(&pkg.manifest_path)
                .with_context(|| {
                    format!("Failed to read {}", pkg.manifest_path.display())
                })?;

            let project_refs = parse_project_dependencies(&build_content);

            for dep_name in &project_refs {
                if module_set.contains(dep_name.as_str()) && dep_name != &pkg_id.0 {
                    edges.push((pkg_id.clone(), PackageId(dep_name.clone())));
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
        vec![
            "gradle".into(),
            format!(":{}:test", package_id.0),
        ]
    }
}

impl GradleResolver {
    /// Read settings.gradle or settings.gradle.kts from the root.
    fn read_settings(&self, root: &Path) -> Result<String> {
        let kts_path = root.join("settings.gradle.kts");
        if kts_path.exists() {
            return std::fs::read_to_string(&kts_path)
                .context("Failed to read settings.gradle.kts");
        }

        let groovy_path = root.join("settings.gradle");
        std::fs::read_to_string(&groovy_path).context("Failed to read settings.gradle")
    }
}

/// Parse `include` directives from a settings.gradle(.kts) file.
///
/// Handles these patterns:
/// - `include ':module-a'`
/// - `include ':module-a', ':module-b'`
/// - `include(":module-a")`
/// - `include(":module-a", ":module-b")`
fn parse_include_directives(content: &str) -> Vec<String> {
    let mut modules = Vec::new();

    // Match quoted module names after include (both Groovy and Kotlin DSL forms).
    // Captures the colon-prefixed module name inside quotes.
    let re = Regex::new(r#"include\s*\(?\s*(?:['"]:[\w-]+['"]\s*,\s*)*['"]:?([\w-]+)['"]"#)
        .unwrap();

    // A simpler approach: find all quoted :module references on lines starting with include
    let module_re = Regex::new(r#"['"]:([\w-]+)['"]"#).unwrap();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("include") {
            continue;
        }

        // Extract all module names from this include line
        for cap in module_re.captures_iter(trimmed) {
            if let Some(name) = cap.get(1) {
                let module_name = name.as_str().to_string();
                if !modules.contains(&module_name) {
                    modules.push(module_name);
                }
            }
        }
    }

    // Suppress unused variable warning
    let _ = re;

    modules
}

/// Parse `project(':...')` references from a build.gradle(.kts) file.
///
/// Matches patterns like:
/// - `project(':core')`
/// - `project(":core")`
/// - `project(':sub-module')`
fn parse_project_dependencies(content: &str) -> Vec<String> {
    let re = Regex::new(r#"project\(\s*['"]:([\w-]+)['"]\s*\)"#).unwrap();
    let mut deps = Vec::new();

    for cap in re.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            let dep_name = name.as_str().to_string();
            if !deps.contains(&dep_name) {
                deps.push(dep_name);
            }
        }
    }

    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_settings_gradle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "include ':app', ':lib'\n",
        )
        .unwrap();
        assert!(GradleResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_settings_gradle_kts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            "include(\":app\")\n",
        )
        .unwrap();
        assert!(GradleResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_no_settings() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!GradleResolver.detect(dir.path()));
    }

    #[test]
    fn test_parse_include_groovy_single() {
        let content = "include ':app'\n";
        let modules = parse_include_directives(content);
        assert_eq!(modules, vec!["app"]);
    }

    #[test]
    fn test_parse_include_groovy_multiple() {
        let content = "include ':app', ':lib', ':core'\n";
        let modules = parse_include_directives(content);
        assert_eq!(modules, vec!["app", "lib", "core"]);
    }

    #[test]
    fn test_parse_include_kts_single() {
        let content = "include(\":app\")\n";
        let modules = parse_include_directives(content);
        assert_eq!(modules, vec!["app"]);
    }

    #[test]
    fn test_parse_include_kts_multiple() {
        let content = "include(\":app\", \":lib\")\n";
        let modules = parse_include_directives(content);
        assert_eq!(modules, vec!["app", "lib"]);
    }

    #[test]
    fn test_parse_include_multi_line() {
        let content = "include ':app'\ninclude ':lib'\n";
        let modules = parse_include_directives(content);
        assert_eq!(modules, vec!["app", "lib"]);
    }

    #[test]
    fn test_parse_include_ignores_non_include_lines() {
        let content = "rootProject.name = 'my-project'\ninclude ':app'\n// include ':commented'\n";
        let modules = parse_include_directives(content);
        assert_eq!(modules, vec!["app"]);
    }

    #[test]
    fn test_parse_include_no_duplicates() {
        let content = "include ':app'\ninclude ':app'\n";
        let modules = parse_include_directives(content);
        assert_eq!(modules, vec!["app"]);
    }

    #[test]
    fn test_parse_project_dependencies() {
        let content = r#"
dependencies {
    implementation project(':core')
    testImplementation project(':test-utils')
    api project(":shared")
}
"#;
        let deps = parse_project_dependencies(content);
        assert_eq!(deps, vec!["core", "test-utils", "shared"]);
    }

    #[test]
    fn test_parse_project_dependencies_kts() {
        let content = r#"
dependencies {
    implementation(project(":core"))
    testImplementation(project(":test-utils"))
}
"#;
        let deps = parse_project_dependencies(content);
        assert_eq!(deps, vec!["core", "test-utils"]);
    }

    #[test]
    fn test_parse_project_dependencies_none() {
        let content = r#"
dependencies {
    implementation "org.example:lib:1.0"
}
"#;
        let deps = parse_project_dependencies(content);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_resolve_gradle_project() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("settings.gradle"),
            "include ':app', ':lib'\n",
        )
        .unwrap();

        // lib module
        std::fs::create_dir_all(dir.path().join("lib")).unwrap();
        std::fs::write(
            dir.path().join("lib/build.gradle"),
            "apply plugin: 'java'\n",
        )
        .unwrap();

        // app module depends on lib
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        std::fs::write(
            dir.path().join("app/build.gradle"),
            "apply plugin: 'java'\ndependencies {\n    implementation project(':lib')\n}\n",
        )
        .unwrap();

        let graph = GradleResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 2);
        assert!(graph.packages.contains_key(&PackageId("app".into())));
        assert!(graph.packages.contains_key(&PackageId("lib".into())));

        // app depends on lib
        assert!(graph.edges.contains(&(
            PackageId("app".into()),
            PackageId("lib".into()),
        )));
    }

    #[test]
    fn test_resolve_gradle_kts_project() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            "include(\":core\", \":api\")\n",
        )
        .unwrap();

        // core module
        std::fs::create_dir_all(dir.path().join("core")).unwrap();
        std::fs::write(
            dir.path().join("core/build.gradle.kts"),
            "plugins { java }\n",
        )
        .unwrap();

        // api module depends on core
        std::fs::create_dir_all(dir.path().join("api")).unwrap();
        std::fs::write(
            dir.path().join("api/build.gradle.kts"),
            "plugins { java }\ndependencies {\n    implementation(project(\":core\"))\n}\n",
        )
        .unwrap();

        let graph = GradleResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 2);
        assert!(graph.packages.contains_key(&PackageId("core".into())));
        assert!(graph.packages.contains_key(&PackageId("api".into())));

        // api depends on core
        assert!(graph.edges.contains(&(
            PackageId("api".into()),
            PackageId("core".into()),
        )));
    }

    #[test]
    fn test_resolve_gradle_no_internal_deps() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("settings.gradle"),
            "include ':alpha', ':beta'\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("alpha")).unwrap();
        std::fs::write(
            dir.path().join("alpha/build.gradle"),
            "apply plugin: 'java'\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("beta")).unwrap();
        std::fs::write(
            dir.path().join("beta/build.gradle"),
            "apply plugin: 'java'\n",
        )
        .unwrap();

        let graph = GradleResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 2);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_resolve_gradle_skips_missing_dir() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("settings.gradle"),
            "include ':exists', ':missing'\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("exists")).unwrap();
        std::fs::write(
            dir.path().join("exists/build.gradle"),
            "apply plugin: 'java'\n",
        )
        .unwrap();
        // 'missing' directory is not created

        let graph = GradleResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 1);
        assert!(graph.packages.contains_key(&PackageId("exists".into())));
    }

    #[test]
    fn test_test_command() {
        let cmd = GradleResolver.test_command(&PackageId("app".into()));
        assert_eq!(cmd, vec!["gradle", ":app:test"]);
    }
}
