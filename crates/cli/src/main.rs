use std::path::PathBuf;

use clap::{Parser, Subcommand};

use agbr::{AppConfig, Engine};

#[derive(Parser)]
#[command(
    name = "agbr",
    version,
    about = "Terminal-native AI RAW processing engine. Non-destructive .pp3 sidecar generation, automated lens/CA corrections, and cloud-segmented regional compositing."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read EXIF metadata and produce an image context.
    Inspect {
        /// Path to the source RAW file.
        photo: PathBuf,
    },

    /// Report the installed backend's runtime capabilities.
    Capabilities,

    /// Print the resolved configuration.
    Config,

    /// Generate, validate, and manage recipes.
    Recipe {
        #[command(subcommand)]
        cmd: RecipeCmd,
    },

    /// Plan a recipe against backend capabilities without executing.
    Plan {
        /// Path to a recipe JSON file.
        #[arg(long)]
        recipe: PathBuf,
    },

    /// Render a downscaled preview.
    Preview {
        photo: PathBuf,
        #[arg(long)]
        recipe: PathBuf,
    },

    /// Apply a recipe and render the full-resolution result.
    Apply {
        photo: PathBuf,
        #[arg(long)]
        recipe: PathBuf,
    },

    /// Export with an explicit format/quality (optionally with a recipe).
    Export {
        photo: PathBuf,
        /// Output format: jpg, tif, or png.
        #[arg(long, default_value = "jpg")]
        format: String,
        /// JPEG quality (1-100).
        #[arg(long)]
        quality: Option<u8>,
        /// Optional recipe to apply during export.
        #[arg(long)]
        recipe: Option<PathBuf>,
    },

    /// MCP interface.
    Mcp {
        #[command(subcommand)]
        cmd: McpCmd,
    },
}

#[derive(Subcommand)]
enum RecipeCmd {
    /// Generate a PhotoRecipe from natural-language intent.
    Create {
        photo: PathBuf,
        #[arg(long)]
        prompt: String,
        /// Use the deterministic mock provider (offline).
        #[arg(long)]
        mock: bool,
    },
    /// Validate an existing recipe JSON file.
    Validate { recipe: PathBuf },
}

#[derive(Subcommand)]
enum McpCmd {
    /// Start the MCP server over stdio.
    Serve,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("config error: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = run(cli, config).await {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli, config: AppConfig) -> anyhow::Result<()> {
    match cli.command {
        Commands::Inspect { photo } => {
            let engine = Engine::new(config)?;
            print_json(engine.inspect(&photo)?);
        }
        Commands::Capabilities => {
            let engine = Engine::new(config)?;
            print_json(serde_json::to_value(engine.capabilities())?);
        }
        Commands::Config => {
            println!("{}", agbr::config::default_config_toml());
        }
        Commands::Recipe { cmd } => match cmd {
            RecipeCmd::Create {
                photo,
                prompt,
                mock,
            } => {
                let engine = Engine::new(config)?;
                print_json(engine.recipe_create(&photo, &prompt, mock).await?);
            }
            RecipeCmd::Validate { recipe } => {
                let engine = Engine::new(config)?;
                print_json(engine.recipe_validate(&recipe)?);
            }
        },
        Commands::Plan { recipe } => {
            let engine = Engine::new(config)?;
            print_json(engine.plan(&recipe)?);
        }
        Commands::Preview { photo, recipe } => {
            let engine = Engine::new(config)?;
            print_json(engine.preview(&photo, &recipe).await?);
        }
        Commands::Apply { photo, recipe } => {
            let engine = Engine::new(config)?;
            print_json(engine.apply(&photo, &recipe).await?);
        }
        Commands::Export {
            photo,
            format,
            quality,
            recipe,
        } => {
            let engine = Engine::new(config)?;
            let format = parse_format(&format, quality);
            print_json(engine.export(&photo, format, recipe.as_deref()).await?);
        }
        Commands::Mcp { cmd } => match cmd {
            McpCmd::Serve => {
                let engine = Engine::new(config)?;
                let engine = std::sync::Arc::new(engine);
                agbr::mcp::serve(engine).await?;
            }
        },
    }
    Ok(())
}

fn parse_format(format: &str, quality: Option<u8>) -> agbr_rawtherapee::cli::OutputFormat {
    match format.to_ascii_lowercase().as_str() {
        "tif" | "tiff" => agbr_rawtherapee::cli::OutputFormat::Tiff,
        "png" => agbr_rawtherapee::cli::OutputFormat::Png,
        _ => agbr_rawtherapee::cli::OutputFormat::Jpeg {
            quality: quality.unwrap_or(95).clamp(1, 100),
        },
    }
}

fn print_json(value: serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|e| e.to_string())
    );
}
