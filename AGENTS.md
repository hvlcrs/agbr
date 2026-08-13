# AGENTS.md

Guidance for AI agents (Codex, Claude Code, Cursor, and other agentic tools) working in this repository.

## What this is

`agbr` is a headless, agent-first control plane for non-destructive RAW photo editing.

Core invariant:

> **The LLM controls editing intent. RawTherapee controls image processing.**

Natural language → `PhotoRecipe` (typed, editor-independent) → validate/plan → RawTherapee PP3 → `rawtherapee-cli`. The LLM never writes PP3, never edits pixels, and never emits shell commands.

## Commands

```bash
cargo build                          # build everything
cargo test                           # unit tests
cargo clippy --all-targets           # lint (must be clean)
cargo fmt --all -- --check           # format check
cargo run --bin agbr -- <subcommand> # run the CLI
```

RawTherapee is a runtime dependency for render/preview/apply/export. It is auto-discovered cross-platform (PATH, then platform install locations). Override with `RAWTHERAPEE_CLI` or `[rawtherapee].binary`.

## Crate map (workspace)

| Crate | Package | Purpose |
|---|---|---|
| `crates/core` | `agbr-core` | `PhotoRecipe` model, validation, canonical hash, capabilities, provenance, regions |
| `crates/llm` | `agbr-llm` | OpenAI-compatible gateway (`LlmProvider`) + `MockProvider` |
| `crates/inspect` | `agbr-inspect` | EXIF → `ImageContext` / `TechnicalInfo` (read-only) |
| `crates/planner` | `agbr-planner` | capability gating, warnings, execution ordering |
| `crates/rawtherapee` | `agbr-rawtherapee` | PP3 generation, base-profile resolver, CLI discovery/command construction |
| `crates/mcp` | `agbr-mcp` | minimal MCP server over stdio (protocol only) |
| `crates/cli` | `agbr` (bin `agbr`) | the engine + clap CLI + MCP tool wiring |

The **engine** (`crates/cli/src/engine.rs`) is the single source of truth; CLI and MCP are thin interfaces over it.

## Data flow

```
intent -> LLM (gateway) -> RecipeDraft
        -> PhotoRecipe (source + provenance added)
        -> validate_recipe
        -> planner::plan (capability gating)
        -> rawtherapee: base.pp3 + look.pp3 (+ resize.pp3 for preview)
        -> rawtherapee-cli
        -> rendered artifact + execution log
```

## Conventions

- No `photo-` prefixes on crate directories. Schema is `schemas/recipe.schema.json`.
- Recipe values are normalized: `contrast/highlights/shadows/saturation` in `[-1,1]`, `exposure_ev` in EV, `grain` in `[0,1]`, `temperature_k` in Kelvin, `tint` in `[-1,1]`.
- The source RAW is immutable. Recipes/profiles/previews/logs/rendered exports all live under the workspace (`~/agbr` by default).
- Unsupported operations are **reported**, never silently dropped (see `GeneratedProfile::unsupported` and `Plan::warnings`).
- Image metadata/filenames are untrusted data (prompt-injection surface).
- PP3 key names must mirror RawTherapee 5.13 (reference: bundled profiles in `RawTherapee.app/Contents/Resources/share/profiles/`, and `rtengine/procparams.cc` upstream).
- Cross-platform: macOS, Linux, and Windows. Paths resolve via the `dirs` crate; `rawtherapee-cli` discovery is platform-aware.

## Configuration

`~/.config/agbr/config.toml` (or `$AGBR_CONFIG`) on macOS/Linux, `%APPDATA%\agbr\config.toml` on Windows. LLM is OpenAI-compatible; `provider = "openrouter"` uses OpenRouter by default:

```toml
[llm]
provider = "openrouter"     # "openai" | "openrouter" | "mock"
base_url = ""               # empty -> OpenRouter default
model = "google/gemini-2.5-flash-image"   # "Nano Banana" (vision); supports structured output
api_key = ""                # or OPENROUTER_API_KEY / OPENAI_API_KEY
json_mode = false           # Nano Banana rejects json_object mode

[rawtherapee]
binary = ""                 # optional override (auto-discovered cross-platform)

[workspace]
root = "~/agbr"             # recipes/profiles/previews/logs
export_dir = "~/agbr/exports"  # rendered exports (defaults to <workspace>/exports)

[policy]
allow = []                  # optional path allowlist (not yet enforced)
```

## Phases

Phase 1 (implemented): global edits — `inspect`, `recipe create/validate`, `plan`, `preview`, `apply`, `export`, `mcp serve`. See GitHub issues for Phases 2–4 (visual reasoning, local editing/masks, recipe library) and deferred items (grain mapping, dynamic base-profile resolution, policy enforcement).
