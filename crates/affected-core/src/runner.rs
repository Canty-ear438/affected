use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::types::{PackageId, TestOutputJson, TestResultJson, TestSummaryJson};

#[non_exhaustive]
pub struct TestResult {
    pub package_id: PackageId,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub output: Option<String>,
}

/// Configuration for creating a Runner.
#[non_exhaustive]
pub struct RunnerConfig {
    pub root: PathBuf,
    pub dry_run: bool,
    pub timeout: Option<Duration>,
    pub jobs: usize,
    pub json: bool,
    pub quiet: bool,
}

impl RunnerConfig {
    /// Create a new RunnerConfig with the given settings.
    pub fn new(
        root: PathBuf,
        dry_run: bool,
        timeout: Option<Duration>,
        jobs: usize,
        json: bool,
        quiet: bool,
    ) -> Self {
        Self {
            root,
            dry_run,
            timeout,
            jobs,
            json,
            quiet,
        }
    }
}

#[non_exhaustive]
pub struct Runner {
    root: PathBuf,
    dry_run: bool,
    timeout: Option<Duration>,
    jobs: usize,
    json: bool,
    quiet: bool,
}

impl Runner {
    pub fn new(config: RunnerConfig) -> Self {
        Self {
            root: config.root,
            dry_run: config.dry_run,
            timeout: config.timeout,
            jobs: if config.jobs == 0 { 1 } else { config.jobs },
            json: config.json,
            quiet: config.quiet,
        }
    }

    /// Whether JSON output mode is enabled.
    pub fn json(&self) -> bool {
        self.json
    }

    /// Whether quiet mode is enabled.
    pub fn quiet(&self) -> bool {
        self.quiet
    }

    /// Convenience constructor for simple cases (backwards compatible).
    pub fn new_simple(root: &Path, dry_run: bool) -> Self {
        Self {
            root: root.to_path_buf(),
            dry_run,
            timeout: None,
            jobs: 1,
            json: false,
            quiet: false,
        }
    }

    /// Execute test commands and collect results.
    pub fn run_tests(&self, commands: Vec<(PackageId, Vec<String>)>) -> Result<Vec<TestResult>> {
        if self.jobs > 1 {
            self.run_tests_parallel(commands)
        } else {
            self.run_tests_sequential(commands)
        }
    }

    fn run_tests_sequential(
        &self,
        commands: Vec<(PackageId, Vec<String>)>,
    ) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        for (pkg_id, args) in commands {
            if args.is_empty() {
                continue;
            }

            let cmd_str = args.join(" ");

            if self.dry_run {
                if !self.quiet {
                    println!("  [dry-run] {}: {}", pkg_id, cmd_str);
                }
                results.push(TestResult {
                    package_id: pkg_id,
                    success: true,
                    exit_code: Some(0),
                    duration: Duration::ZERO,
                    output: None,
                });
                continue;
            }

            if !self.quiet {
                println!("  Testing {}...", pkg_id);
            }

            let result = self.run_single_test(&pkg_id, &args);
            results.push(result);
        }

        Ok(results)
    }

    fn run_tests_parallel(
        &self,
        commands: Vec<(PackageId, Vec<String>)>,
    ) -> Result<Vec<TestResult>> {
        let results = Mutex::new(Vec::new());
        let commands: Vec<_> = commands
            .into_iter()
            .filter(|(_, args)| !args.is_empty())
            .collect();

        if self.dry_run {
            let mut out = Vec::new();
            for (pkg_id, args) in &commands {
                if !self.quiet {
                    println!("  [dry-run] {}: {}", pkg_id, args.join(" "));
                }
                out.push(TestResult {
                    package_id: pkg_id.clone(),
                    success: true,
                    exit_code: Some(0),
                    duration: Duration::ZERO,
                    output: None,
                });
            }
            return Ok(out);
        }

        let jobs = self.jobs;
        std::thread::scope(|s| {
            // Create a simple work-stealing approach: chunk the commands
            let chunks: Vec<Vec<(PackageId, Vec<String>)>> = {
                let mut chunks: Vec<Vec<(PackageId, Vec<String>)>> =
                    (0..jobs).map(|_| Vec::new()).collect();
                for (i, cmd) in commands.into_iter().enumerate() {
                    chunks[i % jobs].push(cmd);
                }
                chunks
            };

            for chunk in chunks {
                let results_ref = &results;
                let root = &self.root;
                let timeout = self.timeout;
                let quiet = self.quiet;
                s.spawn(move || {
                    for (pkg_id, args) in chunk {
                        if !quiet {
                            println!("  Testing {}...", pkg_id);
                        }
                        let result = run_single_test_impl(root, timeout, &pkg_id, &args);
                        results_ref
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push(result);
                    }
                });
            }
        });

        let mut out = results.into_inner().unwrap_or_else(|e| e.into_inner());
        out.sort_by(|a, b| a.package_id.0.cmp(&b.package_id.0));
        Ok(out)
    }

    fn run_single_test(&self, pkg_id: &PackageId, args: &[String]) -> TestResult {
        run_single_test_impl(&self.root, self.timeout, pkg_id, args)
    }
}

fn run_single_test_impl(
    root: &Path,
    timeout: Option<Duration>,
    pkg_id: &PackageId,
    args: &[String],
) -> TestResult {
    let start = Instant::now();

    // When running in parallel or capturing output, pipe stdout/stderr
    let child_result = Command::new(&args[0])
        .args(&args[1..])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    match child_result {
        Ok(child) => {
            if let Some(timeout_dur) = timeout {
                // Spawn a watchdog thread to kill the child if it exceeds the timeout
                let child_id = child.id();
                let (tx, rx) = std::sync::mpsc::channel();
                let watchdog = std::thread::spawn(move || {
                    match rx.recv_timeout(timeout_dur) {
                        Ok(()) => {
                            // Process finished before timeout, nothing to do
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // Timeout expired, kill the process
                            #[cfg(unix)]
                            {
                                unsafe {
                                    libc::kill(child_id as i32, libc::SIGKILL);
                                }
                            }
                            #[cfg(not(unix))]
                            {
                                let _ = child_id; // suppress unused on non-unix
                            }
                        }
                        Err(_) => {}
                    }
                });

                let output = child.wait_with_output();
                let _ = tx.send(()); // Signal watchdog that process is done
                let _ = watchdog.join();
                let duration = start.elapsed();

                match output {
                    Ok(out) => {
                        let captured = format!(
                            "{}{}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        );
                        let timed_out = duration >= timeout_dur;
                        TestResult {
                            package_id: pkg_id.clone(),
                            success: !timed_out && out.status.success(),
                            exit_code: out.status.code(),
                            duration,
                            output: Some(captured),
                        }
                    }
                    Err(e) => {
                        let duration = start.elapsed();
                        TestResult {
                            package_id: pkg_id.clone(),
                            success: false,
                            exit_code: None,
                            duration,
                            output: Some(format!("Failed to wait for process: {e}")),
                        }
                    }
                }
            } else {
                // No timeout, just wait
                let output = child.wait_with_output();
                let duration = start.elapsed();

                match output {
                    Ok(out) => {
                        let captured = format!(
                            "{}{}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        );
                        TestResult {
                            package_id: pkg_id.clone(),
                            success: out.status.success(),
                            exit_code: out.status.code(),
                            duration,
                            output: Some(captured),
                        }
                    }
                    Err(e) => TestResult {
                        package_id: pkg_id.clone(),
                        success: false,
                        exit_code: None,
                        duration,
                        output: Some(format!("Failed to wait for process: {e}")),
                    },
                }
            }
        }
        Err(e) => {
            let cmd_str = args.join(" ");
            let duration = start.elapsed();
            eprintln!("  Failed to execute '{}': {}", cmd_str, e);
            TestResult {
                package_id: pkg_id.clone(),
                success: false,
                exit_code: None,
                duration,
                output: Some(format!("Failed to execute: {e}")),
            }
        }
    }
}

/// Return an empty TestOutputJson (no packages affected).
pub fn empty_test_output() -> TestOutputJson {
    TestOutputJson {
        affected: vec![],
        results: vec![],
        summary: TestSummaryJson {
            passed: 0,
            failed: 0,
            total: 0,
            duration_ms: 0,
        },
    }
}

/// Convert test results to JSON output format.
pub fn results_to_json(affected: &[String], results: &[TestResult]) -> TestOutputJson {
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    let passed = results.iter().filter(|r| r.success).count();
    let failed = results.len() - passed;

    TestOutputJson {
        affected: affected.to_vec(),
        results: results
            .iter()
            .map(|r| TestResultJson {
                package: r.package_id.0.clone(),
                success: r.success,
                duration_ms: r.duration.as_millis() as u64,
                exit_code: r.exit_code,
            })
            .collect(),
        summary: TestSummaryJson {
            passed,
            failed,
            total: results.len(),
            duration_ms: total_duration.as_millis() as u64,
        },
    }
}

/// Convert test results to JUnit XML format.
pub fn results_to_junit(results: &[TestResult]) -> String {
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    let passed = results.iter().filter(|r| r.success).count();
    let failed = results.len() - passed;

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuite name=\"affected\" tests=\"{}\" failures=\"{}\" time=\"{:.3}\">\n",
        results.len(),
        failed,
        total_duration.as_secs_f64(),
    ));

    for r in results {
        let time = r.duration.as_secs_f64();
        xml.push_str(&format!(
            "  <testcase name=\"{}\" classname=\"affected\" time=\"{:.3}\"",
            escape_xml(&r.package_id.0),
            time,
        ));

        if r.success {
            xml.push_str(" />\n");
        } else {
            xml.push_str(">\n");
            let msg = match r.exit_code {
                Some(code) => format!("Exit code: {}", code),
                None => "Process failed to execute".to_string(),
            };
            xml.push_str(&format!(
                "    <failure message=\"{}\">{}</failure>\n",
                escape_xml(&msg),
                escape_xml(r.output.as_deref().unwrap_or("")),
            ));
            xml.push_str("  </testcase>\n");
        }
    }

    xml.push_str("</testsuite>\n");

    let _ = passed; // used in testsuite attributes via failed count
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_runner(root: &Path, dry_run: bool, jobs: usize, timeout: Option<Duration>) -> Runner {
        Runner::new(RunnerConfig {
            root: root.to_path_buf(),
            dry_run,
            timeout,
            jobs,
            json: false,
            quiet: true,
        })
    }

    #[test]
    fn test_sequential_execution() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), false, 1, None);
        let commands = vec![(
            PackageId("pkg-a".into()),
            vec!["echo".into(), "hello".into()],
        )];
        let results = runner.run_tests(commands).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }

    #[test]
    fn test_parallel_execution() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), false, 2, None);
        let commands = vec![
            (PackageId("pkg-a".into()), vec!["echo".into(), "a".into()]),
            (PackageId("pkg-b".into()), vec!["echo".into(), "b".into()]),
            (PackageId("pkg-c".into()), vec!["echo".into(), "c".into()]),
        ];
        let results = runner.run_tests(commands).unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), true, 1, None);
        let commands = vec![(PackageId("pkg-a".into()), vec!["false".into()])];
        let results = runner.run_tests(commands).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success); // dry-run always succeeds
        assert_eq!(results[0].duration, Duration::ZERO);
    }

    #[test]
    fn test_dry_run_parallel() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), true, 2, None);
        let commands = vec![
            (PackageId("pkg-a".into()), vec!["false".into()]),
            (PackageId("pkg-b".into()), vec!["false".into()]),
        ];
        let results = runner.run_tests(commands).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    #[cfg(unix)] // Timeout enforcement uses SIGKILL, which is Unix-only
    fn test_timeout_enforcement() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), false, 1, Some(Duration::from_secs(1)));
        let commands = vec![(PackageId("slow".into()), vec!["sleep".into(), "60".into()])];
        let results = runner.run_tests(commands).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].duration < Duration::from_secs(10));
    }

    #[test]
    fn test_empty_commands() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), false, 1, None);
        let results = runner.run_tests(vec![]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_args_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), false, 1, None);
        let commands = vec![(PackageId("empty".into()), vec![])];
        let results = runner.run_tests(commands).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_all_fail() {
        let dir = tempfile::tempdir().unwrap();
        let runner = make_runner(dir.path(), false, 1, None);
        let commands = vec![
            (PackageId("pkg-a".into()), vec!["false".into()]),
            (PackageId("pkg-b".into()), vec!["false".into()]),
        ];
        let results = runner.run_tests(commands).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| !r.success));
    }

    #[test]
    fn test_results_to_json_output() {
        let results = vec![
            TestResult {
                package_id: PackageId("pkg-a".into()),
                success: true,
                exit_code: Some(0),
                duration: Duration::from_millis(100),
                output: None,
            },
            TestResult {
                package_id: PackageId("pkg-b".into()),
                success: false,
                exit_code: Some(1),
                duration: Duration::from_millis(200),
                output: Some("error".into()),
            },
        ];
        let json = results_to_json(&["pkg-a".into(), "pkg-b".into()], &results);
        assert_eq!(json.summary.passed, 1);
        assert_eq!(json.summary.failed, 1);
        assert_eq!(json.summary.total, 2);
        assert_eq!(json.results.len(), 2);
        assert!(json.results[0].success);
        assert!(!json.results[1].success);
    }

    #[test]
    fn test_results_to_junit_output() {
        let results = vec![
            TestResult {
                package_id: PackageId("pkg-ok".into()),
                success: true,
                exit_code: Some(0),
                duration: Duration::from_millis(50),
                output: None,
            },
            TestResult {
                package_id: PackageId("pkg-fail".into()),
                success: false,
                exit_code: Some(1),
                duration: Duration::from_millis(100),
                output: Some("test failed".into()),
            },
        ];
        let xml = results_to_junit(&results);
        assert!(xml.contains("<?xml version=\"1.0\""));
        assert!(xml.contains("tests=\"2\""));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("name=\"pkg-ok\""));
        assert!(xml.contains("name=\"pkg-fail\""));
        assert!(xml.contains("<failure"));
        assert!(xml.contains("test failed"));
    }
}

/// Print a summary of test results.
pub fn print_summary(results: &[TestResult]) {
    print_summary_impl(results, false);
}

/// Print a summary, respecting quiet mode.
pub fn print_summary_impl(results: &[TestResult], quiet: bool) {
    if quiet {
        return;
    }

    let total = results.len();
    let passed = results.iter().filter(|r| r.success).count();
    let failed = total - passed;
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();

    println!();
    println!(
        "  Results: {} passed, {} failed, {} total ({:.1}s)",
        passed,
        failed,
        total,
        total_duration.as_secs_f64()
    );

    if failed > 0 {
        println!();
        println!("  Failed:");
        for r in results.iter().filter(|r| !r.success) {
            println!("    - {}", r.package_id);
        }
    }
}
