# Command reference

Every `agbr` command is documented inline — `agbr help <command>` always shows the current truth. This page embeds the exact output of `agbr --help` and each subcommand's help (as of v0.1.0).

## Global usage

```
Terminal-native AI RAW processing engine. Non-destructive .pp3 sidecar generation, automated lens/CA corrections, and cloud-segmented regional compositing.

Usage: agbr <COMMAND>

Commands:
  inspect       Read EXIF metadata and produce an image context
  capabilities  Report the installed backend's runtime capabilities
  config        Print the resolved configuration
  recipe        Generate, validate, and manage recipes
  plan          Plan a recipe against backend capabilities without executing
  preview       Render a downscaled preview
  apply         Apply a recipe and render the full-resolution result
  export        Export with an explicit format/quality (optionally with a recipe)
  mcp           MCP interface
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## inspect

```
Usage: agbr inspect <PHOTO>

Arguments:
  <PHOTO>  Path to the source RAW file

Options:
  -h, --help  Print help
```

Reads EXIF and prints an image context (camera, lens, ISO, aperture, shutter, focal length) as JSON.

## capabilities

```
Usage: agbr capabilities

Options:
  -h, --help  Print help
```

Reports the installed backend's runtime capabilities: version, supported global operations, whether local masking exists (not yet), profile layering, and headless export.

## config

```
Usage: agbr config

Options:
  -h, --help  Print help
```

Prints the fully resolved configuration (file + defaults merged): provider, model, workspace paths, backend binary. Debugging starts here.

## recipe

```
Usage: agbr recipe <COMMAND>

Commands:
  create    Generate a PhotoRecipe from natural-language intent
  validate  Validate an existing recipe JSON file
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### recipe create

```
Usage: agbr recipe create [OPTIONS] --prompt <PROMPT> [PHOTOS]...

Arguments:
  [PHOTOS]...  One or more photos (files, directories, or glob patterns)

Options:
      --prompt <PROMPT>  
      --mock             Use the deterministic mock provider (offline)
  -h, --help             Print help
```

Sends the natural-language prompt (plus each photo's EXIF context) to the LLM and writes a validated `PhotoRecipe` to `<workspace>/recipes/<stem>.json` per photo. `--mock` uses a deterministic offline provider (no API key). Multiple photos (or a directory/glob) produce one recipe per photo with a shared prompt.

### recipe validate

```
Usage: agbr recipe validate <RECIPE>

Arguments:
  <RECIPE>  

Options:
  -h, --help  Print help
```

Validates an existing recipe JSON file against the schema and normalized ranges; prints the recipe hash and any errors.

## plan

```
Usage: agbr plan --recipe <RECIPE>

Options:
      --recipe <RECIPE>  Path to a recipe JSON file
  -h, --help             Print help
```

Plans a recipe against backend capabilities without executing: supported steps, warnings for anything unsupported (never silently dropped).

## preview

```
Usage: agbr preview [OPTIONS] --recipe <RECIPE> [PHOTOS]...

Arguments:
  [PHOTOS]...  One or more photos (files, directories, or glob patterns)

Options:
      --recipe <RECIPE>  
      --jobs <JOBS>      Max concurrent renders [default: 1]
  -h, --help             Print help
```

Renders a downscaled preview (long edge 1600px, JPEG q90) to `<workspace>/previews/<stem>-preview.jpg`. Multiple photos run in batch with `--jobs` concurrency.

## apply

```
Usage: agbr apply [OPTIONS] --recipe <RECIPE> [PHOTOS]...

Arguments:
  [PHOTOS]...  One or more photos (files, directories, or glob patterns)

Options:
      --recipe <RECIPE>  
      --jobs <JOBS>      Max concurrent renders [default: 1]
  -h, --help             Print help
```

Applies the recipe and renders the full-resolution result to `<workspace>/exports/<stem>.jpg` (JPEG q95). Multiple photos run in batch with `--jobs` concurrency.

## export

```
Usage: agbr export [OPTIONS] [PHOTOS]...

Arguments:
  [PHOTOS]...  One or more photos (files, directories, or glob patterns)

Options:
      --format <FORMAT>    Output format: jpg, tif, or png [default: jpg]
      --quality <QUALITY>  JPEG quality (1-100)
      --recipe <RECIPE>    Optional recipe to apply during export
      --jobs <JOBS>        Max concurrent renders [default: 1]
  -h, --help               Print help
```

Exports with an explicit format/quality. Without a recipe, renders the neutral base profile only. Multiple photos run in batch with `--jobs` concurrency.

## mcp

```
Usage: agbr mcp <COMMAND>

Commands:
  serve  Start the MCP server over stdio
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

`agbr mcp serve` starts the MCP server over stdio (newline-delimited JSON-RPC) for agent clients. See [mcp.md](mcp.md).
