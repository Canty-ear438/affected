pub mod config;
pub mod detect;
pub mod git;
pub mod graph;
pub mod resolvers;
pub mod runner;
pub mod types;

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

use types::AffectedResult;

/// Main orchestration: given a project root and base ref,
/// determine which packages are affected by git changes.
pub fn find_affected(root: &Path, base_ref: &str) -> Result<AffectedResult> {
    let config = config::Config::load(root)?;

    // 1. Detect ecosystem and get resolver
    let resolver = resolvers::detect_resolver(root)?;

    // 2. Build project graph
    let project_graph = resolver
        .resolve(root)
        .context("Failed to resolve project graph")?;

    // 3. Get changed files from git
    let git_diff = git::changed_files(root, base_ref)
        .context("Failed to compute git diff")?;

    // 4. Map changed files to packages (filtering ignored files)
    let mut changed_packages = HashSet::new();
    for file in &git_diff.changed_files {
        let file_str = file.to_str().unwrap_or("");
        if config.is_ignored(file_str) {
            continue;
        }
        if let Some(pkg_id) = resolver.package_for_file(&project_graph, file) {
            changed_packages.insert(pkg_id);
        }
    }

    // 5. Build dependency graph and compute transitive affected set
    let dep_graph = graph::DepGraph::from_project_graph(&project_graph);
    let affected = dep_graph.affected_by(&changed_packages);

    // 6. Sort for deterministic output
    let mut affected_names: Vec<String> = affected.into_iter().map(|p| p.0).collect();
    affected_names.sort();

    Ok(AffectedResult {
        affected: affected_names,
        base: base_ref.to_string(),
        changed_files: git_diff.changed_files.len(),
        total_packages: project_graph.packages.len(),
    })
}

/// Build the project graph and return it alongside the resolver.
/// Used by commands that need the graph without computing affected packages.
pub fn resolve_project(
    root: &Path,
) -> Result<(Box<dyn resolvers::Resolver>, types::ProjectGraph)> {
    let resolver = resolvers::detect_resolver(root)?;
    let graph = resolver
        .resolve(root)
        .context("Failed to resolve project graph")?;
    Ok((resolver, graph))
}
