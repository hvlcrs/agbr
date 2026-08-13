//! The `agbr` control plane library.
//!
//! The [`engine::Engine`] is the deterministic photo-control engine; the CLI
//! binary and the MCP server are thin interfaces over it.

pub mod config;
pub mod engine;
pub mod mcp;
pub mod prompts;
pub mod workspace;

pub use config::AppConfig;
pub use engine::Engine;
