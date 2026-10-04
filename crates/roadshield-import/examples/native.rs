//! Native usage: load a pack from disk, render a few shields, rasterise one.
//!
//! `cargo run -p roadshield-import --example native -- packs/americana`

use std::path::PathBuf;

use roadshield::{DisplayContext, Rendering, RouteDescriptor};
use roadshield_import::{ImportError, load_engine_blocking};

fn main() -> Result<(), ImportError> {
    tracing_subscriber::fmt().init();
    let dir = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("packs/americana"), PathBuf::from);
    let engine = load_engine_blocking(&dir)?;
    let ctx = DisplayContext::default();
    for route in [
        RouteDescriptor::new("US:I", "287"),
        RouteDescriptor::new("US:NJ:CR", "609"),
        RouteDescriptor::new("US:US:Truck:Bypass", "1"),
        RouteDescriptor::new("US:I", "12345678"),
    ] {
        match engine.render(&route, &ctx) {
            Ok(Rendering::Symbol(s)) => tracing::info!(
                "{route:?}: rule {} size {}x{} key {}",
                s.rule.rule_key,
                s.width,
                s.height,
                s.semantic_key
            ),
            Ok(Rendering::NoShield { reason, .. }) => {
                tracing::info!("{route:?}: no shield ({reason:?})")
            }
            Err(e) => tracing::error!("{route:?}: {e}"),
        }
    }
    Ok(())
}
