use anyhow::Result;
use std::path::Path;
use tracing::debug;

use crate::types::Ecosystem;

/// Detect which ecosystem(s) a project uses by scanning for marker files.
pub fn detect_ecosystems(root: &Path) -> Result<Vec<Ecosystem>> {
    let mut detected = Vec::new();

    // Cargo: Cargo.toml with [workspace]
    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                debug!("Detected Cargo workspace at {}", cargo_toml.display());
                detected.push(Ecosystem::Cargo);
            }
        }
    }

    // Yarn: .yarnrc.yml exists → Ecosystem::Yarn (takes priority over npm)
    let yarnrc = root.join(".yarnrc.yml");
    if yarnrc.exists() {
        debug!("Detected Yarn Berry project via .yarnrc.yml");
        detected.push(Ecosystem::Yarn);
    } else {
        // npm/pnpm: package.json with "workspaces" or pnpm-workspace.yaml
        let pkg_json = root.join("package.json");
        let pnpm_ws = root.join("pnpm-workspace.yaml");
        if pnpm_ws.exists() {
            debug!("Detected pnpm workspace via pnpm-workspace.yaml");
            detected.push(Ecosystem::Npm);
        } else if pkg_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if content.contains("\"workspaces\"") {
                    debug!("Detected npm workspaces in package.json");
                    detected.push(Ecosystem::Npm);
                }
            }
        }
    }

    // Go: go.work (workspace) or go.mod (single module)
    if root.join("go.work").exists() || root.join("go.mod").exists() {
        debug!("Detected Go project");
        detected.push(Ecosystem::Go);
    }

    // Python: check for Poetry, uv, or generic pyproject.toml
    let root_pyproject = root.join("pyproject.toml");
    if root_pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&root_pyproject) {
            if content.contains("[tool.poetry]") {
                debug!("Detected Poetry project via [tool.poetry] in pyproject.toml");
                detected.push(Ecosystem::Python);
            } else if content.contains("[tool.uv.workspace]") {
                debug!("Detected uv workspace via [tool.uv.workspace] in pyproject.toml");
                detected.push(Ecosystem::Python);
            } else {
                debug!("Detected generic Python project via pyproject.toml");
                detected.push(Ecosystem::Python);
            }
        } else {
            detected.push(Ecosystem::Python);
        }
    } else {
        // Scan one level deep for pyproject.toml files
        let pattern = root.join("*/pyproject.toml");
        if let Ok(paths) = glob::glob(pattern.to_str().unwrap_or("")) {
            let count = paths.filter_map(|p| p.ok()).count();
            if count >= 2 {
                debug!("Detected Python monorepo ({} pyproject.toml files found)", count);
                detected.push(Ecosystem::Python);
            }
        }
    }

    // Maven: pom.xml exists at root and contains <modules>
    let pom_xml = root.join("pom.xml");
    if pom_xml.exists() {
        if let Ok(content) = std::fs::read_to_string(&pom_xml) {
            if content.contains("<modules>") {
                debug!("Detected Maven multi-module project via pom.xml");
                detected.push(Ecosystem::Maven);
            }
        }
    }

    // Gradle: settings.gradle or settings.gradle.kts exists
    if root.join("settings.gradle").exists() || root.join("settings.gradle.kts").exists() {
        debug!("Detected Gradle project");
        detected.push(Ecosystem::Gradle);
    }

    if detected.is_empty() {
        anyhow::bail!(
            "No supported project type found at {}.\n\
             Looked for: Cargo.toml (workspace), package.json (workspaces), \
             go.work/go.mod, pyproject.toml, pom.xml (modules), settings.gradle(.kts)",
            root.display()
        );
    }

    debug!("Detected ecosystems: {:?}", detected);
    Ok(detected)
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

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Cargo]);
    }

    #[test]
    fn test_detect_cargo_without_workspace_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"solo\"\n",
        )
        .unwrap();

        assert!(detect_ecosystems(dir.path()).is_err());
    }

    #[test]
    fn test_detect_npm_workspaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "root", "workspaces": ["packages/*"]}"#,
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Npm]);
    }

    #[test]
    fn test_detect_pnpm_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Npm]);
    }

    #[test]
    fn test_detect_yarn_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".yarnrc.yml"),
            "nodeLinker: pnp\n",
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Yarn]);
    }

    #[test]
    fn test_detect_go_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n").unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Go]);
    }

    #[test]
    fn test_detect_go_single_module() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/foo\n").unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Go]);
    }

    #[test]
    fn test_detect_python_root_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"myapp\"\n",
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Python]);
    }

    #[test]
    fn test_detect_multiple_ecosystems() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/x\n").unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert!(ecosystems.contains(&Ecosystem::Cargo));
        assert!(ecosystems.contains(&Ecosystem::Go));
        assert_eq!(ecosystems.len(), 2);
    }

    #[test]
    fn test_detect_empty_directory_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(detect_ecosystems(dir.path()).is_err());
    }

    #[test]
    fn test_detect_npm_without_workspaces_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "solo", "version": "1.0.0"}"#,
        )
        .unwrap();

        assert!(detect_ecosystems(dir.path()).is_err());
    }

    #[test]
    fn test_detect_maven_multi_module() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project><modules><module>core</module></modules></project>"#,
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Maven]);
    }

    #[test]
    fn test_detect_gradle_groovy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "include ':core', ':app'\n",
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Gradle]);
    }

    #[test]
    fn test_detect_gradle_kotlin() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            "include(\":core\", \":app\")\n",
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Gradle]);
    }

    #[test]
    fn test_detect_poetry_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[tool.poetry]\nname = \"myapp\"\n",
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Python]);
    }

    #[test]
    fn test_detect_uv_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
        )
        .unwrap();

        let ecosystems = detect_ecosystems(dir.path()).unwrap();
        assert_eq!(ecosystems, vec![Ecosystem::Python]);
    }
}
