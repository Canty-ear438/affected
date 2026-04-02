# setup-affected

Install the [affected](https://github.com/Rani367/affected) CLI in GitHub Actions. Downloads, caches, and adds the binary to PATH.

## Usage

```yaml
- uses: Rani367/setup-affected@v1
- run: affected list --base origin/main --explain
```

### Pin a version

```yaml
- uses: Rani367/setup-affected@v1
  with:
    version: 'v0.2.0'
```

## Dynamic Matrix (run each package as a separate job)

This is the killer pattern. Each affected package runs as a **separate parallel job** in the GitHub Actions UI:

```yaml
name: CI

on:
  pull_request:
    branches: [main]

jobs:
  detect:
    runs-on: ubuntu-latest
    outputs:
      matrix: ${{ steps.affected.outputs.matrix }}
      has_affected: ${{ steps.affected.outputs.has_affected }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: Rani367/setup-affected@v1

      - name: Detect affected packages
        id: affected
        run: affected ci --merge-base origin/main

  test:
    needs: detect
    if: needs.detect.outputs.has_affected == 'true'
    runs-on: ubuntu-latest
    strategy:
      matrix: ${{ fromJson(needs.detect.outputs.matrix) }}
      fail-fast: false
    steps:
      - uses: actions/checkout@v4

      - name: Test ${{ matrix.package }}
        run: cargo test -p ${{ matrix.package }}
```

## Simple usage (test all affected in one job)

```yaml
name: CI

on:
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: Rani367/setup-affected@v1

      - name: Run affected tests
        run: affected test --merge-base origin/main --jobs 4
```

## Run custom commands on affected packages

```yaml
      - name: Lint affected packages
        run: affected run "cargo clippy -p {package} -- -D warnings" --merge-base origin/main

      - name: Build affected packages
        run: affected run "cargo build -p {package}" --merge-base origin/main --jobs 4
```

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `version` | Version to install (e.g., `v0.2.0` or `latest`) | `latest` |
| `token` | GitHub token for resolving latest version | `${{ github.token }}` |

## Supported platforms

| Runner OS | Architecture |
|-----------|-------------|
| `ubuntu-latest` | x64, arm64 |
| `macos-latest` | x64, arm64 |
| `windows-latest` | x64 |
