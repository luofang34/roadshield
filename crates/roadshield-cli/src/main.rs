//! `roadshield` command-line tool: render shields, batch-render JSONL,
//! build/subset/diff/inspect packs, and benchmark the engine.

// anyhow is banned in libraries via clippy `disallowed-types`; this binary is
// the one place it belongs.
#![allow(clippy::disallowed_types)]

mod cli;
mod commands;

use std::process::ExitCode;

fn main() -> ExitCode {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    let matches = cli::command().get_matches();
    match commands::dispatch(&matches) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}
