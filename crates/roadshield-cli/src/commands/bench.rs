//! `roadshield bench`: cold/hot latency and throughput over every network.

use std::time::Instant;

use anyhow::Result;
use clap::ArgMatches;
use roadshield::{DisplayContext, RouteDescriptor};
use roadshield_raster::{RasterOptions, rasterize};
use serde_json::json;

use super::{load_engine, path, print_json};

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted.get(idx).copied().unwrap_or(0.0)
}

/// Runs the benchmark and prints a JSON report.
pub fn run(m: &ArgMatches) -> Result<()> {
    let load_start = Instant::now();
    let engine = load_engine(m)?;
    let load_ms = load_start.elapsed().as_secs_f64() * 1e3;
    let iterations = m
        .get_one::<usize>("iterations")
        .copied()
        .unwrap_or(2000)
        .max(1);
    let ctx = DisplayContext::default();
    let networks: Vec<String> = engine.networks().map(str::to_owned).collect();
    let refs = ["1", "22", "287", "609", "1234", "66A"];
    let routes: Vec<RouteDescriptor> = (0..iterations)
        .map(|i| {
            let n = networks
                .get(i % networks.len().max(1))
                .cloned()
                .unwrap_or_default();
            RouteDescriptor::new(n, refs.get(i % refs.len()).copied().unwrap_or("1"))
        })
        .collect();

    let cold_start = Instant::now();
    let first = routes.first().map(|r| engine.render(r, &ctx));
    let cold_ms = cold_start.elapsed().as_secs_f64() * 1e3;
    let mut lat = Vec::with_capacity(routes.len());
    let (mut symbols, mut svg_bytes) = (0usize, 0usize);
    let batch_start = Instant::now();
    for r in &routes {
        let t = Instant::now();
        if let Ok(roadshield::Rendering::Symbol(s)) = engine.render(r, &ctx) {
            symbols += 1;
            svg_bytes += s.svg.len();
        }
        lat.push(t.elapsed().as_secs_f64() * 1e6);
    }
    let batch_s = batch_start.elapsed().as_secs_f64();
    lat.sort_by(f64::total_cmp);
    let raster_start = Instant::now();
    let mut rasters = 0usize;
    for r in routes.iter().take(200) {
        let opts = RasterOptions {
            pixel_ratio: 2.0,
            ..RasterOptions::default()
        };
        if let Ok(roadshield::Rendering::Symbol(s)) = engine.render(r, &ctx)
            && rasterize(&s, &opts).is_ok()
        {
            rasters += 1;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let raster_us = raster_start.elapsed().as_secs_f64() * 1e6 / rasters.max(1) as f64;
    #[allow(clippy::cast_precision_loss)]
    let per_s = routes.len() as f64 / batch_s.max(1e-9);
    print_json(&json!({
        "pack": path(m, "pack")?.display().to_string(),
        "pack_load_ms": load_ms,
        "first_render_ms": cold_ms,
        "first_render_ok": first.is_some_and(|r| r.is_ok()),
        "renders": routes.len(),
        "symbols": symbols,
        "renders_per_s": per_s,
        "latency_us": {"p50": percentile(&lat, 0.5), "p90": percentile(&lat, 0.9), "p99": percentile(&lat, 0.99), "max": lat.last()},
        "mean_svg_bytes": svg_bytes / symbols.max(1),
        "render_plus_raster_2x_us": raster_us,
    }))
}
