use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Unique identifier for a package within a project.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageId(pub String);

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single package/module discovered by a resolver.
#[derive(Debug, Clone)]
pub struct Package {
    pub id: PackageId,
    pub name: String,
    pub version: Option<String>,
    /// Absolute path to the package root directory.
    pub path: PathBuf,
    /// Absolute path to the manifest file.
    pub manifest_path: PathBuf,
}

/// The fully-resolved project graph returned by a resolver.
#[derive(Debug)]
pub struct ProjectGraph {
    pub packages: HashMap<PackageId, Package>,
    /// Dependency edges: (dependent, dependency). "A depends on B" = (A, B).
    pub edges: Vec<(PackageId, PackageId)>,
    pub root: PathBuf,
}

/// What kind of ecosystem was detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecosystem {
    Cargo,
    Npm,
    Go,
    Python,
}

impl fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ecosystem::Cargo => write!(f, "cargo"),
            Ecosystem::Npm => write!(f, "npm"),
            Ecosystem::Go => write!(f, "go"),
            Ecosystem::Python => write!(f, "python"),
        }
    }
}

/// The result of the "affected" computation.
#[derive(Debug, Serialize)]
pub struct AffectedResult {
    pub affected: Vec<String>,
    pub base: String,
    pub changed_files: usize,
    pub total_packages: usize,
}
