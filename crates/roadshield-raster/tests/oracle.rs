//! Conformance against captures of the upstream TypeScript renderer
//! (see `fixtures/americana-oracle-1x.json` for provenance).

use std::path::Path;

use roadshield::{DisplayContext, Engine, Rendering, RouteDescriptor};
use roadshield_raster::{AlphaMode, RasterOptions, rasterize};
use serde::Deserialize;

#[derive(Deserialize)]
struct Metric {
    t: String,
    px: f64,
    width: f64,
    l: f64,
    r: f64,
    a: f64,
    d: f64,
}

#[derive(Deserialize)]
struct Case {
    network: String,
    #[serde(rename = "ref")]
    ref_: String,
    name: String,
    width: u32,
    height: u32,
    png: String,
}

#[derive(Deserialize)]
struct Fixture {
    metrics: Vec<Metric>,
    cases: Vec<Case>,
}

#[cfg(test)]
fn fixture() -> Fixture {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/americana-oracle-1x.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

#[cfg(test)]
fn engine() -> Engine {
    roadshield_import::load_engine_blocking(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana"),
    )
    .unwrap()
}

#[cfg(test)]
fn b64(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        _ => 63,
    };
    let bytes: Vec<u8> = s.bytes().filter(|&c| c != b'=').collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &c)| n | u32::from(val(c)) << (18 - 6 * i));
        for i in 0..chunk.len().saturating_sub(1) {
            out.push((n >> (16 - 8 * i)) as u8);
        }
    }
    out
}

#[cfg(test)]
fn route(c: &Case) -> RouteDescriptor {
    let opt = |s: &str| (!s.is_empty()).then(|| s.to_owned());
    RouteDescriptor {
        network: opt(&c.network),
        ref_: opt(&c.ref_),
        name: opt(&c.name),
        ..RouteDescriptor::default()
    }
}

#[test]
fn text_metrics_match_chromium_measure_text() {
    let e = engine();
    let ctx = DisplayContext::default();
    let mut report = Vec::new();
    for m in fixture().metrics {
        let ours = e.measure_text(&m.t, m.px, &ctx).unwrap();
        let pairs = [
            ("width", ours.width, m.width),
            ("left", ours.actual_bounding_box_left, m.l),
            ("right", ours.actual_bounding_box_right, m.r),
            ("ascent", ours.actual_bounding_box_ascent, m.a),
            ("descent", ours.actual_bounding_box_descent, m.d),
        ];
        for (what, a, b) in pairs {
            if (a - b).abs() > 0.0005 {
                report.push(format!(
                    "{:?}@{} {what}: ours {a:.4} chromium {b:.4}",
                    m.t, m.px
                ));
            }
        }
    }
    assert!(report.is_empty(), "\n{}", report.join("\n"));
}

#[test]
fn sizes_and_pixels_match_the_upstream_renderer() {
    let e = engine();
    let mut size_report = Vec::new();
    let mut pixel_report = Vec::new();
    for c in fixture().cases {
        let label = format!("{} {:?} {:?}", c.network, c.ref_, c.name);
        let Rendering::Symbol(s) = e.render(&route(&c), &DisplayContext::default()).unwrap() else {
            size_report.push(format!("{label}: no shield"));
            continue;
        };
        if (s.width, s.height) != (f64::from(c.width), f64::from(c.height)) {
            size_report.push(format!(
                "{label}: ours {}x{} upstream {}x{}",
                s.width, s.height, c.width, c.height
            ));
            continue;
        }
        let ours = rasterize(
            &s,
            &RasterOptions {
                pixel_ratio: 1.0,
                alpha: AlphaMode::Premultiplied,
                ..RasterOptions::default()
            },
        )
        .unwrap();
        let theirs = tiny_skia::Pixmap::decode_png(&b64(&c.png)).unwrap();
        let (mut sum, mut big) = (0u64, 0usize);
        for (a, b) in ours.data.chunks(4).zip(theirs.data().chunks(4)) {
            let d = a
                .iter()
                .zip(b)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            sum += u64::from(d);
            if d > 128 {
                big += 1;
            }
        }
        let n = (ours.width * ours.height) as f64;
        let mean = sum as f64 / n;
        let frac = big as f64 / n;
        // Sizes and text metrics are asserted exactly above; pixels differ
        // only by anti-aliasing: Chromium rasterises glyphs with hinting and
        // gamma-adjusted coverage, resvg without. Observed worst case on this
        // fixture is mean 21 / 1.6%; the bounds catch any geometric error,
        // which moves whole edges.
        if mean > 24.0 || frac > 0.025 {
            pixel_report.push(format!(
                "{label}: mean |Δ| {mean:.1}, {:.1}% pixels Δ>128",
                frac * 100.0
            ));
        }
    }
    assert!(
        size_report.is_empty() && pixel_report.is_empty(),
        "\nsizes:\n{}\npixels:\n{}",
        size_report.join("\n"),
        pixel_report.join("\n")
    );
}
