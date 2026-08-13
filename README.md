# agbr

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
| CLI reference and workflows | [docs/cli.md](docs/cli.md) |
| MCP server for agents | [docs/mcp.md](docs/mcp.md) |
| Configuration reference | [docs/config.md](docs/config.md) |
| Recipe format and schema | [docs/recipe.md](docs/recipe.md) |
| Architecture and application logic | [docs/architecture.md](docs/architecture.md) |

For agent-facing repo guidance (crate map, conventions, commands), see [AGENTS.md](AGENTS.md).

## Status

Implemented: global edits — `inspect`, `recipe create/validate`, `plan`, `preview`, `apply`, `export`, `mcp serve`, `capabilities`, `config`. Local editing/masks, visual reasoning, and the recipe library are tracked as GitHub issues.

Cross-platform: macOS, Linux, Windows.

## License

MIT
