use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "affected",
    version,
    about = "Run only the tests that matter. Language-agnostic affected test detection for monorepos."
)]
struct Cli {
    /// Path to the project root (default: current directory)
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tests for affected packages
    Test {
        /// Base git ref to compare against (branch, tag, or commit)
        #[arg(long)]
        base: String,

        /// Show what would be tested without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// List affected packages without running tests
    List {
        /// Base git ref to compare against
        #[arg(long)]
        base: String,

        /// Output as JSON (for CI integration)
        #[arg(long)]
        json: bool,
    },

    /// Display the project dependency graph
    Graph {
        /// Output in DOT format (for Graphviz)
        #[arg(long)]
        dot: bool,
    },

    /// Show detected project type and packages
    Detect,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = std::fs::canonicalize(&cli.root)?;

    match cli.command {
        Commands::Test { base, dry_run } => cmd_test(&root, &base, dry_run),
        Commands::List { base, json } => cmd_list(&root, &base, json),
        Commands::Graph { dot } => cmd_graph(&root, dot),
        Commands::Detect => cmd_detect(&root),
    }
}

fn cmd_test(root: &PathBuf, base: &str, dry_run: bool) -> Result<()> {
    let config = affected_core::config::Config::load(root)?;
    let result = affected_core::find_affected(root, base)?;

    if result.affected.is_empty() {
        println!("{}", "No packages affected.".dimmed());
        return Ok(());
    }

    println!(
        "{} {} affected package(s) (out of {} total, {} files changed):",
        "Testing".bold().cyan(),
        result.affected.len(),
        result.total_packages,
        result.changed_files,
    );
    println!();

    // Determine ecosystem for test commands
    let resolver = affected_core::resolvers::detect_resolver(root)?;
    let ecosystem = resolver.ecosystem();

    let commands: Vec<_> = result
        .affected
        .iter()
        .map(|name| {
            let pkg_id = affected_core::types::PackageId(name.clone());
            let cmd = config
                .test_command_for(ecosystem, name)
                .unwrap_or_else(|| resolver.test_command(&pkg_id));
            (pkg_id, cmd)
        })
        .collect();

    let runner = affected_core::runner::Runner::new(root, dry_run);
    let results = runner.run_tests(commands)?;
    affected_core::runner::print_summary(&results);

    let any_failed = results.iter().any(|r| !r.success);
    if any_failed {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_list(root: &PathBuf, base: &str, json: bool) -> Result<()> {
    let result = affected_core::find_affected(root, base)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if result.affected.is_empty() {
        println!("{}", "No packages affected.".dimmed());
        return Ok(());
    }

    println!(
        "{} affected package(s) (base: {}, {} files changed):\n",
        result.affected.len().to_string().bold(),
        base.cyan(),
        result.changed_files,
    );

    for name in &result.affected {
        println!("  {} {}", "●".green(), name);
    }

    Ok(())
}

fn cmd_graph(root: &PathBuf, dot: bool) -> Result<()> {
    let (_resolver, project_graph) = affected_core::resolve_project(root)?;
    let dep_graph = affected_core::graph::DepGraph::from_project_graph(&project_graph);

    if dot {
        println!("{}", dep_graph.to_dot());
        return Ok(());
    }

    let edges = dep_graph.edges();
    if edges.is_empty() {
        println!("{}", "No dependencies between packages.".dimmed());
        return Ok(());
    }

    println!("{}\n", "Dependency Graph:".bold());
    for (from, to) in &edges {
        println!("  {} {} {}", from.to_string().cyan(), "→".dimmed(), to);
    }

    Ok(())
}

fn cmd_detect(root: &PathBuf) -> Result<()> {
    let ecosystems = affected_core::detect::detect_ecosystems(root)?;
    let (resolver, project_graph) = affected_core::resolve_project(root)?;

    println!("{} {}\n", "Ecosystem:".bold(), resolver.ecosystem());
    println!(
        "Detected: {}",
        ecosystems
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();
    println!(
        "{} ({} found):\n",
        "Packages".bold(),
        project_graph.packages.len()
    );

    let mut names: Vec<_> = project_graph
        .packages
        .values()
        .map(|p| (&p.name, &p.path))
        .collect();
    names.sort_by_key(|(n, _)| (*n).clone());

    for (name, path) in names {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        println!("  {} {} {}", "●".green(), name.cyan(), rel.dimmed());
    }

    Ok(())
}
