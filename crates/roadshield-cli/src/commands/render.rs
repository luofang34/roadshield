//! `roadshield render`.

use anyhow::{Context as _, Result, bail};
use clap::ArgMatches;
use roadshield::{Rendering, RouteDescriptor};
use roadshield_raster::{AlphaMode, RasterOptions, encode_png, rasterize};

use super::{context, load_engine, print_json, write_out};

/// Renders one shield to SVG/PNG or prints its metadata.
pub fn run(m: &ArgMatches) -> Result<()> {
    let engine = load_engine(m)?;
    let ctx = context(m)?;
    let route = RouteDescriptor {
        network: m.get_one::<String>("network").cloned(),
        ref_: m.get_one::<String>("ref").cloned(),
        name: m.get_one::<String>("name").cloned(),
        source: Some("cli".into()),
        ..RouteDescriptor::default()
    };
    let rendering = engine.render(&route, &ctx)?;
    if m.get_flag("json") {
        return print_json(&rendering);
    }
    let Rendering::Symbol(symbol) = rendering else {
        if let Rendering::NoShield { reason, .. } = rendering {
            bail!("no shield for {route:?}: {reason:?}");
        }
        bail!("no shield for {route:?}");
    };
    let out = m
        .get_one::<std::path::PathBuf>("out")
        .map(std::path::PathBuf::as_path);
    match m.get_one::<String>("format").map(String::as_str) {
        Some("png") => {
            let opts = RasterOptions {
                pixel_ratio: m.get_one::<f32>("dpr").copied().unwrap_or(2.0),
                alpha: AlphaMode::Straight,
                ..RasterOptions::default()
            };
            let png = encode_png(&rasterize(&symbol, &opts)?)?;
            let out = out.context("--out is required for PNG output")?;
            write_out(Some(out), &png)
        }
        _ => write_out(out, symbol.svg.as_bytes()),
    }
}
