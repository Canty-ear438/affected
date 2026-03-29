use affected_core::graph::DepGraph;
use affected_core::resolvers;
use affected_core::types::PackageId;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Helper: init a git repo, add all files, make an initial commit.
fn git_init_and_commit(dir: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args([
            "-c", "user.name=Test",
            "-c", "user.email=test@test.com",
            "commit", "-m", "initial",
        ])
        .current_dir(dir)
        .output()
        .expect("git commit failed");
}

/// Helper: make a change and commit it.
fn git_change_and_commit(dir: &Path, file: &str, content: &str) {
    let path = dir.join(file);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, content).expect("failed to write file");

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args([
            "-c", "user.name=Test",
            "-c", "user.email=test@test.com",
            "commit", "-m", "change",
        ])
        .current_dir(dir)
        .output()
        .expect("git commit failed");
}

// ─── Cargo Integration ─────────────────────────────────────

fn create_cargo_workspace(dir: &Path) {
    std::fs::write(
        dir.join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["crates/core", "crates/api", "crates/cli"]
"#,
    )
    .unwrap();

    // Core crate (no deps)
    std::fs::create_dir_all(dir.join("crates/core/src")).unwrap();
    std::fs::write(
        dir.join("crates/core/Cargo.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("crates/core/src/lib.rs"), "pub fn hello() {}\n").unwrap();

    // API crate (depends on core)
    std::fs::create_dir_all(dir.join("crates/api/src")).unwrap();
    std::fs::write(
        dir.join("crates/api/Cargo.toml"),
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
    )
    .unwrap();
    std::fs::write(dir.join("crates/api/src/lib.rs"), "pub fn serve() {}\n").unwrap();

    // CLI crate (depends on api)
    std::fs::create_dir_all(dir.join("crates/cli/src")).unwrap();
    std::fs::write(
        dir.join("crates/cli/Cargo.toml"),
        "[package]\nname = \"cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\napi = { path = \"../api\" }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("crates/cli/src/main.rs"),
        "fn main() { println!(\"hello\"); }\n",
    )
    .unwrap();
}

#[test]
fn test_cargo_full_pipeline_core_change() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());
    git_init_and_commit(dir.path());

    // Change core → should affect core, api, cli
    git_change_and_commit(dir.path(), "crates/core/src/lib.rs", "pub fn hello() { /* changed */ }\n");

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    assert_eq!(result.total_packages, 3);
    assert!(result.affected.contains(&"core".to_string()));
    assert!(result.affected.contains(&"api".to_string()));
    assert!(result.affected.contains(&"cli".to_string()));
    assert_eq!(result.affected.len(), 3);
}

#[test]
fn test_cargo_full_pipeline_leaf_change() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());
    git_init_and_commit(dir.path());

    // Change cli only → should affect only cli
    git_change_and_commit(
        dir.path(),
        "crates/cli/src/main.rs",
        "fn main() { println!(\"changed\"); }\n",
    );

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    assert_eq!(result.affected, vec!["cli".to_string()]);
}

#[test]
fn test_cargo_full_pipeline_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());
    git_init_and_commit(dir.path());

    // No changes since HEAD → empty
    let result = affected_core::find_affected(dir.path(), "HEAD").unwrap();
    assert!(result.affected.is_empty());
    assert_eq!(result.changed_files, 0);
}

#[test]
fn test_cargo_full_pipeline_middle_change() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());
    git_init_and_commit(dir.path());

    // Change api → should affect api + cli (not core)
    git_change_and_commit(dir.path(), "crates/api/src/lib.rs", "pub fn serve() { /* v2 */ }\n");

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    assert!(result.affected.contains(&"api".to_string()));
    assert!(result.affected.contains(&"cli".to_string()));
    assert!(!result.affected.contains(&"core".to_string()));
}

// ─── npm Integration ────────────────────────────────────────

fn create_npm_workspace(dir: &Path) {
    std::fs::write(
        dir.join("package.json"),
        r#"{"name": "root", "private": true, "workspaces": ["packages/*"]}"#,
    )
    .unwrap();

    // Package: shared (no deps)
    std::fs::create_dir_all(dir.join("packages/shared/src")).unwrap();
    std::fs::write(
        dir.join("packages/shared/package.json"),
        r#"{"name": "shared", "version": "1.0.0"}"#,
    )
    .unwrap();
    std::fs::write(dir.join("packages/shared/src/index.js"), "module.exports = {};\n").unwrap();

    // Package: app (depends on shared)
    std::fs::create_dir_all(dir.join("packages/app/src")).unwrap();
    std::fs::write(
        dir.join("packages/app/package.json"),
        r#"{"name": "app", "version": "1.0.0", "dependencies": {"shared": "workspace:*"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("packages/app/src/index.js"),
        "const shared = require('shared');\n",
    )
    .unwrap();
}

#[test]
fn test_npm_full_pipeline_shared_change() {
    let dir = tempfile::tempdir().unwrap();
    create_npm_workspace(dir.path());
    git_init_and_commit(dir.path());

    git_change_and_commit(
        dir.path(),
        "packages/shared/src/index.js",
        "module.exports = { changed: true };\n",
    );

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    assert!(result.affected.contains(&"shared".to_string()));
    assert!(result.affected.contains(&"app".to_string()));
    assert_eq!(result.affected.len(), 2);
}

#[test]
fn test_npm_full_pipeline_leaf_change() {
    let dir = tempfile::tempdir().unwrap();
    create_npm_workspace(dir.path());
    git_init_and_commit(dir.path());

    git_change_and_commit(
        dir.path(),
        "packages/app/src/index.js",
        "// changed\nconst shared = require('shared');\n",
    );

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    assert_eq!(result.affected, vec!["app".to_string()]);
}

// ─── Dependency Graph Integration ───────────────────────────

#[test]
fn test_cargo_dependency_graph_structure() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());

    let resolver = resolvers::detect_resolver(dir.path()).unwrap();
    let graph = resolver.resolve(dir.path()).unwrap();
    let dep_graph = DepGraph::from_project_graph(&graph);

    // Verify graph structure
    let edges = dep_graph.edges();
    assert!(!edges.is_empty());

    // Verify DOT output is valid
    let dot = dep_graph.to_dot();
    assert!(dot.contains("digraph"));
    assert!(dot.contains("->"));
}

#[test]
fn test_npm_dependency_graph_structure() {
    let dir = tempfile::tempdir().unwrap();
    create_npm_workspace(dir.path());

    let resolver = resolvers::detect_resolver(dir.path()).unwrap();
    let graph = resolver.resolve(dir.path()).unwrap();
    let dep_graph = DepGraph::from_project_graph(&graph);

    let edges = dep_graph.edges();
    assert_eq!(edges.len(), 1); // app -> shared
}

// ─── Config Integration ─────────────────────────────────────

#[test]
fn test_config_ignore_files() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());

    // Add config that ignores .md files
    std::fs::write(
        dir.path().join(".affected.toml"),
        "ignore = [\"*.md\"]\n",
    )
    .unwrap();

    git_init_and_commit(dir.path());

    // Change only a markdown file
    git_change_and_commit(dir.path(), "README.md", "# Changed\n");

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    // The .md file is ignored, so nothing should be affected
    // (the file doesn't belong to any package anyway, but this tests the ignore path)
    assert!(result.affected.is_empty());
}

// ─── Edge Cases ─────────────────────────────────────────────

#[test]
fn test_file_outside_any_package() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());
    git_init_and_commit(dir.path());

    // Change a file at root level (not in any crate)
    git_change_and_commit(dir.path(), "scripts/deploy.sh", "#!/bin/bash\necho deploy\n");

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    assert!(result.affected.is_empty());
    assert!(result.changed_files > 0);
}

#[test]
fn test_invalid_base_ref() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());
    git_init_and_commit(dir.path());

    let result = affected_core::find_affected(dir.path(), "nonexistent-branch");
    assert!(result.is_err());
}

#[test]
fn test_multiple_files_same_package() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());
    git_init_and_commit(dir.path());

    // Change two files in core
    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn hello() { /* v2 */ }\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("crates/core/src/utils.rs"),
        "pub fn util() {}\n",
    )
    .unwrap();

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "-c", "user.name=Test",
            "-c", "user.email=test@test.com",
            "commit", "-m", "multi-file change",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let result = affected_core::find_affected(dir.path(), "HEAD~1").unwrap();
    // Still just the same 3 packages, not duplicated
    assert_eq!(result.affected.len(), 3);
}

// ─── Reverse BFS Correctness with Real Resolver ─────────────

#[test]
fn test_reverse_bfs_with_real_cargo_graph() {
    let dir = tempfile::tempdir().unwrap();
    create_cargo_workspace(dir.path());

    let resolver = resolvers::detect_resolver(dir.path()).unwrap();
    let graph = resolver.resolve(dir.path()).unwrap();
    let dep_graph = DepGraph::from_project_graph(&graph);

    // If core changes, all 3 are affected
    let changed: HashSet<_> = [PackageId("core".into())].into();
    let affected = dep_graph.affected_by(&changed);
    assert_eq!(affected.len(), 3);

    // If api changes, api + cli are affected
    let changed: HashSet<_> = [PackageId("api".into())].into();
    let affected = dep_graph.affected_by(&changed);
    assert_eq!(affected.len(), 2);
    assert!(affected.contains(&PackageId("api".into())));
    assert!(affected.contains(&PackageId("cli".into())));

    // If cli changes, only cli
    let changed: HashSet<_> = [PackageId("cli".into())].into();
    let affected = dep_graph.affected_by(&changed);
    assert_eq!(affected.len(), 1);
}
