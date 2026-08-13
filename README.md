# agbr

[![CI](https://github.com/hvlcrs/agbr/actions/workflows/ci.yml/badge.svg)](https://github.com/hvlcrs/agbr/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/hvlcrs/agbr)](https://github.com/hvlcrs/agbr/releases)

Terminal-native AI RAW processing engine. Describe the look in plain language; `agbr` turns it into a validated recipe and renders it non-destructively with RawTherapee.

> **The LLM controls editing intent. RawTherapee controls image processing.**

## Why "agbr"?

`AgBr` is the chemical formula for **silver bromide** — the primary light-sensitive halide compound that made film photography, and later RAW capture, possible. Every RAW sensor traces its lineage to silver-halide crystals; AgBr is the molecule that started it.

It's also a playful rotation of **ARGB** (alpha–red–green–blue), the color format — a nod to what this tool works on: pixels, but the way they were *captured*, not the way they're *displayed*.

## Application logic

```
intent (plain language)
  -> LLM gateway        # produces a typed PhotoRecipe (JSON)
  -> validate           # schema + normalized-range checks
  -> plan               # capability-gated; unsupported ops are reported, never dropped
  -> PP3 generation     # recipe -> rawtherapee .pp3 profiles
  -> rawtherapee-cli    # non-destructive render (source RAW is never modified)
  -> artifact           # JPEG/TIFF/PNG + auditable execution log
```

Detailed walkthrough: [docs/architecture.md](docs/architecture.md).

## Installation

Prebuilt binaries are attached to every [GitHub Release](https://github.com/hvlcrs/agbr/releases) — no Rust toolchain required.

```bash
# macOS (Apple Silicon)
curl -L -o agbr https://github.com/hvlcrs/agbr/releases/latest/download/agbr-aarch64-apple-darwin
chmod +x agbr && sudo mv agbr /usr/local/bin/

# Linux (x86_64)
curl -L -o agbr https://github.com/hvlcrs/agbr/releases/latest/download/agbr-x86_64-unknown-linux-gnu
chmod +x agbr && sudo mv agbr /usr/local/bin/

# Windows (x86_64) — PowerShell
Invoke-WebRequest https://github.com/hvlcrs/agbr/releases/latest/download/agbr-x86_64-pc-windows-msvc.exe -OutFile agbr.exe
```

Then install the render backend and verify:

```bash
brew install --cask rawtherapee      # macOS; see docs/config.md for Linux/Windows
agbr capabilities
```

Building from source instead? `cargo build --release` produces `target/release/agbr` (Rust 1.96+, see [docs/release.md](docs/release.md)).

## Commands

`agbr help` lists everything; full output of every command is in [docs/commands.md](docs/commands.md).

| Command | What it does |
|---|---|
| `agbr inspect <photo>` | Read EXIF -> image context |
| `agbr recipe create <photo> --prompt "..."` | LLM generates a validated recipe |
| `agbr recipe validate <recipe>` | Validate a recipe file |
| `agbr plan --recipe <recipe>` | Check the recipe against backend capabilities |
| `agbr preview <photo> --recipe <recipe>` | Downscaled preview |
| `agbr apply <photo> --recipe <recipe>` | Full-resolution render |
| `agbr export <photo> --format jpg\|tif\|png` | Export with explicit format/quality |
| `agbr mcp serve` | MCP server over stdio (for agents) |
| `agbr capabilities` / `agbr config` | Backend capabilities / resolved config |

## Quick start

```bash
brew install --cask rawtherapee        # render backend (macOS)
cargo build --release

# 1. Inspect the photo (EXIF context).
./target/release/agbr inspect ~/photos/IMG_001.ARW

# 2. Generate a recipe from natural-language intent.
./target/release/agbr recipe create ~/photos/IMG_001.ARW \
  --prompt "Nostalgic cinematic film. Focus on the people."

# 3. Check what will actually execute, then render.
./target/release/agbr plan --recipe ~/agbr/recipes/IMG_001.json
./target/release/agbr apply ~/photos/IMG_001.ARW \
  --recipe ~/agbr/recipes/IMG_001.json
```

Output lands in `~/agbr/exports/`. No LLM key? Use `--mock` for offline recipe generation.

## Documentation

| Topic | File |
|---|---|
| Command reference (full `--help` output) | [docs/commands.md](docs/commands.md) |
| CLI workflows | [docs/cli.md](docs/cli.md) |
| MCP server for agents | [docs/mcp.md](docs/mcp.md) |
| Configuration reference | [docs/config.md](docs/config.md) |
| Recipe format and schema | [docs/recipe.md](docs/recipe.md) |
| Architecture and application logic | [docs/architecture.md](docs/architecture.md) |
| Releasing and CI/CD | [docs/release.md](docs/release.md) |

For agent-facing repo guidance (crate map, conventions, commands), see [AGENTS.md](AGENTS.md).

## Status

Implemented: global edits — `inspect`, `recipe create/validate`, `plan`, `preview`, `apply`, `export`, `mcp serve`, `capabilities`, `config`. Local editing/masks, visual reasoning, and the recipe library are tracked as GitHub issues.

Cross-platform: macOS, Linux, Windows.

## License

MIT
