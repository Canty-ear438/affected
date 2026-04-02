pub mod config;
pub mod detect;
pub mod git;
pub mod graph;
pub mod resolvers;
pub mod runner;
pub mod types;

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::debug;

use types::{AffectedResult, ExplainEntry, ExplainReason};

/// Main orchestration: given a project root and base ref,
/// determine which packages are affected by git changes.
pub fn find_affected(root: &Path, base_ref: &str) -> Result<AffectedResult> {
    find_affected_with_options(root, base_ref, false, None, None)
}

/// Enhanced version of find_affected with support for explain, filter, and skip.
///
/// - `explain`: When true, populates the `explanations` field in the result.
/// - `filter`: Optional glob pattern to include only matching package names.
/// - `skip`: Optional glob pattern to exclude matching package names.
pub fn find_affected_with_options(
    root: &Path,
    base_ref: &str,
    explain: bool,
    filter: Option<&str>,
    skip: Option<&str>,
) -> Result<AffectedResult> {
    debug!(
        "Finding affected packages at {} with base ref '{}'",
        root.display(),
        base_ref
    );

    let config = config::Config::load(root)?;

    // 1. Detect ecosystem and get resolver
    let resolver = resolvers::detect_resolver(root)?;
    debug!("Using resolver for ecosystem: {}", resolver.ecosystem());

    // 2. Build project graph
    let project_graph = resolver
        .resolve(root)
        .context("Failed to resolve project graph")?;
    debug!("Resolved {} packages", project_graph.packages.len());

    // 3. Get changed files from git
    let git_diff = git::changed_files(root, base_ref).context("Failed to compute git diff")?;
    debug!(
        "{} files changed since {}",
        git_diff.changed_files.len(),
        base_ref
    );

    // 4. Map changed files to packages (filtering ignored files)
    let mut changed_packages = HashSet::new();
    let mut changed_files_per_package: HashMap<types::PackageId, Vec<String>> = HashMap::new();
    for file in &git_diff.changed_files {
        let file_str = file.to_str().unwrap_or("");
        if config.is_ignored(file_str) {
            debug!("Ignoring file: {}", file_str);
            continue;
        }
        if let Some(pkg_id) = resolver.package_for_file(&project_graph, file) {
            changed_packages.insert(pkg_id.clone());
            if explain {
                changed_files_per_package
                    .entry(pkg_id)
                    .or_default()
                    .push(file_str.to_string());
            }
        }
    }
    debug!("{} packages directly changed", changed_packages.len());

    // 5. Build dependency graph and compute transitive affected set
    let dep_graph = graph::DepGraph::from_project_graph(&project_graph);
    let affected = dep_graph.affected_by(&changed_packages);
    debug!(
        "{} packages affected (including transitive)",
        affected.len()
    );

    // 6. Build explanations if requested
    let explanations = if explain {
        let chains = dep_graph.explain_affected(&changed_packages, &affected);
        let mut entries: Vec<ExplainEntry> = chains
            .into_iter()
            .map(|(pkg_id, chain)| {
                let reason = if changed_packages.contains(&pkg_id) {
                    ExplainReason::DirectlyChanged {
                        files: changed_files_per_package
                            .get(&pkg_id)
                            .cloned()
                            .unwrap_or_default(),
                    }
                } else {
                    ExplainReason::TransitivelyAffected {
                        chain: chain.into_iter().map(|p| p.0).collect(),
                    }
                };
                ExplainEntry {
                    package: pkg_id.0,
                    reason,
                }
            })
            .collect();
        entries.sort_by(|a, b| a.package.cmp(&b.package));
        Some(entries)
    } else {
        None
    };

    // 7. Sort and apply filter/skip for deterministic output
    let mut affected_names: Vec<String> = affected.into_iter().map(|p| p.0).collect();
    affected_names.sort();

    // Apply filter (include only matching)
    if let Some(filter_pattern) = filter {
        let pat = glob::Pattern::new(filter_pattern)
            .with_context(|| format!("Invalid filter pattern '{filter_pattern}'"))?;
        debug!("Applying filter pattern: {}", filter_pattern);
        affected_names.retain(|name| pat.matches(name));
    }

    // Apply skip (exclude matching)
    if let Some(skip_pattern) = skip {
        let pat = glob::Pattern::new(skip_pattern)
            .with_context(|| format!("Invalid skip pattern '{skip_pattern}'"))?;
        debug!("Applying skip pattern: {}", skip_pattern);
        affected_names.retain(|name| !pat.matches(name));
    }

    Ok(AffectedResult {
        affected: affected_names,
        base: base_ref.to_string(),
        changed_files: git_diff.changed_files.len(),
        total_packages: project_graph.packages.len(),
        explanations,
    })
}

/// Compute the merge-base between HEAD and the given branch.
/// Wraps `git::merge_base` for use from the CLI.
pub fn find_merge_base(root: &Path, branch: &str) -> Result<String> {
    debug!("Finding merge-base with branch '{}'", branch);
    git::merge_base(root, branch)
}

/// Build the project graph and return it alongside the resolver.
/// Used by commands that need the graph without computing affected packages.
pub fn resolve_project(root: &Path) -> Result<(Box<dyn resolvers::Resolver>, types::ProjectGraph)> {
    let resolver = resolvers::detect_resolver(root)?;
    let graph = resolver
        .resolve(root)
        .context("Failed to resolve project graph")?;
    Ok((resolver, graph))
}
