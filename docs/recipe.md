# PhotoRecipe format

A `PhotoRecipe` is the typed, editor-independent description of editing intent. The LLM produces a draft; the engine adds source/provenance, validates, and stores it as JSON. It is the single contract between intent and rendering — the LLM never writes PP3 directly.

Canonical schema: [`schemas/recipe.schema.json`](../schemas/recipe.schema.json).

## Example

```json
{
  "version": "1.0",
  "intent": {
    "vibe": "Nostalgic cinematic film",
    "notes": ["Keep it natural, focus on the people"]
  },
  "source": {
    "path": "IMG_001.ARW"
  },
  "global": {
    "exposure_ev": 0.5,
    "contrast": 0.1,
    "highlights": -0.2,
    "shadows": 0.2,
    "saturation": 0.1,
    "tone_curve": {
      "points": [[0.0, 0.0], [0.25, 0.23], [0.5, 0.5], [0.75, 0.77], [1.0, 1.0]]
    },
    "white_balance": {
      "temperature_k": 6000,
      "tint": 0.0
    }
  },
  "local": [
    {
      "target": { "label": "monk", "type": "semantic_region" },
      "operations": { "exposure_ev": 0.2, "highlights": 0.3 }
    }
  ],
  "constraints": { "preserve_skin": false, "no_crop": false, "no_generation": true }
}
```

Saved recipes also carry a `provenance` section (creation timestamp, producer) and are stored at `<workspace>/recipes/<stem>.json`.

## Normalized ranges

| Field | Unit / range |
|---|---|
| `exposure_ev` | EV (positive = brighter) |
| `contrast`, `highlights`, `shadows`, `saturation`, `tint` | `[-1, 1]` |
| `temperature_k` | Kelvin, `[1000, 20000]` |
| `grain` | `[0, 1]` |

## Sections

- **intent** — the human/agent prompt and notes that produced the recipe (provenance).
- **source** — the immutable input file (path recorded at creation time; hashes are recorded in the execution log at render time).
- **global** — adjustments applied to the whole image (currently executed).
- **local** — region/subject-targeted operations (`semantic_region`, masks). Produced by the model, **validated and planned, but not yet executable** — `plan` reports them as warnings; tracked as a GitHub issue (local editing/masks).
- **constraints** — optional hard constraints (schema-defined) on the recipe.

## Lifecycle

```
draft (LLM) -> source + provenance added -> validate -> plan (capability gate) -> PP3 -> render
```

- `agbr recipe validate` checks a recipe file against the schema/range rules.
- `agbr plan` checks it against the installed backend's capabilities.
- Recipes live in `<workspace>/recipes/` and are immutable once saved; edit by regenerating.
