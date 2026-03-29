use anyhow::Result;
use std::path::Path;

use crate::types::{Ecosystem, PackageId, ProjectGraph};

pub mod cargo;
pub mod go;
pub mod npm;
pub mod python;

/// Trait implemented by each ecosystem resolver.
pub trait Resolver {
    /// Which ecosystem this resolver handles.
    fn ecosystem(&self) -> Ecosystem;

    /// Can this resolver handle the project at the given root path?
    fn detect(&self, root: &Path) -> bool;

    /// Build the full project graph: packages + dependency edges.
    fn resolve(&self, root: &Path) -> Result<ProjectGraph>;

    /// Given a file path (relative to project root), return which package owns it.
    fn package_for_file(&self, graph: &ProjectGraph, file: &Path) -> Option<PackageId>;

    /// Return the shell command to run tests for a given package.
    fn test_command(&self, package_id: &PackageId) -> Vec<String>;
}

/// Return all available resolvers.
pub fn all_resolvers() -> Vec<Box<dyn Resolver>> {
    vec![
        Box::new(cargo::CargoResolver),
        Box::new(npm::NpmResolver),
        Box::new(go::GoResolver),
        Box::new(python::PythonResolver),
    ]
}

/// Auto-select the first matching resolver for a project.
pub fn detect_resolver(root: &Path) -> Result<Box<dyn Resolver>> {
    for resolver in all_resolvers() {
        if resolver.detect(root) {
            return Ok(resolver);
        }
    }
    anyhow::bail!(
        "No supported project type detected at {}",
        root.display()
    )
}

/// Map a file to its owning package using longest-prefix directory matching.
pub fn file_to_package(graph: &ProjectGraph, file: &Path) -> Option<PackageId> {
    let mut best: Option<(&PackageId, usize)> = None;

    for (id, pkg) in &graph.packages {
        // Get package path relative to project root
        let pkg_rel = pkg.path.strip_prefix(&graph.root).unwrap_or(&pkg.path);

        if file.starts_with(pkg_rel) {
            let depth = pkg_rel.components().count();
            if best.is_none() || depth > best.unwrap().1 {
                best = Some((id, depth));
            }
        }
    }

    best.map(|(id, _)| id.clone())
}
