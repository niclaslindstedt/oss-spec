//! oss-spec — bootstrap new open source repositories that conform to OSS_SPEC.md.
//!
//! This crate exposes both a binary (`oss-spec`) and a library so the bootstrap
//! engine can be reused programmatically (and tested without spawning a process).

pub mod agent_help;
pub mod ai;
pub mod bootstrap;
pub mod check;
pub mod cli;
pub mod embedded;
pub mod fix;
pub mod git;
pub mod interview;
pub mod manifest;
pub mod prompts;
pub mod render;

use anyhow::Result;

/// Top-level entry point. The binary calls this after parsing `Cli`.
pub async fn run(cli: cli::Cli) -> Result<()> {
    cli::dispatch(cli).await
}
