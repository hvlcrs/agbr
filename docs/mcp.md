# MCP server

`agbr mcp serve` exposes the same engine as the CLI over the Model Context Protocol (MCP), so agents (Claude Code, Codex, Cursor, etc.) can drive RAW editing directly.

## Protocol

- Transport: **stdio** only (newline-delimited JSON-RPC 2.0).
- `mcp serve` reads requests from stdin and writes responses to stdout; logs go to stderr.
- Start it with `agbr mcp serve` and hand it to your agent client's MCP configuration as a stdio server.

Example client config (Claude Code style):

```json
{
  "mcpServers": {
    "agbr": {
      "command": "agbr",
      "args": ["mcp", "serve"]
    }
  }
}
```

## Tools

All tools are thin wrappers over the engine; behavior matches the CLI equivalents in [cli.md](cli.md).

| Tool | Description |
|---|---|
| `photo.inspect` | Inspect a RAW photo: read EXIF and produce an image context. |
| `photo.capabilities` | Report the installed backend's runtime capabilities. |
| `photo.recipe.create` | Generate a `PhotoRecipe` from natural-language intent (via the LLM gateway; `mock` flag for offline). |
| `photo.recipe.validate` | Validate a recipe JSON file. |
| `photo.plan` | Plan a recipe against backend capabilities without executing. |
| `photo.preview` | Render a downscaled preview of a recipe. |
| `photo.apply` | Apply a recipe and render the full-resolution result. |
| `photo.apply.batch` | Apply a recipe to multiple photos (files, directories, or glob patterns) and render them. |
| `photo.export` | Export a photo with an explicit format and quality. |

## Typical agent flow

```
photo.inspect(photo)          -> context (camera, lens, ISO, ...)
photo.recipe.create(photo,
  prompt)                     -> recipe (validated, saved to workspace)
photo.plan(recipe)            -> supported steps + warnings
photo.preview(photo, recipe)  -> downscaled preview path
photo.apply(photo, recipe)    -> full-res export path
```

## Notes

- Paths are resolved on the machine running the server; the agent sees filesystem paths, not image bytes.
- Unsupported operations surface as warnings in `photo.plan` output — never silently dropped.
- The LLM API key and config come from the same sources as the CLI ([config.md](config.md)); the MCP server never exposes the key to clients.
