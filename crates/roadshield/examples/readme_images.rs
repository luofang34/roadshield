//! Regenerates the images in `docs/images` (or the directory given as the
//! first argument): a strip of shields from several networks.
//!
//! `cargo run -p roadshield --example readme_images`

#[path = "support/pack.rs"]
mod pack;

use std::path::{Path, PathBuf};

use roadshield::{DisplayContext, Rendering, RouteDescriptor};

const SCALE: f64 = 3.0;
const GAP: f64 = 12.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/images"));
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
    Ok(())
}
