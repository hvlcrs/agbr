# Architecture

Core invariant:

> **The LLM controls editing intent. RawTherapee controls image processing.**

`agbr` is a control plane, not an editor. It turns natural-language intent into a typed, validated `PhotoRecipe`, then renders that recipe non-destructively with RawTherapee. The LLM never writes PP3, never edits pixels, and never emits shell commands.

## Pipeline

```
intent -> LLM gateway -> RecipeDraft
       -> PhotoRecipe (source + provenance added)
       -> validate
       -> planner::plan (capability gating)
       -> rawtherapee: base.pp3 + look.pp3 (+ resize.pp3 for preview)
       -> rawtherapee-cli
       -> rendered artifact + execution log
```

Every render records its exact `command`, hashes, backend version, and recipe hash in `<workspace>/logs/execution.jsonl` — the pipeline is fully auditable.

## Crates

| Crate | Package | Purpose |
|---|---|---|
| `crates/core` | `agbr-core` | `PhotoRecipe` model, validation, canonical hash, capabilities, provenance, regions |
| `crates/llm` | `agbr-llm` | OpenAI-compatible gateway (`LlmProvider`) + `MockProvider` |
| `crates/inspect` | `agbr-inspect` | EXIF -> image context (read-only) |
| `crates/planner` | `agbr-planner` | capability gating, warnings, execution ordering |
| `crates/rawtherapee` | `agbr-rawtherapee` | PP3 generation, base-profile resolver, CLI discovery/command construction |
| `crates/mcp` | `agbr-mcp` | minimal MCP server over stdio (protocol only) |
| `crates/cli` | `agbr` (bin `agbr`) | the engine + clap CLI + MCP tool wiring |

The **engine** (`crates/cli/src/engine.rs`) is the single source of truth; the CLI and MCP server are thin interfaces over it.

## Rendering

A render layers at most three PP3 profiles:

1. **base** — neutral/corrective starting point (lens + CA corrections via `LcMode=lfauto`, denoise, sharpening baseline)
2. **look** — the recipe's global adjustments (exposure, WB, tone, etc.)
3. **resize** — previews only (long edge 1600px)

PP3 keys mirror RawTherapee 5.13 (`[Exposure]`, `[White Balance]`, `[Sharpening]`, `[Resize]`, ...).

## Design rules

- **Unsupported operations are reported, never silently dropped.** The planner emits warnings for anything the backend can't do (e.g. local masks in Phase 1).
- **The source RAW is immutable.** Everything generated lives under the workspace.
- **Values are normalized** (see [recipe.md](recipe.md)) so recipes are backend-agnostic.
- **Metadata and filenames are untrusted data** (prompt-injection surface); only parsed, never executed.

## Phases

| Phase | Scope | Status |
|---|---|---|
| 1 | Global edits: inspect, recipe create/validate, plan, preview, apply, export, MCP | Implemented |
| 2 | Visual reasoning / choosing the best frame | GitHub issue |
| 3 | Local editing / masks | GitHub issue |
| 4 | Recipe library | GitHub issue |
