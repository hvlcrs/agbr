//! `agbr-rawtherapee` — RawTherapee adapter.
//!
//! Owns everything PP3-related: key names, sections, profile layering, CLI
//! construction, and capability reporting. The LLM never sees PP3 syntax.

mod adapter;
pub mod capabilities;
pub mod cli;
pub mod pp3;

pub use adapter::{generate_base_pp3, generate_look_pp3, BaseProfileInput, GeneratedProfile};
pub use capabilities::rawtherapee_capabilities;
pub use cli::{discover_binary, Cli, CliError, Command, OutputFormat};
pub use pp3::Pp3;
