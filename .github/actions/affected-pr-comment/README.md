# affected-pr-comment

Automatically comment on pull requests showing which packages are affected by the changes and why.

## Usage

```yaml
name: PR Comment

on:
  pull_request:
    branches: [main]

permissions:
  pull-requests: write

jobs:
  comment:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: Rani367/affected-pr-comment@v1
        with:
          base: origin/main
```

## What it looks like

The action posts a comment like this on your PR:

> ## Affected Packages (3 of 8)
>
> | Package | Reason |
> |---------|--------|
> | **core** | directly changed: `src/lib.rs` |
> | **api** | depends on: core |
> | **cli** | depends on: api -> core |
>
> <details><summary>2 file(s) changed</summary>
>
> - `crates/core/src/lib.rs`
> - `crates/core/src/types.rs`
>
> </details>

The comment is automatically updated on each push to the PR (no duplicate comments).

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `base` | Base branch to compare against | `origin/main` |
| `version` | affected CLI version to install | `latest` |
| `token` | GitHub token for posting comments | `${{ github.token }}` |
| `header` | Unique identifier for comment dedup | `affected-packages` |

## Permissions

This action requires `pull-requests: write` permission to post comments.

For **fork PRs**, use the `pull_request_target` event instead of `pull_request`:

```yaml
on:
  pull_request_target:
    branches: [main]
```

Note: `pull_request_target` runs in the context of the base branch, which has write access but uses the base branch code. Make sure to check out the PR head if needed.
