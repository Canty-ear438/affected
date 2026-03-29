# affected

Run only the tests that matter. A standalone, language-agnostic CLI that detects which packages in your monorepo are affected by git changes and runs only their tests.

## Why

Every monorepo team hacks together bash scripts with `git diff | grep` to avoid running all tests on every PR. Tools like Nx, Turborepo, and Bazel solve this but require buying into an entire build system. `affected` is a single binary you install and run -- no config files, no framework, no lock-in.

## Features

- **Zero config** -- auto-detects your project type and dependency graph
- **7 ecosystems** -- Cargo, npm, pnpm, Yarn Berry, Go, Python (Poetry/uv), Maven, Gradle
- **Transitive detection** -- if `core` changes and `api` depends on `core`, both are tested
- **`--explain`** -- shows *why* each package is affected with the dependency chain
- **Parallel tests** -- `--jobs 4` runs tests across multiple threads
- **CI-first** -- `--json`, `--junit`, and `affected ci` for GitHub Actions integration
- **Fast** -- written in Rust, uses libgit2 for native git operations

## Install

```bash
cargo install affected-cli
```

Or download a pre-built binary from [Releases](https://github.com/Rani367/affected/releases).

## Quick Start

```bash
# What's affected?
affected list --base main

# Run only affected tests
affected test --base main

# See why each package is affected
affected list --base main --explain

# Dry run (show commands without executing)
affected test --base main --dry-run
```

## Usage

### `affected test`

Run tests for affected packages.

```bash
affected test --base main                     # run affected tests
affected test --base HEAD~3                   # compare vs 3 commits ago
affected test --merge-base main               # auto-detect merge-base (best for PRs)
affected test --base main --jobs 4            # parallel execution
affected test --base main --timeout 300       # 5 min timeout per package
affected test --base main --dry-run           # show what would run
affected test --base main --json              # structured JSON output
affected test --base main --junit results.xml # JUnit XML for CI
affected test --base main --filter "lib-*"    # only test matching packages
affected test --base main --skip "e2e-*"      # skip matching packages
affected test --base main --explain           # show why each is affected
```

### `affected list`

List affected packages without running tests.

```bash
affected list --base main                     # list affected packages
affected list --base main --json              # JSON output for CI
affected list --base main --explain           # show dependency chains
```

### `affected graph`

Display the project dependency graph.

```bash
affected graph                                # human-readable graph
affected graph --dot                          # DOT format for Graphviz
affected graph --dot | dot -Tpng -o graph.png # render as image
```

### `affected detect`

Show detected project type and all packages.

```bash
affected detect
```

### `affected ci`

Output variables for CI systems (GitHub Actions).

```bash
affected ci --base main
# Output:
#   affected=core,api,cli
#   count=3
#   has_affected=true
```

In GitHub Actions:
```yaml
- name: Detect affected
  id: affected
  run: affected ci --merge-base main

- name: Run tests
  if: steps.affected.outputs.has_affected == 'true'
  run: affected test --merge-base main --jobs 4 --junit results.xml
```

### `affected completions`

Generate shell completions.

```bash
affected completions bash >> ~/.bashrc
affected completions zsh >> ~/.zshrc
affected completions fish > ~/.config/fish/completions/affected.fish
```

## Supported Ecosystems

| Ecosystem | Detected By | Dependency Source |
|-----------|------------|-------------------|
| **Cargo** | `Cargo.toml` with `[workspace]` | `cargo metadata` JSON |
| **npm** | `package.json` with `workspaces` | `package.json` dependencies |
| **pnpm** | `pnpm-workspace.yaml` | `package.json` dependencies |
| **Yarn Berry** | `.yarnrc.yml` | `package.json` dependencies |
| **Go** | `go.work` / `go.mod` | `go mod graph` |
| **Python** | `pyproject.toml` | PEP 621 deps + import scanning |
| **Poetry** | `[tool.poetry]` in pyproject.toml | Poetry path dependencies |
| **uv** | `[tool.uv.workspace]` in pyproject.toml | Workspace member globs |
| **Maven** | `pom.xml` with `<modules>` | POM dependency declarations |
| **Gradle** | `settings.gradle(.kts)` | `project(':...')` references |

## Configuration

Create `.affected.toml` in your project root (optional):

```toml
# Ignore files that should never trigger tests
ignore = ["*.md", "docs/**", ".github/**"]

# Custom test commands per ecosystem
[test]
cargo = "cargo nextest run -p {package}"
npm = "pnpm test --filter {package}"
go = "go test -v ./{package}/..."
python = "uv run --package {package} pytest"
maven = "mvn test -pl {package}"
gradle = "gradle :{package}:test"

# Per-package overrides
[packages.slow-e2e]
test = "cargo test -p slow-e2e -- --ignored"
timeout = 600

[packages.legacy-service]
skip = true
```

## How It Works

1. **Detect** -- scans for marker files to identify the ecosystem
2. **Resolve** -- builds a dependency graph from project manifests
3. **Diff** -- computes changed files using libgit2 (base ref vs HEAD + working tree)
4. **Map** -- maps each changed file to its owning package
5. **Traverse** -- runs reverse BFS on the dependency graph to find all transitively affected packages
6. **Execute** -- runs test commands for affected packages only

## Global Flags

```
-v, --verbose    Increase verbosity (-v for debug, -vv for trace)
-q, --quiet      Suppress non-essential output
--no-color       Disable colored output (also respects NO_COLOR env var)
--root <PATH>    Path to project root (default: current directory)
--config <PATH>  Path to custom config file
```

## License

MIT
