# Releasing

Releases are automated end-to-end by GitHub Actions (see [`.github/workflows/release.yml`](../.github/workflows/release.yml)):

- **release-please** scans conventional commits on `main`, bumps the version in every crate's `Cargo.toml` (+ `Cargo.lock`), and opens a **release PR** with an auto-generated `CHANGELOG.md`.
- The workflow **auto-merges** the release PR.
- On the next `main` push, release-please detects the merged PR, creates the version tag and the GitHub Release (with the changelog), and the workflow **cross-compiles the binaries and uploads them** to that Release as assets.

## How it works

```
conventional commit -> push to main
  -> release-please opens "chore(main): release agbr vX.Y.Z" PR (CHANGELOG.md generated)
  -> workflow auto-merges the release PR
  -> next push: tag vX.Y.Z + GitHub Release created, binaries built & uploaded
```

- Versions are read from each crate's `Cargo.toml` (`release-type: rust`) — all crates in the workspace share one version and are released together. The workspace root carries a small umbrella package (`agbr-workspace`, in the root `Cargo.toml`) so release-please can parse the root manifest; crate manifests use explicit `version = "x.y.z"` (not `version.workspace = true`), which release-please cannot parse.
- No manual tagging required — release-please creates the tag and release.
- **Requires a `GH_TOKEN` repository secret** (a personal access token with `repo` scope). The default `GITHUB_TOKEN` cannot push branches/tags or re-trigger workflows, so the release PR auto-merge and tag creation would fail without it.

## Cut a release

1. Merge your changes to `main` using conventional commit messages (`feat:`, `fix:`, `docs:`, ...).
2. Wait for the release PR — the workflow merges it automatically.
3. The tag, GitHub Release, and binaries are produced automatically. Download them from the Releases page.

## Assets

| Asset | Platform |
|---|---|
| `agbr-x86_64-unknown-linux-gnu` | Linux x86_64 |
| `agbr-x86_64-pc-windows-msvc.exe` | Windows x86_64 |
| `agbr-aarch64-apple-darwin` | macOS (Apple Silicon) |

## Install from a release

```bash
# macOS (Apple Silicon)
curl -L -o agbr https://github.com/hvlcrs/agbr/releases/latest/download/agbr-aarch64-apple-darwin
chmod +x agbr
sudo mv agbr /usr/local/bin/
```

## CI

[`ci.yml`](../.github/workflows/ci.yml) runs on every push to `main` and every pull request: cross-compiles for all four release targets and runs the test suite on each. Build caches are shared between CI and release jobs via a per-target `shared-key` in `Swatinem/rust-cache`.
