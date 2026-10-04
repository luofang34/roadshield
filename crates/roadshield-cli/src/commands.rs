//! Subcommand implementations.

mod batch;
mod bench;
mod pack;
mod render;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use clap::ArgMatches;
use roadshield::{DisplayContext, Engine, UnknownNetworkPolicy};

/// Runs the selected subcommand.
pub fn dispatch(m: &ArgMatches) -> Result<()> {
    match m.subcommand() {
        Some(("render", sub)) => render::run(sub),
        Some(("batch", sub)) => batch::run(sub),
        Some(("import", sub)) => pack::import(sub),
        Some(("subset", sub)) => pack::subset(sub),
        Some(("diff", sub)) => pack::diff(sub),
        Some(("inspect", sub)) => pack::inspect(sub),
        Some(("bench", sub)) => bench::run(sub),
        _ => anyhow::bail!("unknown subcommand"),
    }
}

fn path(m: &ArgMatches, name: &str) -> Result<PathBuf> {
    m.get_one::<PathBuf>(name)
        .cloned()
        .with_context(|| format!("--{name} is required"))
}

fn load_engine(m: &ArgMatches) -> Result<Engine> {
    let dir = path(m, "pack")?;
    roadshield_import::load_engine_blocking(&dir)
        .with_context(|| format!("loading pack {}", dir.display()))
}

fn context(m: &ArgMatches) -> Result<DisplayContext> {
    let mut ctx = match m.get_one::<PathBuf>("context") {
        Some(p) => {
            let bytes = roadshield_import::read_blocking(p)?;
            serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", p.display()))?
        }
        None => {
            // Raster output at DPR > 1 uses upstream's 2x layout grid, as
            // MapLibre would request 2x sprites.
            let raster = m.get_one::<String>("format").is_some_and(|f| f != "svg");
            let dpr = m.get_one::<f32>("dpr").copied().unwrap_or(1.0);
            DisplayContext {
                scale: m.get_one::<f64>("scale").copied().unwrap_or(1.0),
                language: m.get_one::<String>("lang").cloned(),
                pixel_grid: if raster && dpr > 1.0 { 2 } else { 1 },
                ..DisplayContext::default()
            }
        }
    };
    if m.get_flag("strict-network") {
        ctx.unknown_network = UnknownNetworkPolicy::Unsupported;
    }
    Ok(ctx)
}

fn write_out(out: Option<&Path>, bytes: &[u8]) -> Result<()> {
    match out {
        Some(p) => Ok(roadshield_import::write_blocking(p, bytes)?),
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(bytes).context("writing stdout")?;
            stdout.flush().context("flushing stdout")
        }
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_out(None, &bytes)
}
