use anyhow::{Context, Result};
use git2::Repository;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

    let base_obj = repo
        .revparse_single(base_ref)
        .with_context(|| format!("Could not resolve base ref '{base_ref}'"))?;

    let base_tree = base_obj
        .peel_to_tree()
        .with_context(|| format!("Could not peel '{base_ref}' to a tree"))?;

    let head_ref = repo.head().context("Could not get HEAD")?;
    let head_tree = head_ref
        .peel_to_tree()
        .context("Could not peel HEAD to a tree")?;

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

    Ok(GitDiff {
        changed_files,
        repo_root,
    })
}
