use anyhow::Result;
use std::path::Path;

use crate::types::Ecosystem;

/// Detect which ecosystem(s) a project uses by scanning for marker files.
pub fn detect_ecosystems(root: &Path) -> Result<Vec<Ecosystem>> {
    let mut detected = Vec::new();

    // Cargo: Cargo.toml with [workspace]
    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                detected.push(Ecosystem::Cargo);
            }
        }
    }

    // npm/pnpm: package.json with "workspaces" or pnpm-workspace.yaml
    let pkg_json = root.join("package.json");
    let pnpm_ws = root.join("pnpm-workspace.yaml");
    if pnpm_ws.exists() {
        detected.push(Ecosystem::Npm);
    } else if pkg_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if content.contains("\"workspaces\"") {
                detected.push(Ecosystem::Npm);
            }
        }
    }

    // Go: go.work (workspace) or go.mod (single module)
    if root.join("go.work").exists() || root.join("go.mod").exists() {
        detected.push(Ecosystem::Go);
    }

    // Python: multiple pyproject.toml files in subdirectories
    let root_pyproject = root.join("pyproject.toml");
    if root_pyproject.exists() {
        detected.push(Ecosystem::Python);
    } else {
        // Scan one level deep for pyproject.toml files
        let pattern = root.join("*/pyproject.toml");
        if let Ok(paths) = glob::glob(pattern.to_str().unwrap_or("")) {
            let count = paths.filter_map(|p| p.ok()).count();
            if count >= 2 {
                detected.push(Ecosystem::Python);
            }
        }
    }

    if detected.is_empty() {
        anyhow::bail!(
            "No supported project type found at {}.\n\
             Looked for: Cargo.toml (workspace), package.json (workspaces), \
             go.work/go.mod, pyproject.toml",
            root.display()
        );
    }

    Ok(detected)
}
