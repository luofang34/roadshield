//! Acceptance cases from the requirements, against the generated Americana
//! pack, through the public API only.

use std::collections::HashMap;
use std::path::Path;

use roadshield::{
    Accessibility, AppliedOverride, DisplayContext, Engine, MissingGlyphPolicy, NoShieldReason,
    Rendering, ResourcePack, RouteDescriptor, ShieldError, ShieldSymbol, UnknownNetworkPolicy,
    Warning,
};

#[cfg(test)]
fn load(dir: &Path, root: &Path, out: &mut HashMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            load(&p, root, out);
        } else {
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, std::fs::read(&p).unwrap());
        }
    }
}

#[cfg(test)]
fn engine() -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana");
    let mut files = HashMap::new();
    load(&root, &root, &mut files);
    Engine::new(ResourcePack::load(&files).unwrap()).unwrap()
}

#[cfg(test)]
fn render(e: &Engine, route: RouteDescriptor) -> Box<ShieldSymbol> {
    match e.render(&route, &DisplayContext::default()).unwrap() {
        Rendering::Symbol(s) => s,
        other => panic!("{route:?} gave {other:?}"),
    }
}

#[cfg(test)]
fn assert_svg_parses(s: &ShieldSymbol) {
    let tree = usvg::Tree::from_str(&s.svg, &usvg::Options::default()).unwrap();
    assert!((f64::from(tree.size().width()) - s.width).abs() < 1e-3);
}

#[cfg(test)]
fn inside(s: &ShieldSymbol) {
    let t = s.text.as_ref().unwrap();
    let b = s.shield_box;
    assert!(
        t.ink.x >= b.x - 0.5 && t.ink.x + t.ink.width <= b.x + b.width + 0.5,
        "{t:?} in {b:?}"
    );
    assert!(
        t.ink.y >= b.y - 0.5 && t.ink.y + t.ink.height <= b.y + b.height + 0.5,
        "{t:?} in {b:?}"
    );
}

#[test]
fn interstate_287_uses_the_three_digit_blank() {
    let s = render(&engine(), RouteDescriptor::new("US:I", "287"));
    assert_eq!(s.rule.rule_key, "US:I");
    assert_eq!(s.rule.blank.as_deref(), Some("shield_us_interstate_3"));
    assert_eq!((s.width, s.height), (25.0, 20.0));
    assert_eq!(s.text.as_ref().unwrap().text, "287");
    let px = s.text.as_ref().unwrap().font_px;
    assert!(px > 6.0 && px <= 14.0, "{px}");
    assert!(s.svg.contains("fill=\"#ffffff\""), "white text");
    inside(&s);
    assert_svg_parses(&s);
    assert!(s.warnings.is_empty());
}

#[test]
fn us_route_22_uses_the_two_digit_badge() {
    let s = render(&engine(), RouteDescriptor::new("US:US", "22"));
    assert_eq!(s.rule.blank.as_deref(), Some("shield_badge_2"));
    inside(&s);
}

#[test]
fn county_route_609_is_a_drawn_pentagon() {
    let s = render(&engine(), RouteDescriptor::new("US:NJ:CR", "609"));
    assert_eq!(s.rule.shape.as_deref(), Some("pentagon"));
    assert!(s.rule.blank.is_none());
    assert!(s.width > 20.0 && s.width < 40.0, "{}", s.width);
    assert!(s.svg.contains("#003f87") && s.svg.contains("#ffcd00"));
    inside(&s);
    assert_svg_parses(&s);
}

#[test]
fn state_route_is_a_drawn_ellipse() {
    let s = render(&engine(), RouteDescriptor::new("US:NJ", "17"));
    assert_eq!(s.rule.shape.as_deref(), Some("ellipse"));
    inside(&s);
}

#[test]
fn unnumbered_turnpike_uses_the_noref_blank_without_text() {
    let route = RouteDescriptor {
        network: Some("US:PA:Turnpike".into()),
        ..RouteDescriptor::default()
    };
    let s = render(&engine(), route);
    assert_eq!(s.rule.blank.as_deref(), Some("shield_us_pa_turnpike_noref"));
    assert_eq!(s.rule.overrides, vec![AppliedOverride::NoRef]);
    assert!(s.text.is_none());
    // colorDarken #006747 is applied to the white parts of the vector blank.
    assert!(s.svg.contains("#006747"), "{}", s.svg);
}

#[test]
fn over_long_refs_produce_no_shield_not_a_blank_image() {
    let r = engine()
        .render(
            &RouteDescriptor::new("US:I", "12345678"),
            &DisplayContext::default(),
        )
        .unwrap();
    assert!(matches!(
        r,
        Rendering::NoShield {
            reason: NoShieldReason::InvalidRef { utf16_len: 8, .. },
            ..
        }
    ));
}

#[test]
fn names_map_to_refs() {
    let route = RouteDescriptor {
        network: Some("US:KY:Parkway".into()),
        ..RouteDescriptor::default()
    }
    .with_name("Audubon Parkway");
    let s = render(&engine(), route);
    assert_eq!(s.text.as_ref().unwrap().text, "AU");
}

#[test]
fn banners_stack_above_the_shield() {
    let e = engine();
    let one = render(&e, RouteDescriptor::new("US:US:Alternate", "1"));
    assert_eq!(one.banners.len(), 1);
    assert_eq!(one.banners[0].text, "ALT");
    assert_eq!(one.height, 29.0);
    assert_eq!(one.shield_box.y, 9.0);
    assert!(one.banners[0].ink.y + one.banners[0].ink.height <= 9.0);
    let two = render(&e, RouteDescriptor::new("US:US:Truck:Bypass", "1"));
    assert_eq!(
        two.banners
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>(),
        ["TRK", "BYP"]
    );
    assert_eq!(two.height, 20.0 + 9.0 + 1.0 + 9.0);
    assert!(two.banners[1].ink.y >= 10.0);
    assert!(two.svg.contains("<title>TRK BYP 1</title>"));
}

#[test]
fn roman_numbering_and_reflection_and_overrides() {
    let e = engine();
    let ne = render(&e, RouteDescriptor::new("IN:NE", "5"));
    assert_eq!(ne.text.as_ref().unwrap().text, "V");
    assert_eq!(ne.banners[0].text, "NE");
    let hi = render(&e, RouteDescriptor::new("US:HI", "130"));
    assert!(hi.svg.contains("scale(1 -1)"));
    let qew = render(&e, RouteDescriptor::new("CA:ON:primary", "QEW"));
    assert_eq!(
        qew.rule.overrides,
        vec![AppliedOverride::ByRef("QEW".into())]
    );
    assert!(qew.svg.contains("#003f87"));
}

#[test]
fn non_us_and_unicode_refs() {
    let e = engine();
    let s = render(&e, RouteDescriptor::new("CA:ON:primary", "401"));
    assert_eq!(s.rule.blank.as_deref(), Some("shield_ca_on_primary"));
    let greek = render(&e, RouteDescriptor::new("GR:national", "Ε65"));
    assert_eq!(greek.text.as_ref().unwrap().text, "Ε65");
    assert_svg_parses(&greek);
}

#[test]
fn armenian_and_georgian_refs_use_their_noto_faces() {
    let e = engine();
    for (network, r, face) in [
        ("ZZ:unknown", "Մ1", "noto-sans-armenian-condensed-medium"),
        ("US:NJ", "Մ4", "noto-sans-armenian-condensed-medium"),
        ("US:NJ:CR", "ს1", "noto-sans-georgian-condensed-medium"),
    ] {
        let s = render(&e, RouteDescriptor::new(network, r));
        let fonts: Vec<&str> = s
            .dependencies
            .iter()
            .filter(|d| d.kind == roadshield::DependencyKind::Font)
            .map(|d| d.id.as_str())
            .collect();
        assert!(fonts.contains(&face), "{r}: {fonts:?}");
        let no_glyph_warnings = s
            .warnings
            .iter()
            .all(|w| matches!(w, Warning::GenericFallback { .. }));
        assert!(no_glyph_warnings, "{r}: {:?}", s.warnings);
        inside(&s);
        assert_svg_parses(&s);
    }
}

#[test]
fn unknown_networks_use_the_explicit_generic_rule_never_a_guess() {
    let e = engine();
    let s = render(&e, RouteDescriptor::new("ZZ:somewhere", "95"));
    assert_eq!(s.rule.rule_key, "default");
    assert!(s.rule.fallback && s.rule.blank.is_none() && s.rule.shape.is_none());
    assert_eq!(
        s.warnings,
        vec![Warning::GenericFallback {
            network: "ZZ:somewhere".into()
        }]
    );
    let strict = DisplayContext {
        unknown_network: UnknownNetworkPolicy::Unsupported,
        ..DisplayContext::default()
    };
    let err = e
        .render(&RouteDescriptor::new("ZZ:somewhere", "95"), &strict)
        .unwrap_err();
    assert!(matches!(err, ShieldError::UnknownNetwork { .. }));
}

#[test]
fn missing_glyphs_are_explicit() {
    let e = engine();
    let err = e
        .render(
            &RouteDescriptor::new("ZZ", "中1"),
            &DisplayContext::default(),
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            ShieldError::MissingGlyph {
                codepoint: 0x4E2D,
                ..
            }
        ),
        "{err}"
    );
    let lenient = DisplayContext {
        missing_glyph: MissingGlyphPolicy::Notdef,
        ..DisplayContext::default()
    };
    let r = e
        .render(&RouteDescriptor::new("ZZ", "中1"), &lenient)
        .unwrap();
    assert!(
        r.symbol()
            .unwrap()
            .warnings
            .contains(&Warning::NotdefDrawn { codepoint: 0x4E2D })
    );
}

#[test]
fn oversized_input_is_rejected() {
    let huge = "9".repeat(10_000);
    let err = engine()
        .render(
            &RouteDescriptor::new("US:I", huge),
            &DisplayContext::default(),
        )
        .unwrap_err();
    assert!(matches!(err, ShieldError::InvalidInput { .. }));
}

#[test]
fn output_is_deterministic_across_threads_and_reloads() {
    let e = engine();
    let route = RouteDescriptor::new("US:I", "287");
    let first = render(&e, route.clone());
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| s.spawn(|| render(&e, route.clone())))
            .collect();
        for h in handles {
            let other = h.join().unwrap();
            assert_eq!(other.svg, first.svg);
            assert_eq!(other.semantic_key, first.semantic_key);
        }
    });
    let reloaded = render(&engine(), route.clone());
    assert_eq!(reloaded.svg, first.svg);
    let scaled = e
        .render(
            &route,
            &DisplayContext {
                scale: 2.0,
                ..DisplayContext::default()
            },
        )
        .unwrap();
    let scaled = scaled.symbol().unwrap();
    assert_ne!(scaled.semantic_key, first.semantic_key);
    assert_eq!((scaled.width, scaled.view_box.width), (50.0, 25.0));
}

#[test]
fn semantic_keys_isolate_pack_versions() {
    let route = RouteDescriptor::new("US:I", "287");
    let ctx = DisplayContext::default();
    assert_ne!(
        roadshield::semantic_key("aaa", &route, &ctx),
        roadshield::semantic_key("bbb", &route, &ctx)
    );
    let colour_only = RouteDescriptor {
        colour: Some("red".into()),
        ..route.clone()
    };
    assert_eq!(
        roadshield::semantic_key("aaa", &route, &ctx),
        roadshield::semantic_key("aaa", &colour_only, &ctx)
    );
}

#[test]
fn keys_ignore_inputs_that_do_not_change_output() {
    let e = engine();
    let route = RouteDescriptor::new("US:I", "287");
    let plain = DisplayContext::default();
    let pinned = DisplayContext {
        expected_pack: Some(roadshield::PackExpectation {
            id: "americana".into(),
            content_hash: Some(e.manifest().content_hash.clone()),
        }),
        ..DisplayContext::default()
    };
    let explicit_stack = DisplayContext {
        font_stack: Some(e.manifest().font_stack.clone()),
        ..DisplayContext::default()
    };
    let key = e.semantic_key(&route, &plain);
    assert_eq!(e.semantic_key(&route, &pinned), key);
    assert_eq!(e.semantic_key(&route, &explicit_stack), key);
    let a = e.render(&route, &pinned).unwrap();
    let b = e.render(&route, &explicit_stack).unwrap();
    assert_eq!(a.symbol().unwrap().svg, b.symbol().unwrap().svg);
    let strict = DisplayContext {
        unknown_network: roadshield::UnknownNetworkPolicy::Unsupported,
        ..DisplayContext::default()
    };
    assert_ne!(
        e.semantic_key(&route, &strict),
        key,
        "policies can change output"
    );
}

#[test]
fn metadata_reports_versions_and_dependencies() {
    let e = engine();
    let s = render(&e, RouteDescriptor::new("US:I", "287"));
    assert_eq!(s.provenance.pack_id, "americana");
    assert_eq!(s.provenance.pack_content_hash, e.manifest().content_hash);
    assert_eq!(s.provenance.upstream_commit, e.manifest().upstream.commit);
    let kinds: Vec<String> = s
        .dependencies
        .iter()
        .map(|d| format!("{:?}:{}", d.kind, d.id))
        .collect();
    assert!(
        kinds.contains(&"Blank:shield_us_interstate_3".to_owned()),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&"Font:noto-sans-condensed-medium".to_owned()),
        "{kinds:?}"
    );
    assert!(s.dependencies.iter().all(|d| d.blake3.len() == 64));
    assert_eq!(s.anchor, (12.5, 10.0));
}

#[test]
fn accessibility_title_can_be_omitted() {
    let e = engine();
    let s = render(&e, RouteDescriptor::new("US:I", "287"));
    assert!(
        s.svg.contains("role=\"img\" aria-label=\"287\"") && s.svg.contains("<title>287</title>")
    );
    let ctx = DisplayContext {
        accessibility: Accessibility {
            omit_title: true,
            ..Accessibility::default()
        },
        ..DisplayContext::default()
    };
    let hidden = e
        .render(&RouteDescriptor::new("US:I", "287"), &ctx)
        .unwrap();
    let svg = &hidden.symbol().unwrap().svg;
    assert!(svg.contains("aria-hidden=\"true\"") && !svg.contains("<title>"));
}

#[test]
fn every_network_renders_or_declines_without_errors() {
    let e = engine();
    let networks: Vec<String> = e.networks().map(str::to_owned).collect();
    let mut symbols = 0;
    let mut errors = Vec::new();
    let mut blanks_used = std::collections::HashSet::new();
    for n in &networks {
        for r in ["1", "22", "287", "1234"] {
            match e.render(
                &RouteDescriptor::new(n.clone(), r),
                &DisplayContext::default(),
            ) {
                Ok(Rendering::Symbol(s)) => {
                    symbols += 1;
                    if let Some(b) = s.rule.blank {
                        blanks_used.insert(b);
                    }
                    assert!(
                        s.width >= 10.0 && s.width <= 64.0 && s.height >= 10.0,
                        "{n} {r}: {}x{}",
                        s.width,
                        s.height
                    );
                }
                Ok(Rendering::NoShield { .. }) => {}
                Err(err) => errors.push(format!("{n} {r}: {err}")),
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    assert!(symbols > networks.len() * 3, "{symbols}");
    assert!(blanks_used.len() > 150, "{}", blanks_used.len());
}
