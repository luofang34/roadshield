//! Text halo corner joins, through the real path: `Engine::render` then the
//! raster adapter, at several scales and device pixel ratios.
//!
//! A halo pixel's distance to the nearest glyph pixel is at most the halo
//! half-width (plus anti-aliasing) for round and bevel joins. Upstream's
//! miter joins (limit 10) reach several times further at the acute inner
//! corners of A, V and M; those are the spikes.

use std::path::Path;

use roadshield::{DisplayContext, Engine, Rendering, RouteDescriptor, ShieldSymbol, TextHaloJoin};
use roadshield_raster::{AlphaMode, RasterImage, RasterOptions, rasterize};

#[cfg(test)]
fn engine() -> Engine {
    roadshield_import::load_engine_blocking(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana"),
    )
    .unwrap()
}

#[cfg(test)]
fn render(
    e: &Engine,
    route: &RouteDescriptor,
    scale: f64,
    dpr: f32,
    join: TextHaloJoin,
) -> (ShieldSymbol, RasterImage) {
    let ctx = DisplayContext {
        scale,
        pixel_grid: if dpr > 1.0 { 2 } else { 1 },
        text_halo_join: join,
        ..DisplayContext::default()
    };
    let Rendering::Symbol(s) = e.render(route, &ctx).unwrap() else {
        panic!("{route:?} drew no shield");
    };
    let opts = RasterOptions {
        pixel_ratio: dpr,
        alpha: AlphaMode::Straight,
        ..RasterOptions::default()
    };
    let img = rasterize(&s, &opts).unwrap();
    (*s, img)
}

/// Furthest distance, in device pixels, from an opaque light halo pixel to
/// the nearest dark glyph pixel, within rows `0..rows`.
#[cfg(test)]
fn halo_reach(img: &RasterImage, rows: u32) -> f64 {
    let px = |x: u32, y: u32| {
        let i = ((y * img.width + x) * 4) as usize;
        &img.data[i..i + 4]
    };
    let mut glyph = Vec::new();
    let mut halo = Vec::new();
    for y in 0..rows.min(img.height) {
        for x in 0..img.width {
            let p = px(x, y);
            let max = p[0].max(p[1]).max(p[2]);
            let min = p[0].min(p[1]).min(p[2]);
            // Glyph cores are dark; anti-aliased thin stems stay well below
            // the light halo, which is near-white.
            if p[3] > 200 && max < 150 {
                glyph.push((f64::from(x), f64::from(y)));
            } else if p[3] > 160 && min > 215 {
                halo.push((f64::from(x), f64::from(y)));
            }
        }
    }
    assert!(
        !glyph.is_empty() && !halo.is_empty(),
        "no text or halo found"
    );
    halo.iter()
        .map(|&(hx, hy)| {
            glyph
                .iter()
                .map(|&(gx, gy)| (hx - gx).hypot(hy - gy))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

/// Opaque pixels on the image border (clipped content touches it).
#[cfg(test)]
fn border_pixels(img: &RasterImage) -> usize {
    let (w, h) = (img.width, img.height);
    (0..w)
        .flat_map(|x| [(x, 0), (x, h - 1)])
        .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]))
        .filter(|&(x, y)| img.data[((y * w + x) * 4 + 3) as usize] > 160)
        .count()
}

const SPIKY_REFS: &[&str] = &["A 115", "A100", "A111", "M 5", "V 7"];
const SCALES: &[f64] = &[1.0, 2.0];
const DPRS: &[f32] = &[1.0, 2.0, 3.0];

#[test]
fn fallback_text_halo_has_no_spikes_with_round_or_bevel_joins() {
    let e = engine();
    for r in SPIKY_REFS {
        let route = RouteDescriptor::new("XX:unknown", *r);
        for &scale in SCALES {
            for &dpr in DPRS {
                // The halo is 2 layout px wide: half-width scale * dpr device px.
                let half = scale * f64::from(dpr);
                let limit = half + 1.5;
                for join in [TextHaloJoin::Round, TextHaloJoin::Bevel] {
                    let (s, img) = render(&e, &route, scale, dpr, join);
                    assert_eq!(
                        s.rule.rule_key, "default",
                        "an unknown network uses the generic text rule"
                    );
                    let reach = halo_reach(&img, img.height);
                    assert!(
                        reach <= limit,
                        "{r} scale {scale} dpr {dpr} {join:?}: halo reaches {reach:.1}, limit {limit:.1}"
                    );
                }
                if r.starts_with('A') || r.starts_with('V') {
                    let (_, img) = render(&e, &route, scale, dpr, TextHaloJoin::AMERICANA);
                    let reach = halo_reach(&img, img.height);
                    assert!(
                        reach > 2.5 * half,
                        "{r} scale {scale} dpr {dpr}: expected an upstream miter spike, reach {reach:.1}"
                    );
                }
            }
        }
    }
}

#[test]
fn round_joins_add_no_clipping() {
    let e = engine();
    for r in SPIKY_REFS {
        for &dpr in DPRS {
            let route = RouteDescriptor::new("XX:unknown", *r);
            let (round_s, round) = render(&e, &route, 1.0, dpr, TextHaloJoin::Round);
            let (miter_s, miter) = render(&e, &route, 1.0, dpr, TextHaloJoin::AMERICANA);
            assert_eq!(
                (round_s.width, round_s.height),
                (miter_s.width, miter_s.height)
            );
            assert_eq!(round_s.anchor, miter_s.anchor);
            assert!(
                border_pixels(&round) <= border_pixels(&miter),
                "{r} dpr {dpr}"
            );
        }
    }
}

#[test]
fn banner_halos_on_a_shield_lose_spikes_but_the_blank_is_untouched() {
    let e = engine();
    let route = RouteDescriptor::new("US:US:Alternate", "1");
    for &scale in SCALES {
        for &dpr in DPRS {
            let (s, img) = render(&e, &route, scale, dpr, TextHaloJoin::Round);
            let (m, _) = render(&e, &route, scale, dpr, TextHaloJoin::AMERICANA);
            // Rows above the shield body hold the "ALT" banner and its halo.
            let rows = (s.shield_box.y * f64::from(dpr)).floor() as u32;
            let limit = scale * f64::from(dpr) + 1.5;
            let reach = halo_reach(&img, rows);
            assert!(
                reach <= limit,
                "scale {scale} dpr {dpr}: banner halo reaches {reach:.1}"
            );
            let blank = |svg: &str| {
                let start = svg.find("<svg width").unwrap();
                svg[start..svg[start..].find("</svg>").unwrap() + start].to_owned()
            };
            assert_eq!(
                blank(&s.svg),
                blank(&m.svg),
                "blank artwork must not depend on the halo join"
            );
            assert_eq!((s.width, s.height, s.anchor), (m.width, m.height, m.anchor));
        }
    }
}

#[test]
fn shapes_without_text_halos_render_identically_under_every_join() {
    let e = engine();
    for (network, r) in [("US:NJ:CR", "609"), ("US:NJ", "17"), ("US:I", "287")] {
        let route = RouteDescriptor::new(network, r);
        let (a, _) = render(&e, &route, 2.0, 2.0, TextHaloJoin::Round);
        let (b, _) = render(&e, &route, 2.0, 2.0, TextHaloJoin::AMERICANA);
        assert_eq!(a.svg, b.svg, "{network} {r}");
        assert!(!a.svg.contains("stroke-linejoin=\"round\"") || network != "US:NJ:CR");
    }
    // Shape outlines keep canvas miter joins.
    let (cr, _) = render(
        &e,
        &RouteDescriptor::new("US:NJ:CR", "609"),
        1.0,
        1.0,
        TextHaloJoin::Round,
    );
    assert!(
        cr.svg
            .contains("stroke-linejoin=\"miter\" stroke-miterlimit=\"10\"")
    );
}

#[test]
fn halo_join_is_part_of_the_cache_key() {
    let e = engine();
    let route = RouteDescriptor::new("XX:unknown", "A 115");
    let key = |join| {
        e.semantic_key(
            &route,
            &DisplayContext {
                text_halo_join: join,
                ..DisplayContext::default()
            },
        )
    };
    let keys = [
        key(TextHaloJoin::Round),
        key(TextHaloJoin::Bevel),
        key(TextHaloJoin::AMERICANA),
        key(TextHaloJoin::Miter { limit: 2.0 }),
    ];
    for i in 0..keys.len() {
        for j in i + 1..keys.len() {
            assert_ne!(keys[i], keys[j]);
        }
    }
    let bad = DisplayContext {
        text_halo_join: TextHaloJoin::Miter { limit: 20.0 },
        ..DisplayContext::default()
    };
    assert!(
        e.render(&route, &bad).is_err(),
        "limits above upstream's are rejected"
    );
}
