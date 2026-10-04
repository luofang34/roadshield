//! A road carrying several routes, the way a map labels it: one shield per
//! route, laid out in a row with a gap, centred on the label point. Shields
//! are rendered on the 2x layout grid (as for a high-density display) and
//! placed in logical pixels from their metadata.
//!
//! `cargo run -p roadshield --example concurrency > concurrency.svg`

#[path = "support/pack.rs"]
mod pack;

use std::io::Write as _;

use roadshield::{DisplayContext, Rendering, RouteDescriptor};

/// Gap between neighbouring shields, in logical pixels.
const GAP: f64 = 3.0;
/// Display scale relative to Americana's 1x pixels.
const SCALE: f64 = 2.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = pack::engine()?;
    let ctx = DisplayContext {
        scale: SCALE,
        pixel_grid: 2,
        ..DisplayContext::default()
    };
    // US 1, US 9 and NJ 27 share a road through New Jersey, with I-95 nearby.
    let roads: [&[(&str, &str)]; 2] = [
        &[("US:US", "1"), ("US:US", "9"), ("US:NJ", "27")],
        &[
            ("US:I", "95"),
            ("US:NJ:CR", "609"),
            ("US:US:Truck:Bypass", "1"),
        ],
    ];
    let mut body = String::new();
    let (width, mut y) = (360.0, 20.0);
    for routes in roads {
        let mut shields = Vec::new();
        for (network, r) in routes {
            if let Rendering::Symbol(s) =
                engine.render(&RouteDescriptor::new(*network, *r), &ctx)?
            {
                shields.push(s);
            }
        }
        let row: f64 = shields.iter().map(|s| s.width).sum::<f64>()
            + GAP * (shields.len().saturating_sub(1)) as f64;
        let tallest = shields.iter().map(|s| s.height).fold(0.0, f64::max);
        let mut x = (width - row) / 2.0;
        for s in &shields {
            // Bottom-align so banners stack above a common shield baseline.
            let top = y + tallest - s.height;
            body.push_str(&format!(
                "<g transform=\"translate({x} {top})\">{}</g>",
                s.svg
            ));
            x += s.width + GAP;
        }
        y += tallest + 24.0;
    }
    let doc = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{y}\" viewBox=\"0 0 {width} {y}\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#f6f1ea\"/>{body}</svg>\n"
    );
    std::io::stdout().lock().write_all(doc.as_bytes())?;
    Ok(())
}
