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

## Application logic

The CLI and MCP server are thin interfaces; every operation is dispatched through the engine (`crates/cli/src/engine.rs`). The two core flows:

### Recipe creation (intent -> recipe)

1. **Inspect**: `agbr inspect <photo>` parses EXIF (`crates/inspect`) into an `ImageContext` (camera, lens, ISO, aperture, shutter, focal length) — the only signal the LLM gets about the photo.
2. **Generate**: `agbr recipe create` sends the prompt + image context to the LLM gateway (`crates/llm`, OpenAI-compatible). The model returns a `RecipeDraft` as JSON (`json_mode = false` uses prompt-based JSON with tolerant parsing — required for vision models like Nano Banana that reject `response_format: json_object`).
3. **Harden**: the draft is converted into a `PhotoRecipe`: source path recorded, provenance added, defaults filled in.
4. **Validate**: `validate_recipe` (`crates/core`) checks the schema and normalized ranges (`exposure_ev` in EV, `contrast/highlights/shadows/saturation/tint` in `[-1,1]`, `temperature_k` in Kelvin, `grain` in `[0,1]`). The canonical `recipe_hash` is computed.
5. **Persist**: the validated recipe is written to `<workspace>/recipes/<stem>.json`. Recipes are immutable once saved — edit by regenerating.

### Rendering (recipe -> artifact)

1. **Plan**: `planner::plan` (`crates/planner`) walks the recipe against the backend's `Capabilities`, producing `Plan { recipe_hash, steps, warnings }`. Anything unsupported becomes a **warning** — the recipe is never silently altered.
2. **PP3 generation**: `crates/rawtherapee` maps each supported global operation to RawTherapee 5.13 PP3 keys and writes at most three profiles:
   - `base.pp3` — neutral/corrective starting point (lens + CA corrections via `LcMode=lfauto`, denoise, sharpening baseline), resolved from `profiles/base/`
   - `look.pp3` — the recipe's global adjustments (exposure, WB, tone curve, contrast, saturation, highlights/shadows)
   - `resize.pp3` — previews only (long edge 1600px, JPEG q90)
3. **Invoke**: the CLI binary is discovered cross-platform (PATH -> platform install locations; override with `RAWTHERAPEE_CLI` / `[rawtherapee].binary`) and invoked as `rawtherapee-cli -q -o <out> -p base.pp3 -p look.pp3 [-p resize.pp3] -j95 -Y -c <input>`.
4. **Record**: the exact command, output/source sha256 hashes, backend version, and recipe hash are appended to `<workspace>/logs/execution.jsonl`, and returned to the caller as JSON.

The recipe never executes directly — `plan` gates it, PP3 generation translates it, and only `rawtherapee-cli` touches pixels.

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

## Design rules

- **Unsupported operations are reported, never silently dropped.** The planner emits warnings for anything the backend can't do (e.g. local masks).
- **The source RAW is immutable.** Everything generated lives under the workspace.
- **Values are normalized** (see [recipe.md](recipe.md)) so recipes are backend-agnostic.
- **Metadata and filenames are untrusted data** (prompt-injection surface); only parsed, never executed.

## Status

| Scope | Status |
|---|---|
| Global edits: inspect, recipe create/validate, plan, preview, apply, export, MCP | Implemented |
| Visual reasoning / choosing the best frame | GitHub issue |
| Local editing / masks | GitHub issue |
| Recipe library | GitHub issue |
