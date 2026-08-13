# Releasing

Releases are automated end-to-end by GitHub Actions:

- [release-please](https://github.com/googleapis/release-please-action) scans conventional commits on `main`, bumps the version, and opens a **release PR** with an auto-generated `CHANGELOG.md`.
- Merging the release PR creates the version tag and the GitHub Release (with the changelog).
- The [release workflow](../.github/workflows/release.yml) then builds, packages, and uploads the binaries to that Release.

## How it works

```
conventional commit -> push to main
  -> release-please opens "chore(main): release agbr vX.Y.Z" PR (CHANGELOG.md generated)
  -> merge the release PR
  -> tag vX.Y.Z + GitHub Release created (changelog attached)
  -> release workflow builds binaries and uploads them as assets
```

- Version is read from `Cargo.toml` (`release-type: rust`) and bumped by release-please semantics (`feat:` -> minor, `fix:` -> patch, breaking -> major).
- No manual tagging required — release-please creates the tag and release.
- The binary build is triggered by the tag push and **uploads to the release that release-please already created** (with a bare fallback if the release doesn't exist yet).

## Cut a release

1. Merge your changes to `main` using conventional commit messages (`feat:`, `fix:`, `docs:`, ...).
2. Wait for the release-please action to open the release PR (a few minutes).
3. Review the generated `CHANGELOG.md` and merge the release PR.
4. The tag, GitHub Release, and binaries are produced automatically. Download them from the Releases page.

## Assets

| Asset | Platform |
|---|---|
| `agbr-<version>-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 |
| `agbr-<version>-x86_64-pc-windows-msvc.tar.gz` | Windows x86_64 |
| `agbr-<version>-x86_64-apple-darwin.tar.gz` | macOS Intel |
| `agbr-<version>-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon |
| `agbr-<version>-universal-apple-darwin.tar.gz` | macOS, either architecture |

## Install from a release

```bash
# macOS (universal binary)
curl -L -o agbr.tar.gz https://github.com/hvlcrs/agbr/releases/download/v0.1.0/agbr-v0.1.0-universal-apple-darwin.tar.gz
tar -xzf agbr.tar.gz
sudo mv agbr /usr/local/bin/
```

## Verification

CI ([workflow](../.github/workflows/ci.yml)) runs on every push and pull request: `cargo fmt --check`, `clippy -D warnings`, `cargo test`, and a release build, on all three platforms.
