use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::types::PackageId;

pub struct TestResult {
    pub package_id: PackageId,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration: std::time::Duration,
}

pub struct Runner {
    root: PathBuf,
    dry_run: bool,
}

impl Runner {
    pub fn new(root: &Path, dry_run: bool) -> Self {
        Self {
            root: root.to_path_buf(),
            dry_run,
        }
    }

    /// Execute test commands and collect results.
    pub fn run_tests(&self, commands: Vec<(PackageId, Vec<String>)>) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        for (pkg_id, args) in commands {
            if args.is_empty() {
                continue;
            }

            let cmd_str = args.join(" ");

            if self.dry_run {
                println!("  [dry-run] {}: {}", pkg_id, cmd_str);
                results.push(TestResult {
                    package_id: pkg_id,
                    success: true,
                    exit_code: Some(0),
                    duration: std::time::Duration::ZERO,
                });
                continue;
            }

            println!("  Testing {}...", pkg_id);

            let start = Instant::now();
            let status = Command::new(&args[0])
                .args(&args[1..])
                .current_dir(&self.root)
                .status();
            let duration = start.elapsed();

            match status {
                Ok(s) => {
                    let success = s.success();
                    results.push(TestResult {
                        package_id: pkg_id,
                        success,
                        exit_code: s.code(),
                        duration,
                    });
                }
                Err(e) => {
                    eprintln!("  Failed to execute '{}': {}", cmd_str, e);
                    results.push(TestResult {
                        package_id: pkg_id,
                        success: false,
                        exit_code: None,
                        duration,
                    });
                }
            }
        }

        Ok(results)
    }
}

/// Print a summary of test results.
pub fn print_summary(results: &[TestResult]) {
    let total = results.len();
    let passed = results.iter().filter(|r| r.success).count();
    let failed = total - passed;
    let total_duration: std::time::Duration = results.iter().map(|r| r.duration).sum();

    println!();
    println!("  Results: {} passed, {} failed, {} total ({:.1}s)",
        passed, failed, total, total_duration.as_secs_f64());

    if failed > 0 {
        println!();
        println!("  Failed:");
        for r in results.iter().filter(|r| !r.success) {
            println!("    - {}", r.package_id);
        }
    }
}
