# agbr

Terminal-native AI RAW processing engine. Describe the look in plain language; `agbr` turns it into a validated recipe and renders it non-destructively with RawTherapee.

> **The LLM controls editing intent. RawTherapee controls image processing.**

## How it works

1. `agbr inspect` reads your photo's EXIF.
2. `agbr recipe create` sends your natural-language intent (e.g. *"nostalgic cinematic film"*) to an LLM, which returns a typed, validated `PhotoRecipe`.
3. `agbr plan` checks the recipe against the installed RawTherapee — anything unsupported is reported, never silently dropped.
4. `agbr preview` / `agbr apply` render the result via `rawtherapee-cli`. Your RAW file is never modified.

## Quick start

```bash
brew install --cask rawtherapee        # render backend (macOS)
cargo build --release

./target/release/agbr recipe create ~/photos/IMG_001.ARW \
  --prompt "Nostalgic cinematic film. Focus on the people."

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
| Architecture and design | [docs/architecture.md](docs/architecture.md) |

For agent-facing repo guidance (crate map, conventions, commands), see [AGENTS.md](AGENTS.md).

## Status

**Phase 1 (global edits)** is implemented: `inspect`, `recipe create/validate`, `plan`, `preview`, `apply`, `export`, `mcp serve`, `capabilities`, `config`. Phases 2–4 (visual reasoning, local editing/masks, recipe library) are tracked as GitHub issues.

Cross-platform: macOS, Linux, Windows.

## License

MIT
