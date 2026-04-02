use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

/// Run a git command in the given directory and assert it succeeds.
fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_commit(dir: &Path, msg: &str) {
    git(
        dir,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@test.com",
            "commit",
            "-m",
            msg,
        ],
    );
    // On Windows, ensure git index is flushed before libgit2 reads it
    git(dir, &["status"]);
}

/// Helper: create a Cargo workspace in a temp dir with git.
fn setup_cargo_workspace(dir: &Path) {
    std::fs::write(
        dir.join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["crates/core", "crates/app"]
"#,
    )
    .unwrap();

    std::fs::create_dir_all(dir.join("crates/core/src")).unwrap();
    std::fs::write(
        dir.join("crates/core/Cargo.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("crates/core/src/lib.rs"), "pub fn hello() {}\n").unwrap();

    std::fs::create_dir_all(dir.join("crates/app/src")).unwrap();
    std::fs::write(
        dir.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("crates/app/src/main.rs"),
        "fn main() { println!(\"hi\"); }\n",
    )
    .unwrap();

    // Git init + commit (disable autocrlf to avoid Windows line-ending issues)
    git(dir, &["init"]);
    git(dir, &["config", "core.autocrlf", "false"]);
    git(dir, &["add", "-A"]);
    git_commit(dir, "init");
}

fn affected_cmd() -> Command {
    let mut cmd = Command::cargo_bin("affected").unwrap();
    // Prevent tests from writing to the real GITHUB_OUTPUT file in CI
    cmd.env_remove("GITHUB_OUTPUT");
    cmd
}

// ─── detect ─────────────────────────────────────────────────

#[test]
fn test_cli_detect() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .arg("detect")
        .arg("--root")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo"))
        .stdout(predicate::str::contains("core"))
        .stdout(predicate::str::contains("app"));
}

#[test]
fn test_cli_detect_no_project() {
    let dir = tempfile::tempdir().unwrap();

    affected_cmd()
        .arg("detect")
        .arg("--root")
        .arg(dir.path())
        .assert()
        .failure();
}

// ─── graph ──────────────────────────────────────────────────

#[test]
fn test_cli_graph() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .arg("graph")
        .arg("--root")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependency Graph"));
}

#[test]
fn test_cli_graph_dot() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .arg("graph")
        .arg("--dot")
        .arg("--root")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("digraph"))
        .stdout(predicate::str::contains("->"));
}

// ─── list ───────────────────────────────────────────────────

#[test]
fn test_cli_list_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .args(["list", "--base", "HEAD", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages affected"));
}

#[test]
fn test_cli_list_with_changes() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn hello() { /* v2 */ }\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    affected_cmd()
        .args(["list", "--base", "HEAD~1", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("core"))
        .stdout(predicate::str::contains("app"));
}

#[test]
fn test_cli_list_json() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn hello() { /* changed */ }\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change");

    let output = affected_cmd()
        .args(["list", "--base", "HEAD~1", "--json", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["affected"].is_array());
    assert!(json["total_packages"].as_u64().unwrap() >= 2);
    assert!(json["changed_files"].as_u64().unwrap() >= 1);
}

// ─── test --dry-run ─────────────────────────────────────────

#[test]
fn test_cli_test_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/app/src/main.rs"),
        "fn main() { println!(\"v2\"); }\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change app");

    affected_cmd()
        .args(["test", "--base", "HEAD~1", "--dry-run", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("cargo test"));
}

#[test]
fn test_cli_test_dry_run_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .args(["test", "--base", "HEAD", "--dry-run", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages affected"));
}

// ─── ci ────────────────────────────────────────────────────

#[test]
fn test_cli_ci_matrix_output() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn hello() { /* ci-test */ }\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    let output = affected_cmd()
        .args(["ci", "--base", "HEAD~1", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify matrix line is present and valid JSON
    let matrix_line = stdout
        .lines()
        .find(|l| l.starts_with("matrix="))
        .expect("matrix= line missing");
    let matrix_json: serde_json::Value =
        serde_json::from_str(matrix_line.strip_prefix("matrix=").unwrap()).unwrap();
    let packages = matrix_json["package"].as_array().unwrap();
    assert!(!packages.is_empty());

    // Verify has_affected and count are present
    assert!(stdout.contains("has_affected=true"));
    assert!(stdout.contains("count="));
}

#[test]
fn test_cli_ci_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    let output = affected_cmd()
        .args(["ci", "--base", "HEAD", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("has_affected=false"));
    assert!(stdout.contains("count=0"));
    assert!(stdout.contains(r#"matrix={"package":[]}"#));
}

// ─── run ───────────────────────────────────────────────────

#[test]
fn test_cli_run_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn hello() { /* run-test */ }\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    affected_cmd()
        .args([
            "run",
            "echo testing {package}",
            "--base",
            "HEAD~1",
            "--dry-run",
            "--root",
        ])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"))
        .stdout(predicate::str::contains("echo testing core"));
}

#[test]
fn test_cli_run_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .args([
            "run",
            "echo {package}",
            "--base",
            "HEAD",
            "--dry-run",
            "--root",
        ])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages affected"));
}

// ─── Error cases ────────────────────────────────────────────

#[test]
fn test_cli_invalid_base_ref() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .args(["list", "--base", "nonexistent-ref", "--root"])
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn test_cli_no_subcommand() {
    affected_cmd().assert().failure();
}

#[test]
fn test_cli_version() {
    affected_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("affected"));
}

#[test]
fn test_cli_help() {
    affected_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detect affected packages"));
}

// ─── Phase 5: New comprehensive tests ──────────────────────

#[test]
fn test_cli_list_filter() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    // Make a change that affects both packages
    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    affected_cmd()
        .args(["list", "--base", "HEAD~1", "--filter", "core", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("core"));
}

#[test]
fn test_cli_list_skip() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    let output = affected_cmd()
        .args(["list", "--base", "HEAD~1", "--skip", "app", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("core"));
    assert!(!stdout.contains("app"));
}

#[test]
fn test_cli_invalid_filter_pattern() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    affected_cmd()
        .args(["list", "--base", "HEAD~1", "--filter", "[invalid", "--root"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid filter pattern"));
}

#[test]
fn test_cli_invalid_skip_pattern() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    affected_cmd()
        .args(["list", "--base", "HEAD~1", "--skip", "[invalid", "--root"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid skip pattern"));
}

#[test]
fn test_cli_init_non_interactive() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    affected_cmd()
        .args(["init", "--non-interactive", "--root"])
        .arg(dir.path())
        .assert()
        .success();

    // Verify config file was created
    assert!(dir.path().join(".affected.toml").exists());

    // Verify it contains expected TOML content
    let content = std::fs::read_to_string(dir.path().join(".affected.toml")).unwrap();
    assert!(
        content.contains("[test]") || content.contains("ignore"),
        "generated config should contain TOML sections"
    );
}

#[test]
fn test_cli_completions_bash() {
    affected_cmd()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_cli_completions_zsh() {
    affected_cmd()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("compdef"));
}

#[test]
fn test_cli_graph_with_base() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    affected_cmd()
        .args(["graph", "--base", "HEAD~1", "--root"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependency Graph"));
}

#[test]
fn test_cli_list_json_structure() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    let output = affected_cmd()
        .args(["list", "--base", "HEAD~1", "--json", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(json["affected"].is_array());
    assert!(json["base"].is_string());
    assert!(json["changed_files"].is_number());
    assert!(json["total_packages"].is_number());
}

#[test]
fn test_cli_ci_gitlab_format() {
    let dir = tempfile::tempdir().unwrap();
    setup_cargo_workspace(dir.path());

    std::fs::write(
        dir.path().join("crates/core/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git_commit(dir.path(), "change core");

    affected_cmd()
        .args(["ci", "--format", "gitlab", "--base", "HEAD~1", "--root"])
        .arg(dir.path())
        .assert()
        .success();
}
