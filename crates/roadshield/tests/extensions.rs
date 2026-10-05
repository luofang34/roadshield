//! Networks the pack adds beyond Americana (`packs/americana.extensions.json`).
//! There is no upstream oracle for these until Americana adopts them, so
//! they are checked structurally here; `oracle/sweep.mjs --include-extensions`
//! compares them against an upstream build with the contribution draft.

use std::collections::HashMap;
use std::path::Path;

use roadshield::{DisplayContext, Engine, Rendering, ResourcePack, RouteDescriptor, RuleOrigin};

#[cfg(test)]
fn engine() -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana");
    let mut files = HashMap::new();
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                dirs.push(p);
            } else {
                let key = p
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(key, std::fs::read(&p).unwrap());
            }
        }
    }
    Engine::new(ResourcePack::load(&files).unwrap()).unwrap()
}

/// (network, refs, shape) for every extension network.
const CASES: &[(&str, &[&str], &str)] = &[
    (
        "BAB",
        &["A 1", "A 7", "A 48", "A 100", "A 115", "A 661"],
        "hexagonHorizontal",
    ),
    ("AH", &["AH1", "AH6", "AH26", "AH150"], "roundedRectangle"),
];

#[test]
fn extension_networks_draw_their_shape_within_upstream_guidelines() {
    let e = engine();
    for (network, refs, shape) in CASES {
        assert_eq!(e.rule_origin(network), Some(RuleOrigin::Extension));
        for r in *refs {
            let route = RouteDescriptor::new(*network, *r);
            let Rendering::Symbol(s) = e.render(&route, &DisplayContext::default()).unwrap() else {
                panic!("{network} {r} drew no shield");
            };
            assert_eq!(s.rule.rule_key, *network);
            assert_eq!(s.rule.origin, RuleOrigin::Extension);
            assert_eq!(s.rule.shape.as_deref(), Some(*shape));
            assert!(s.warnings.is_empty(), "{network} {r}: {:?}", s.warnings);
            // Americana's shield guide: 20 px tall, text 8-14 px, width
            // clamped to the generic maximum.
            assert_eq!(s.height, 20.0);
            assert!(
                s.width >= 20.0 && s.width <= 34.0,
                "{network} {r}: width {}",
                s.width
            );
            let t = s.text.as_ref().unwrap();
            assert!(
                t.font_px >= 8.0 && t.font_px <= 14.0,
                "{network} {r}: font {}",
                t.font_px
            );
            assert!(
                t.ink.x >= 0.0 && t.ink.x + t.ink.width <= s.width,
                "{network} {r}: ink {:?}",
                t.ink
            );
            usvg::Tree::from_str(&s.svg, &usvg::Options::default()).unwrap();
        }
    }
}

#[test]
fn upstream_networks_report_upstream_origin() {
    let e = engine();
    for network in ["e-road", "DE:national", "CN:JS", "US:I"] {
        assert_eq!(
            e.rule_origin(network),
            Some(RuleOrigin::Upstream),
            "{network}"
        );
    }
    let generic = e
        .render(
            &RouteDescriptor::new("XX:none", "7"),
            &DisplayContext::default(),
        )
        .unwrap();
    assert_eq!(generic.symbol().unwrap().rule.origin, RuleOrigin::Upstream);
    assert_eq!(e.rule_origin("XX:none"), None);
}
