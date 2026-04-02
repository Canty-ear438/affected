use anyhow::{Context, Result};
use git2::Repository;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::debug;

#[non_exhaustive]
pub struct GitDiff {
    pub changed_files: Vec<PathBuf>,
    pub repo_root: PathBuf,
}

/// Compute changed files between a base ref and HEAD (plus uncommitted changes).
pub fn changed_files(repo_path: &Path, base_ref: &str) -> Result<GitDiff> {
    let repo = Repository::discover(repo_path).context("Not a git repository")?;

    let repo_root = repo
        .workdir()
        .context("Bare repositories are not supported")?
        .to_path_buf();

    debug!("Found git repository at {}", repo_root.display());

    let base_obj = repo
        .revparse_single(base_ref)
        .with_context(|| format!("Could not resolve base ref '{base_ref}'"))?;

    debug!("Resolved base ref '{}' to {}", base_ref, base_obj.id());

    let base_tree = base_obj
        .peel_to_tree()
        .with_context(|| format!("Could not peel '{base_ref}' to a tree"))?;

    let head_ref = repo.head().context("Could not get HEAD")?;
    let head_tree = head_ref
        .peel_to_tree()
        .context("Could not peel HEAD to a tree")?;

    // Refresh index from disk to pick up changes made by the git CLI
    let mut index = repo.index().context("Could not read index")?;
    index
        .read(true)
        .context("Could not refresh index from disk")?;

    let mut files = HashSet::new();

    // Committed changes: base..HEAD
    let diff_committed = repo
        .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
        .context("Failed to diff base..HEAD")?;

    for delta in diff_committed.deltas() {
        if let Some(p) = delta.new_file().path() {
            files.insert(p.to_path_buf());
        }
        if let Some(p) = delta.old_file().path() {
            files.insert(p.to_path_buf());
        }
    }

    // Uncommitted changes: HEAD vs working tree + index
    let diff_uncommitted = repo
        .diff_tree_to_workdir_with_index(Some(&head_tree), None)
        .context("Failed to diff HEAD vs working tree")?;

    for delta in diff_uncommitted.deltas() {
        if let Some(p) = delta.new_file().path() {
            files.insert(p.to_path_buf());
        }
        if let Some(p) = delta.old_file().path() {
            files.insert(p.to_path_buf());
        }
    }

    let mut changed_files: Vec<PathBuf> = files.into_iter().collect();
    changed_files.sort();

    debug!("Detected {} changed files", changed_files.len());

    Ok(GitDiff {
        changed_files,
        repo_root,
    })
}

/// Compute the merge-base between HEAD and the given branch.
/// Returns the commit SHA as a string. Used when `--merge-base` is passed.
pub fn merge_base(repo_path: &Path, branch: &str) -> Result<String> {
    let repo = Repository::discover(repo_path).context("Not a git repository")?;

    debug!("Computing merge-base between HEAD and '{}'", branch);

    let head_oid = repo
        .head()
        .context("Could not get HEAD")?
        .target()
        .context("HEAD is not a direct reference")?;

    let branch_obj = repo
        .revparse_single(branch)
        .with_context(|| format!("Could not resolve branch '{branch}'"))?;
    let branch_oid = branch_obj.id();

    let merge_base_oid = repo
        .merge_base(head_oid, branch_oid)
        .with_context(|| format!("Could not find merge-base between HEAD and '{branch}'"))?;

    let sha = merge_base_oid.to_string();
    debug!("Merge-base resolved to {}", sha);

    Ok(sha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git command failed");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn setup_repo(dir: &Path) {
        git(dir, &["init"]);
        git(dir, &["config", "user.email", "test@test.com"]);
        git(dir, &["config", "user.name", "test"]);
        std::fs::write(dir.join("file.txt"), "initial").unwrap();
        git(dir, &["add", "."]);
        git(dir, &["commit", "-m", "initial"]);
    }

    #[test]
    fn test_changed_files_committed() {
        let dir = tempfile::tempdir().unwrap();
        setup_repo(dir.path());
        let base = git(dir.path(), &["rev-parse", "HEAD"]);

        std::fs::write(dir.path().join("new.txt"), "new content").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-m", "add new file"]);

        let diff = changed_files(dir.path(), &base).unwrap();
        assert!(diff.changed_files.iter().any(|f| f.ends_with("new.txt")));
    }

    #[test]
    fn test_changed_files_uncommitted() {
        let dir = tempfile::tempdir().unwrap();
        setup_repo(dir.path());
        let base = git(dir.path(), &["rev-parse", "HEAD"]);

        // Modify an existing tracked file (untracked files don't show in tree diff)
        std::fs::write(dir.path().join("file.txt"), "modified content").unwrap();

        let diff = changed_files(dir.path(), &base).unwrap();
        assert!(diff.changed_files.iter().any(|f| f.ends_with("file.txt")));
    }

    #[test]
    fn test_changed_files_invalid_ref() {
        let dir = tempfile::tempdir().unwrap();
        setup_repo(dir.path());

        let result = changed_files(dir.path(), "nonexistent_ref_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_base_computation() {
        let dir = tempfile::tempdir().unwrap();
        setup_repo(dir.path());

        // Get the default branch name (may be "main", "master", etc.)
        let default_branch = git(dir.path(), &["branch", "--show-current"]);
        let main_sha = git(dir.path(), &["rev-parse", "HEAD"]);
        git(dir.path(), &["checkout", "-b", "feature"]);
        std::fs::write(dir.path().join("feature.txt"), "feature").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-m", "feature commit"]);

        let result = merge_base(dir.path(), &default_branch).unwrap();
        assert_eq!(result, main_sha);
    }

    #[test]
    fn test_not_a_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let result = changed_files(dir.path(), "HEAD");
        assert!(result.is_err());
    }
}
