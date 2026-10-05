//! Regenerates the images in `docs/images` (or the directory given as the
//! first argument): a strip of shields from several networks, and a
//! comparison of text halo corner joins.
//!
//! `cargo run -p roadshield --example readme_images`

#[path = "support/pack.rs"]
mod pack;

use std::path::{Path, PathBuf};

use roadshield::{DisplayContext, Engine, Rendering, RouteDescriptor, TextHaloJoin};

const SCALE: f64 = 3.0;
const GAP: f64 = 12.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args_os().nth(1).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/images"),
        PathBuf::from,
    );
    let engine = pack::engine()?;
    let ctx = DisplayContext {
        scale: SCALE,
        ..DisplayContext::default()
    };
    let routes = [
        RouteDescriptor::new("US:I", "287"),
        RouteDescriptor::new("US:US", "22"),
        RouteDescriptor::new("US:NJ:CR", "609"),
        RouteDescriptor::new("US:NJ", "17"),
        RouteDescriptor::new("US:US:Truck:Bypass", "1"),
        RouteDescriptor::new("US:I:Business:Loop", "80"),
        RouteDescriptor {
            network: Some("US:PA:Turnpike".into()),
            ..RouteDescriptor::default()
        },
        RouteDescriptor {
            network: Some("US:KY:Parkway".into()),
            ..RouteDescriptor::default()
        }
        .with_name("Audubon Parkway"),
        RouteDescriptor::new("IN:NE", "5"),
        RouteDescriptor::new("CA:ON:primary", "401"),
        RouteDescriptor::new("CA:ON:primary", "QEW"),
        RouteDescriptor::new("GR:national", "Ε65"),
        RouteDescriptor::new("US:TX:FM", "1960"),
        RouteDescriptor::new("US:NJ", "Մ4"),
    ];
    let mut shields = Vec::new();
    for route in &routes {
        if let Rendering::Symbol(s) = engine.render(route, &ctx)? {
            shields.push(s);
        }
    }
    let height = shields.iter().map(|s| s.height).fold(0.0, f64::max);
    let mut x = 0.0;
    let mut body = String::new();
    for s in &shields {
        body.push_str(&format!(
            "<g transform=\"translate({x} {})\">{}</g>",
            height - s.height,
            s.svg
        ));
        x += s.width + GAP;
    }
    let width = x - GAP;
    let doc = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">{body}</svg>\n"
    );
    std::fs::create_dir_all(&out_dir)?;
    std::fs::write(out_dir.join("shields.svg"), doc)?;
    std::fs::write(out_dir.join("text-halo-joins.svg"), halo_joins(&engine)?)?;
    Ok(())
}

/// One row per text halo join, on a dark background so the light halo
/// shows: upstream's miter joins spike at the acute inner corners of A, V
/// and M; shields without text halos are identical in every row.
fn halo_joins(engine: &Engine) -> Result<String, Box<dyn std::error::Error>> {
    let rows = [
        ("americana (miter 10)", TextHaloJoin::AMERICANA),
        ("round (default)", TextHaloJoin::Round),
        ("bevel", TextHaloJoin::Bevel),
        ("miter 2", TextHaloJoin::Miter { limit: 2.0 }),
    ];
    let routes = [
        RouteDescriptor::new("BAB", "A 115"),
        RouteDescriptor::new("BAB", "A100"),
        RouteDescriptor::new("BAB", "A111"),
        RouteDescriptor::new("BAB", "M 5"),
        RouteDescriptor::new("BAB", "V 7"),
        RouteDescriptor::new("US:US:Alternate", "1"),
        RouteDescriptor::new("US:NJ:CR", "609"),
    ];
    let (label_w, row_h) = (190.0, 100.0);
    let mut body = String::new();
    let mut width: f64 = 0.0;
    for (i, (label, join)) in rows.iter().enumerate() {
        let y = i as f64 * row_h + 10.0;
        body.push_str(&format!(
            "<text x=\"10\" y=\"{}\" fill=\"#f6f1ea\" font-family=\"sans-serif\" font-size=\"16\">{label}</text>",
            y + row_h / 2.0
        ));
        let ctx = DisplayContext {
            scale: SCALE,
            text_halo_join: *join,
            ..DisplayContext::default()
        };
        let mut x = label_w;
        for route in &routes {
            if let Rendering::Symbol(s) = engine.render(route, &ctx)? {
                body.push_str(&format!(
                    "<g transform=\"translate({x} {})\">{}</g>",
                    y + row_h - 10.0 - s.height,
                    s.svg
                ));
                x += s.width + GAP;
            }
        }
        width = width.max(x);
    }
    let height = rows.len() as f64 * row_h + 20.0;
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#283c5a\"/>{body}</svg>\n"
    ))
}
