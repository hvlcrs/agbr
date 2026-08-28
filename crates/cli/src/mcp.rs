//! MCP tool surface — maps MCP tools onto the engine (design section 14).

use std::sync::Arc;

use serde_json::{json, Value};

use agbr_mcp::{serve_stdio, ServerInfo, Tool};

use crate::engine::Engine;

/// Build the MCP tools exposed by the engine.
pub fn tools(engine: Arc<Engine>) -> Vec<Tool> {
    let e_inspect = engine.clone();
    let e_caps = engine.clone();
    let e_create = engine.clone();
    let e_validate = engine.clone();
    let e_plan = engine.clone();
    let e_preview = engine.clone();
    let e_apply = engine.clone();
    let e_apply_batch = engine.clone();
    let e_export = engine.clone();

    vec![
        Tool::new(
            "photo.inspect",
            "Inspect a RAW photo: read EXIF metadata and produce an image context.",
            object_schema(&[("photo", "string")], &[]),
            move |args| {
                let e = e_inspect.clone();
                async move {
                    let path = arg_str(&args, "photo")?;
                    e.inspect(std::path::Path::new(&path))
                        .map_err(|e| e.to_string())
                }
            },
        ),
        Tool::new(
            "photo.capabilities",
            "Report the installed backend's runtime capabilities.",
            json!({ "type": "object", "properties": {} }),
            move |_args| {
                let e = e_caps.clone();
                async move { serde_json::to_value(e.capabilities()).map_err(|e| e.to_string()) }
            },
        ),
        Tool::new(
            "photo.recipe.create",
            "Generate a PhotoRecipe from natural-language intent (via the configured LLM).",
            object_schema(
                &[("photo", "string"), ("instruction", "string")],
                &[("mock", "boolean")],
            ),
            move |args| {
                let e = e_create.clone();
                async move {
                    let path = arg_str(&args, "photo")?;
                    let instruction = arg_str(&args, "instruction")?;
                    let mock = args.get("mock").and_then(Value::as_bool).unwrap_or(false);
                    e.recipe_create(std::path::Path::new(&path), &instruction, mock)
                        .await
                        .map_err(|e| e.to_string())
                }
            },
        ),
        Tool::new(
            "photo.recipe.validate",
            "Validate a recipe JSON file.",
            object_schema(&[("recipe", "string")], &[]),
            move |args| {
                let e = e_validate.clone();
                async move {
                    let recipe = arg_str(&args, "recipe")?;
                    e.recipe_validate(std::path::Path::new(&recipe))
                        .map_err(|e| e.to_string())
                }
            },
        ),
        Tool::new(
            "photo.plan",
            "Plan a recipe against backend capabilities without executing it.",
            object_schema(&[("recipe", "string")], &[]),
            move |args| {
                let e = e_plan.clone();
                async move {
                    let recipe = arg_str(&args, "recipe")?;
                    e.plan(std::path::Path::new(&recipe))
                        .map_err(|e| e.to_string())
                }
            },
        ),
        Tool::new(
            "photo.preview",
            "Render a downscaled preview of a recipe.",
            object_schema(&[("photo", "string"), ("recipe", "string")], &[]),
            move |args| {
                let e = e_preview.clone();
                async move {
                    let photo = arg_str(&args, "photo")?;
                    let recipe = arg_str(&args, "recipe")?;
                    e.preview(std::path::Path::new(&photo), std::path::Path::new(&recipe))
                        .await
                        .map_err(|e| e.to_string())
                }
            },
        ),
        Tool::new(
            "photo.apply",
            "Apply a recipe and render the full-resolution result.",
            object_schema(&[("photo", "string"), ("recipe", "string")], &[]),
            move |args| {
                let e = e_apply.clone();
                async move {
                    let photo = arg_str(&args, "photo")?;
                    let recipe = arg_str(&args, "recipe")?;
                    e.apply(std::path::Path::new(&photo), std::path::Path::new(&recipe))
                        .await
                        .map_err(|e| e.to_string())
                }
            },
        ),
        Tool::new(
            "photo.apply.batch",
            "Apply a recipe to multiple photos (files, directories, or glob patterns) and render them.",
            object_schema(
                &[("photos", "array"), ("recipe", "string")],
                &[("jobs", "integer")],
            ),
            move |args| {
                let e = e_apply_batch.clone();
                async move {
                    let photos = arg_str_array(&args, "photos")?
                        .iter()
                        .map(std::path::PathBuf::from)
                        .collect::<Vec<_>>();
                    let sources = crate::batch::expand(&photos).map_err(|e| e.to_string())?;
                    let recipe = arg_str(&args, "recipe")?;
                    let jobs = args
                        .get("jobs")
                        .and_then(Value::as_u64)
                        .unwrap_or(1)
                        .clamp(1, 64) as usize;
                    e.apply_batch(&sources, std::path::Path::new(&recipe), jobs)
                        .map_err(|e| e.to_string())
                }
            },
        ),
        Tool::new(
            "photo.export",
            "Export a photo with an explicit format and quality.",
            object_schema(
                &[("photo", "string")],
                &[
                    ("format", "string"),
                    ("quality", "integer"),
                    ("recipe", "string"),
                ],
            ),
            move |args| {
                let e = e_export.clone();
                async move {
                    let photo = arg_str(&args, "photo")?;
                    let format = match args.get("format").and_then(Value::as_str).unwrap_or("jpg") {
                        "tif" | "tiff" => agbr_rawtherapee::cli::OutputFormat::Tiff,
                        "png" => agbr_rawtherapee::cli::OutputFormat::Png,
                        _ => {
                            let q = args
                                .get("quality")
                                .and_then(Value::as_u64)
                                .unwrap_or(95)
                                .clamp(1, 100) as u8;
                            agbr_rawtherapee::cli::OutputFormat::Jpeg { quality: q }
                        }
                    };
                    let recipe = args.get("recipe").and_then(Value::as_str);
                    e.export(
                        std::path::Path::new(&photo),
                        format,
                        recipe.map(std::path::Path::new),
                    )
                    .await
                    .map_err(|e| e.to_string())
                }
            },
        ),
    ]
}

/// Run the MCP server (blocking until stdin closes).
pub async fn serve(engine: Arc<Engine>) -> Result<(), agbr_mcp::McpError> {
    serve_stdio(
        ServerInfo {
            name: "agbr".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        Arc::new(tools(engine)),
    )
    .await
}

fn object_schema(required_fields: &[(&str, &str)], optional_fields: &[(&str, &str)]) -> Value {
    let mut properties = serde_json::Map::new();
    for (name, ty) in required_fields.iter().chain(optional_fields) {
        properties.insert((*name).to_string(), json!({ "type": ty }));
    }
    let required: Vec<Value> = required_fields
        .iter()
        .map(|(name, _)| Value::String((*name).to_string()))
        .collect();
    json!({ "type": "object", "properties": properties, "required": required })
}

fn arg_str(args: &Value, name: &str) -> Result<String, String> {
    args.get(name)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing required argument '{name}'"))
}

fn arg_str_array(args: &Value, name: &str) -> Result<Vec<String>, String> {
    args.get(name)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .ok_or_else(|| format!("missing or empty required argument '{name}'"))
}
