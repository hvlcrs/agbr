# Configuration

`agbr` reads a TOML config file, overridable per-option by environment variables. `agbr config` prints the fully resolved configuration.

## Config file location

| Platform | Path |
|---|---|
| macOS / Linux | `~/.config/agbr/config.toml` (or `$XDG_CONFIG_HOME/agbr/config.toml`) |
| Windows | `%APPDATA%\agbr\config.toml` |

Override the location entirely with `AGBR_CONFIG=/path/to/config.toml`.

## Full reference

```toml
[llm]
provider = "openrouter"     # "openai" | "openrouter" | "mock"
base_url = ""               # empty -> provider default (OpenRouter: https://openrouter.ai/api/v1)
model = "google/gemini-2.5-flash-image"   # Nano Banana (vision)
api_key = ""                # or OPENROUTER_API_KEY / OPENAI_API_KEY
json_mode = false           # false -> prompt-based JSON (needed for Nano Banana)

[rawtherapee]
binary = ""                 # optional override; auto-discovered cross-platform

[workspace]
root = "~/agbr"             # recipes/, profiles/, previews/, logs/
export_dir = "~/agbr/exports"   # rendered exports (defaults to <root>/exports)

[policy]
allow = []                  # optional path allowlist (not yet enforced)
```

## Environment variables

| Variable | Effect |
|---|---|
| `AGBR_CONFIG` | Path to an alternative config file |
| `OPENROUTER_API_KEY` | API key when `provider = "openrouter"` |
| `OPENAI_API_KEY` | API key when `provider = "openai"` |
| `RAWTHERAPEE_CLI` | Explicit path to `rawtherapee-cli` (overrides auto-discovery) |

## Providers

- **openrouter** (default when configured): OpenAI-compatible gateway at `https://openrouter.ai/api/v1`. Use model ids like `google/gemini-2.5-flash-image`.
- **openai**: any OpenAI-compatible endpoint; set `base_url` for self-hosted/other gateways.
- **mock**: deterministic offline recipe generation; no key required. Useful for tests and CI.

`json_mode` note: some vision models (e.g. Nano Banana) reject `response_format: json_object`. With `json_mode = false` the gateway uses prompt-based JSON with tolerant parsing. Enable `json_mode = true` only for models that support structured output.

## Backend discovery

`rawtherapee-cli` is auto-discovered in this order:

1. `RAWTHERAPEE_CLI` env or `[rawtherapee].binary` config
2. `PATH` (`rawtherapee-cli` / `rawtherapee-cli.exe`)
3. Platform install locations (macOS: Homebrew + the `RawTherapee.app` bundle; Linux: `/usr/bin`, snap, flatpak; Windows: `RawTherapee\<version>\rawtherapee-cli.exe`)

## Workspace

The workspace holds every generated artifact; the source RAW is immutable and never touched. Layout:

```
<workspace>/
  recipes/       validated PhotoRecipe JSON
  profiles/      generated .pp3 profiles (base + look [+ resize])
  previews/      downscaled previews
  exports/       full-resolution renders
  logs/          execution.jsonl
```
