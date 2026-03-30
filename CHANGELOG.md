# Changelog

## [0.3.0] - 2026-03-30

### Added

- **6 new ecosystem resolvers** (7 → 13 total):
  - **Bun** — detected via `bun.lock` / `bun.lockb` / `bunfig.toml`
  - **.NET/C#** — detected via `*.sln`, parses `<ProjectReference>` in `.csproj`
  - **Dart/Flutter** — supports `pubspec.yaml` workspaces, Melos, and generic layouts
  - **Swift/SPM** — parses `Package.swift` multi-target and multi-package projects
  - **Elixir/Mix** — detects umbrella projects via `mix.exs` + `apps/`
  - **Scala/sbt** — parses `build.sbt` `lazy val` projects and `.dependsOn()` deps
- **`affected init`** — interactive setup wizard to generate `.affected.toml` (with `--non-interactive` mode)
- **`affected watch`** — file watcher that re-runs test/list/run on changes with debouncing
- **`affected ci --format`** — multi-CI platform support: `github`, `gitlab`, `circleci`, `azure`, `generic`
- **`affected graph` tree view** — Unicode dependency tree (replaces edge list), with `--base` for affected highlighting

### Changed

- `affected graph` default output is now a Unicode tree instead of edge pairs (use `--dot` for DOT format)
- `affected ci` now accepts `--format` flag (defaults to `github` for backward compatibility)

## [0.2.1] - 2025-03-15

### Added

- PyPI packaging for `uv tool install affected` / `pipx` / `pip`
- Track `Cargo.lock` for reproducible builds

### Fixed

- Windows: use `dunce::canonicalize` to avoid `\\?\` UNC path issues
- Windows: normalize UNC paths and refresh git index for libgit2 compatibility

## [0.2.0] - 2025-03-01

### Added

- Initial release with 7 ecosystem resolvers: Cargo, npm/pnpm, Yarn Berry, Go, Python (Poetry/uv/generic), Maven, Gradle
- Subcommands: `test`, `run`, `list`, `graph`, `detect`, `ci`, `completions`
- Transitive dependency detection via reverse BFS
- `--explain` flag for dependency chain visualization
- `--json`, `--junit` output formats
- Parallel execution with `--jobs`
- GitHub Actions: `setup-affected` action, PR comment bot action
- `.affected.toml` configuration with per-ecosystem and per-package overrides
- Shell completions for bash, zsh, fish
