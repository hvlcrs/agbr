# Releasing

Releases are fully automated by GitHub Actions. Push a `v*` tag and the [release workflow](../.github/workflows/release.yml) builds, packages, and publishes the binaries to the GitHub Release.

## Cut a release

```bash
# Bump the version in Cargo.toml first, if needed.
cargo build --release          # sanity check
cargo test --locked            # CI does this too

git add Cargo.toml Cargo.lock && git commit -m "release: v0.1.0"
git push

git tag v0.1.0
git push origin v0.1.0
```

The workflow then:

1. Builds release binaries for **Linux x86_64**, **Windows x86_64**, **macOS Intel (x86_64)**, and **macOS Apple Silicon (arm64)**.
2. Combines the two macOS binaries into a **universal** binary with `lipo`.
3. Creates the GitHub Release (auto-generated notes) and uploads all packages as assets.

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
