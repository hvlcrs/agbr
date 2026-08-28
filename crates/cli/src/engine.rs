//! The photo-control engine. CLI and MCP both invoke these functions.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use agbr_core::hash::{hash_str, recipe_hash};
use agbr_core::provenance::Provenance;
use agbr_core::{validate_recipe, BackendCapabilities, PhotoRecipe, RecipeDraft, Source};
use agbr_inspect as inspect;
use agbr_llm::{LlmProvider, LlmRequest, MockProvider, OpenAIProvider};
use agbr_rawtherapee::cli::{build_command, Cli, OutputFormat};
use agbr_rawtherapee::{
    generate_base_pp3, generate_look_pp3, rawtherapee_capabilities, BaseProfileInput, Pp3,
};

use crate::batch;
use crate::config::AppConfig;
use crate::prompts;
use crate::workspace::Workspace;

const DEFAULT_PREVIEW_LONG_EDGE: u32 = 1600;

/// The control-plane engine.
pub struct Engine {
    config: AppConfig,
    rt: Cli,
    workspace: Workspace,
}

impl Engine {
    pub fn new(config: AppConfig) -> Result<Self> {
        let rt = match &config.rawtherapee.binary {
            Some(binary) if !binary.is_empty() => Cli::with_binary(PathBuf::from(binary)),
            _ => Cli::discover().map_err(|e| anyhow!(e.to_string()))?,
        };
        let workspace = Workspace::new(config.workspace_root(), config.export_dir());
        Ok(Self {
            config,
            rt,
            workspace,
        })
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn rt_version(&self) -> Option<&str> {
        self.rt.version.as_deref()
    }

    /// Runtime capability snapshot (design 10.4).
    pub fn capabilities(&self) -> BackendCapabilities {
        rawtherapee_capabilities(self.rt.version.clone().unwrap_or_else(|| "unknown".into()))
    }

    // --- Commands ----------------------------------------------------------

    /// `inspect` — read metadata and build an ImageContext.
    pub fn inspect(&self, path: &Path) -> Result<Value> {
        let report = inspect::inspect(path).map_err(|e| anyhow!("inspect failed: {e}"))?;
        Ok(serde_json::to_value(&report)?)
    }

    /// `recipe create` — translate intent into a validated PhotoRecipe.
    pub async fn recipe_create(&self, path: &Path, prompt: &str, use_mock: bool) -> Result<Value> {
        let canonical = canonicalize_source(path)?;
        let report = inspect::inspect(&canonical)?;
        let capabilities = self.capabilities();

        let draft: RecipeDraft = if use_mock || self.config.llm.provider == "mock" {
            let mock = MockProvider::new(mock_draft());
            mock.complete_structured(&LlmRequest {
                system: prompts::system_prompt(&capabilities),
                user: prompts::user_prompt(
                    prompt,
                    &serde_json::to_value(&report.context)?,
                    &capabilities,
                ),
                schema: None,
                max_tokens: self.config.llm.max_tokens,
            })
            .await?
        } else {
            if !self.config.llm.is_usable() {
                let key_var = if self.config.llm.provider == "openrouter" {
                    "OPENROUTER_API_KEY"
                } else {
                    "OPENAI_API_KEY"
                };
                bail!(
                    "LLM is not configured for provider '{}'. Set {key_var} (or api_key in ~/.config/agbr/config.toml) and a model, or pass --mock.",
                    self.config.llm.provider
                );
            }
            let provider = OpenAIProvider::new(self.config.llm.clone());
            provider
                .complete_structured(&LlmRequest {
                    system: prompts::system_prompt(&capabilities),
                    user: prompts::user_prompt(
                        prompt,
                        &serde_json::to_value(&report.context)?,
                        &capabilities,
                    ),
                    schema: None,
                    max_tokens: self.config.llm.max_tokens,
                })
                .await?
        };

        let mut recipe =
            PhotoRecipe::from_draft(Source::new(canonical.display().to_string()), draft);

        let validation = validate_recipe(&recipe);
        if !validation.is_valid() {
            let errors: Vec<String> = validation.errors.iter().map(|e| e.to_string()).collect();
            bail!("generated recipe is invalid: {}", errors.join("; "));
        }

        let hash = recipe_hash(&recipe)?;
        let model = if self.config.llm.model.is_empty() {
            "mock".to_string()
        } else {
            self.config.llm.model.clone()
        };
        let provenance = Provenance {
            provider: Some(self.config.llm.provider.clone()),
            model: Some(model),
            recipe_hash: Some(hash.clone()),
            created_at: Some(now_rfc3339()),
            backend_version: self.rt.version.clone(),
            source_hash: Some(hash_file(&canonical)?),
            ..Default::default()
        };
        recipe.provenance = Some(provenance);

        let recipe_path = self.save_recipe(&recipe, &canonical)?;

        Ok(json!({
            "recipe_path": recipe_path,
            "recipe_hash": hash,
            "status": "validated",
            "backend": "rawtherapee",
            "warnings": validation.warnings,
            "recipe": recipe,
        }))
    }

    /// `recipe validate` — validate an existing recipe file.
    pub fn recipe_validate(&self, recipe_path: &Path) -> Result<Value> {
        let recipe = load_recipe(recipe_path)?;
        let validation = validate_recipe(&recipe);
        Ok(json!({
            "valid": validation.is_valid(),
            "errors": validation.errors,
            "warnings": validation.warnings,
        }))
    }

    /// `plan` — resolve operations and ordering against capabilities.
    pub fn plan(&self, recipe_path: &Path) -> Result<Value> {
        let recipe = load_recipe(recipe_path)?;
        let capabilities = self.capabilities();
        let plan = agbr_planner::plan(&recipe, &capabilities);
        Ok(serde_json::to_value(&plan)?)
    }

    /// `preview` — render a downscaled preview.
    pub async fn preview(&self, source: &Path, recipe_path: &Path) -> Result<Value> {
        let recipe = load_recipe(recipe_path)?;
        let canonical = canonicalize_source(source)?;
        let stem = Workspace::stem_for(&canonical);
        let out_file = self
            .workspace
            .previews_dir()
            .join(format!("{stem}-preview.jpg"));

        let result = self.render(
            &canonical,
            Some(&recipe),
            &out_file,
            OutputFormat::Jpeg { quality: 90 },
            Some(DEFAULT_PREVIEW_LONG_EDGE),
        )?;

        Ok(serde_json::to_value(result)?)
    }

    /// `apply` — render the full-resolution result.
    pub async fn apply(&self, source: &Path, recipe_path: &Path) -> Result<Value> {
        let recipe = load_recipe(recipe_path)?;
        let canonical = canonicalize_source(source)?;
        let stem = Workspace::stem_for(&canonical);
        let out_file = self.workspace.export_dir().join(format!("{stem}.jpg"));

        let result = self.render(
            &canonical,
            Some(&recipe),
            &out_file,
            OutputFormat::Jpeg { quality: 95 },
            None,
        )?;

        Ok(serde_json::to_value(result)?)
    }

    /// `export` — export with an explicit format (optionally with a recipe).
    pub async fn export(
        &self,
        source: &Path,
        format: OutputFormat,
        recipe_path: Option<&Path>,
    ) -> Result<Value> {
        let canonical = canonicalize_source(source)?;
        let stem = Workspace::stem_for(&canonical);
        let ext = format.extension();
        let out_file = self.workspace.export_dir().join(format!("{stem}.{ext}"));

        let recipe = match recipe_path {
            Some(p) => Some(load_recipe(p)?),
            None => None,
        };

        let result = self.render(&canonical, recipe.as_ref(), &out_file, format, None)?;

        Ok(serde_json::to_value(result)?)
    }

    /// `apply --batch` — render a set of photos with one shared recipe.
    pub fn apply_batch(
        &self,
        sources: &[PathBuf],
        recipe_path: &Path,
        jobs: usize,
    ) -> Result<Value> {
        let recipe = Arc::new(load_recipe(recipe_path)?);
        self.render_batch(
            sources,
            Some(recipe),
            OutputFormat::Jpeg { quality: 95 },
            None,
            jobs,
            &|stem| self.workspace.export_dir().join(format!("{stem}.jpg")),
        )
    }

    /// `preview --batch` — render downscaled previews for a set of photos.
    pub fn preview_batch(
        &self,
        sources: &[PathBuf],
        recipe_path: &Path,
        jobs: usize,
    ) -> Result<Value> {
        let recipe = Arc::new(load_recipe(recipe_path)?);
        self.render_batch(
            sources,
            Some(recipe),
            OutputFormat::Jpeg { quality: 90 },
            Some(DEFAULT_PREVIEW_LONG_EDGE),
            jobs,
            &|stem| {
                self.workspace
                    .previews_dir()
                    .join(format!("{stem}-preview.jpg"))
            },
        )
    }

    /// `export --batch` — export a set of photos with an explicit format.
    pub fn export_batch(
        &self,
        sources: &[PathBuf],
        format: OutputFormat,
        recipe_path: Option<&Path>,
        jobs: usize,
    ) -> Result<Value> {
        let recipe = match recipe_path {
            Some(p) => Some(Arc::new(load_recipe(p)?)),
            None => None,
        };
        let ext = format.extension();
        self.render_batch(sources, recipe, format, None, jobs, &|stem| {
            self.workspace.export_dir().join(format!("{stem}.{ext}"))
        })
    }

    /// `recipe create --batch` — generate a recipe per photo (shared prompt).
    pub async fn recipe_create_batch(
        &self,
        sources: &[PathBuf],
        prompt: &str,
        use_mock: bool,
    ) -> Result<Value> {
        let mut inputs = Vec::with_capacity(sources.len());
        let mut results = Vec::with_capacity(sources.len());
        for src in sources {
            let canonical = canonicalize_source(src)?;
            let stem = Workspace::stem_for(&canonical);
            inputs.push((canonical.display().to_string(), stem));
            results.push(self.recipe_create(src, prompt, use_mock).await);
        }
        Ok(batch::summarize(&inputs, results, 1))
    }

    // --- Internals ---------------------------------------------------------

    fn render_batch(
        &self,
        sources: &[PathBuf],
        recipe: Option<Arc<PhotoRecipe>>,
        format: OutputFormat,
        resize_long_edge: Option<u32>,
        jobs: usize,
        out_for: &dyn Fn(&str) -> PathBuf,
    ) -> Result<Value> {
        let mut canonicals = Vec::with_capacity(sources.len());
        let mut out_files = Vec::with_capacity(sources.len());
        let mut inputs = Vec::with_capacity(sources.len());

        for src in sources {
            let canonical = canonicalize_source(src)?;
            let stem = Workspace::stem_for(&canonical);
            out_files.push(out_for(&stem));
            inputs.push((canonical.display().to_string(), stem));
            canonicals.push(canonical);
        }

        let recipe_ref = recipe.as_deref();
        let results = batch::map_concurrent(canonicals.len(), jobs, |i| {
            self.render(
                &canonicals[i],
                recipe_ref,
                &out_files[i],
                format,
                resize_long_edge,
            )
        });

        Ok(batch::summarize(&inputs, results, jobs))
    }

    fn render(
        &self,
        source: &Path,
        recipe: Option<&PhotoRecipe>,
        out_file: &Path,
        format: OutputFormat,
        resize_long_edge: Option<u32>,
    ) -> Result<Value> {
        self.workspace.ensure_dirs()?;
        let stem = Workspace::stem_for(source);

        let report = inspect::inspect(source).map_err(|e| anyhow!("inspect failed: {e}"))?;

        // 1. Technical baseline.
        let base = generate_base_pp3(&BaseProfileInput {
            camera: report.technical.camera.clone(),
            lens: report.technical.lens.clone(),
            iso: report.technical.iso,
        });

        // 2. Creative look.
        let look = recipe
            .map(|r| generate_look_pp3(&r.global))
            .unwrap_or_default();

        // 3. Optional resize (preview).
        let resize = resize_long_edge.map(resize_pp3);

        let mut profiles: Vec<PathBuf> = Vec::new();

        let base_path = self
            .workspace
            .profiles_dir()
            .join(format!("{stem}.base.pp3"));
        std::fs::write(&base_path, &base.pp3)?;
        profiles.push(base_path);

        if recipe.is_some() {
            let look_path = self
                .workspace
                .profiles_dir()
                .join(format!("{stem}.look.pp3"));
            std::fs::write(&look_path, &look.pp3)?;
            profiles.push(look_path);
        }

        if let Some(resize) = &resize {
            let resize_path = self
                .workspace
                .profiles_dir()
                .join(format!("{stem}.resize.pp3"));
            std::fs::write(&resize_path, &resize.pp3)?;
            profiles.push(resize_path);
        }

        // Collect representability warnings.
        let mut warnings: Vec<String> = base.notes.clone();
        warnings.extend(base.unsupported.clone());
        warnings.extend(look.unsupported.clone());

        let command = build_command(&self.rt, source, out_file, &profiles, format, true);
        let output = run_rawtherapee(&command)?;

        let source_hash = hash_file(source)?;
        let output_hash = hash_file(out_file).map_err(|_| {
            anyhow!(
                "render produced no output at {}: {}",
                out_file.display(),
                output
            )
        })?;

        let entry = json!({
            "timestamp": now_rfc3339(),
            "command": command.display(),
            "source": source.display().to_string(),
            "source_hash": source_hash,
            "output": out_file.display().to_string(),
            "output_hash": output_hash,
            "backend_version": self.rt.version,
            "warnings": warnings,
        });
        self.workspace.append_log(&entry)?;

        Ok(json!({
            "output": out_file.display().to_string(),
            "format": format.extension(),
            "source_hash": source_hash,
            "output_hash": output_hash,
            "backend_version": self.rt.version,
            "command": command.display(),
            "warnings": warnings,
        }))
    }

    fn save_recipe(&self, recipe: &PhotoRecipe, source: &Path) -> Result<String> {
        self.workspace.ensure_dirs()?;
        let stem = Workspace::stem_for(source);
        let path = self.workspace.recipes_dir().join(format!("{stem}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(recipe)?)?;
        Ok(path.display().to_string())
    }
}

fn resize_pp3(long_edge: u32) -> agbr_rawtherapee::GeneratedProfile {
    use agbr_rawtherapee::GeneratedProfile;
    let mut pp3 = Pp3::new();
    pp3.set_bool("Resize", "Enabled", true);
    pp3.set_int("Resize", "DataSpecified", 4);
    pp3.set_int("Resize", "LongEdge", long_edge as i64);
    pp3.set("Resize", "AppliesTo", "Cropped area");
    pp3.set("Resize", "Method", "Lanczos");
    pp3.set_bool("Resize", "AllowUpscaling", false);
    GeneratedProfile {
        pp3: pp3.to_string(),
        notes: vec![],
        unsupported: vec![],
    }
}

fn run_rawtherapee(command: &agbr_rawtherapee::cli::Command) -> Result<String> {
    let output = std::process::Command::new(&command.program)
        .args(&command.args)
        .output()
        .context("failed to spawn rawtherapee-cli")?;

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        bail!("rawtherapee-cli failed: {}", stderr);
    }
    Ok(stderr)
}

fn load_recipe(path: &Path) -> Result<PhotoRecipe> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read recipe {}", path.display()))?;
    let recipe: PhotoRecipe = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse recipe {}", path.display()))?;
    Ok(recipe)
}

fn canonicalize_source(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        bail!("source file does not exist: {}", path.display());
    }
    path.canonicalize()
        .with_context(|| format!("failed to resolve path {}", path.display()))
}

fn hash_file(path: &Path) -> Result<String> {
    use sha2::Digest;
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hash_str(hasher.finalize().as_slice()))
}

fn now_rfc3339() -> String {
    // Format a Unix timestamp as a basic RFC 3339-like string without adding
    // the `chrono` dependency.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs = secs as i64;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // Civil-from-days algorithm (Howard Hinnant).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn mock_draft() -> Value {
    json!({
        "intent": { "vibe": "warm nostalgic summer", "intensity": 0.7, "notes": ["soft contrast", "natural skin"] },
        "global": {
            "exposure_ev": 0.1,
            "white_balance": { "temperature_k": 5700, "tint": 0.04 },
            "contrast": -0.12,
            "highlights": -0.2,
            "shadows": 0.12,
            "saturation": -0.08
        },
        "local": [],
        "constraints": { "preserve_skin": true, "no_crop": true, "no_generation": true, "preserve_original": true }
    })
}
