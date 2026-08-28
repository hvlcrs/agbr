# CLI

`agbr` is a terminal-native tool. Every operation is a subcommand that reads/writes JSON over stdio. The source RAW is never modified; all artifacts land in the workspace.

> The exact `--help` output of every command lives in [commands.md](commands.md); this page covers behavior and workflows.

## Global behavior

- All render commands return JSON with `output`, `output_hash` (sha256), `source_hash`, `backend_version`, and the exact `command` that was executed.
- Exit code `0` on success; non-zero with a message on failure.
- Use `-h`/`--help` on any subcommand for exact flags.

## Subcommands

### inspect

Read EXIF metadata and produce an image context (read-only).

```bash
agbr inspect ~/photos/IMG_001.ARW
```

Output: camera, lens, ISO, aperture, shutter, focal length, and source format. This context is what the LLM sees when generating a recipe.

### capabilities

Report the installed backend's runtime capabilities.

```bash
agbr capabilities
```

Output: the backend version, which global operations are supported, whether local masking is available (currently none), and whether profile layering/headless export work.

### config

Print the resolved configuration (file + defaults merged).

```bash
agbr config
```

Useful for debugging: it shows the effective provider, model, workspace paths, and backend binary. See [config.md](config.md) for all keys.

### recipe create

Generate a `PhotoRecipe` from natural-language intent (requires LLM config, or `--mock`).

```bash
agbr recipe create ~/photos/IMG_001.ARW \
  --prompt "Nostalgic cinematic film. Focus on the people."
```

```bash
agbr recipe create ~/photos/IMG_001.ARW --mock \
  --prompt "Nostalgic cinematic film."
```

- Writes the validated recipe to `<workspace>/recipes/<stem>.json`.
- `--mock` uses a deterministic offline provider — no API key needed.
- Localized (subject/mask) operations are only *produced* here if the model chooses them; they cannot be executed yet (see `plan`).

### recipe validate

Validate an existing recipe JSON file without executing anything.

```bash
agbr recipe validate ~/agbr/recipes/IMG_001.json
```

Reports the recipe hash and any validation errors.

### plan

Plan a recipe against backend capabilities without executing.

```bash
agbr plan --recipe ~/agbr/recipes/IMG_001.json
```

Output: recipe hash, supported steps, and warnings. Unsupported operations are **reported, never silently dropped** — run this before `preview`/`apply` to see what will actually happen.

### preview

Render a downscaled preview (long edge 1600px, JPEG q90).

```bash
agbr preview ~/photos/IMG_001.ARW --recipe ~/agbr/recipes/IMG_001.json
```

Writes to `<workspace>/previews/<stem>-preview.jpg`. Fast iteration before committing to a full render.

### apply

Apply a recipe and render the full-resolution result.

```bash
agbr apply ~/photos/IMG_001.ARW --recipe ~/agbr/recipes/IMG_001.json
```

Writes to `<workspace>/exports/<stem>.jpg` (JPEG q95 by default). This is the main "develop" step.

### export

Export with an explicit format/quality, optionally applying a recipe.

```bash
agbr export ~/photos/IMG_001.ARW --format tif
agbr export ~/photos/IMG_001.ARW --format png
agbr export ~/photos/IMG_001.ARW --format jpg --quality 100
agbr export ~/photos/IMG_001.ARW --format jpg --recipe ~/agbr/recipes/IMG_001.json
```

`--format` is `jpg` (default), `tif`, or `png`. Without a recipe, the render uses the neutral base profile only.

### mcp serve

Start the MCP server over stdio (see [mcp.md](mcp.md)).

```bash
agbr mcp serve
```

## Batch processing

`preview`, `apply`, `export`, and `recipe create` accept **multiple photos**. Each argument may be:

- a concrete file
- a directory (scanned recursively for supported image files)
- a glob pattern (e.g. `~/shoot/**/*.ARW`; a leading `~/` is expanded)

When more than one photo resolves, the command processes all of them and prints a summary:

```bash
agbr apply '~/shoot/**/*.ARW' --recipe ~/agbr/recipes/look.json --jobs 4
```

```json
{
  "processed": 120,
  "succeeded": 118,
  "failed": 2,
  "jobs": 4,
  "results": [ { "source": "...", "output": "...", ... }, ... ]
}
```

- `--jobs N` bounds concurrency (default `1`); RawTherapee is CPU-heavy, so tune to your core count.
- A single resolved photo keeps the original single-object output shape (backward compatible).
- `recipe create` in batch mode generates one recipe per photo from the shared prompt, using each photo's own EXIF context.
- The source RAW is never modified; per-photo outputs keep the existing `<stem>.<ext>` naming and every render is recorded in `logs/execution.jsonl`.

## Typical workflow

```bash
# 1. See what you're working with.
agbr inspect ~/photos/IMG_001.ARW

# 2. Generate a recipe from intent.
agbr recipe create ~/photos/IMG_001.ARW --prompt "Bright, airy, natural light."

# 3. Check what will actually execute.
agbr plan --recipe ~/agbr/recipes/IMG_001.json

# 4. Fast preview, iterate on the prompt if needed.
agbr preview ~/photos/IMG_001.ARW --recipe ~/agbr/recipes/IMG_001.json

# 5. Full-resolution render.
agbr apply ~/photos/IMG_001.ARW --recipe ~/agbr/recipes/IMG_001.json
```

## Output locations

| Artifact | Default location |
|---|---|
| Recipes | `<workspace>/recipes/` |
| Generated PP3 profiles | `<workspace>/profiles/` |
| Previews | `<workspace>/previews/` |
| Exports | `<workspace>/exports/` |
| Execution log | `<workspace>/logs/execution.jsonl` |

Workspace paths are configurable; see [config.md](config.md).
