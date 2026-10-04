use super::*;

fn pack_font(id: &str) -> FontFace {
    let path = format!(
        "{}/../../packs/americana/fonts/{id}.ttf",
        env!("CARGO_MANIFEST_DIR")
    );
    FontFace::parse(id, std::fs::read(path).unwrap()).unwrap()
}

fn stack() -> FontStack {
    FontStack::new(vec![
        pack_font("noto-sans-condensed-medium"),
        pack_font("noto-sans-armenian-condensed-medium"),
        pack_font("noto-sans-georgian-condensed-medium"),
    ])
    .unwrap()
}

#[test]
fn top_baseline_is_normalised_typo_ascent() {
    let f = pack_font("noto-sans-condensed-medium");
    assert!((f.top_em() - 1069.0 / 1362.0).abs() < 1e-12);
    let arm = pack_font("noto-sans-armenian-condensed-medium");
    assert!((arm.top_em() - 1068.0 / 1360.0).abs() < 1e-12);
}

#[test]
fn shapes_digits_with_ink_inside_the_em_box() {
    let t = stack()
        .shape("287", TextDirection::Auto, None, MissingGlyphPolicy::Error)
        .unwrap();
    assert!(t.advance > 1.0 && t.advance < 2.0, "{}", t.advance);
    let ink = t.ink.unwrap();
    assert!(ink.min_y > -0.05 && ink.min_y < 0.05, "{ink:?}");
    assert!(ink.max_y > 0.65 && ink.max_y < 0.75, "{ink:?}");
    assert!(ink.min_x > 0.0 && ink.max_x < t.advance);
    assert_eq!(t.faces_used, vec![0]);
}

#[test]
fn falls_back_per_character() {
    let s = stack();
    let arm = s
        .shape("Մ1", TextDirection::Auto, None, MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(arm.faces_used, vec![1, 0]);
    let geo = s
        .shape("ს1", TextDirection::Auto, None, MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(geo.faces_used, vec![2, 0]);
    // Armenian capitals sit on the baseline at cap height like Latin ones.
    let ink = arm.ink.unwrap();
    assert!(
        ink.min_y.abs() < 0.02 && ink.max_y > 0.65 && ink.max_y < 0.75,
        "{ink:?}"
    );
}

#[test]
fn missing_glyphs_are_errors_or_notdef() {
    let err = stack()
        .shape("中1", TextDirection::Auto, None, MissingGlyphPolicy::Error)
        .unwrap_err();
    assert!(
        matches!(
            err,
            ShieldError::MissingGlyph {
                codepoint: 0x4E2D,
                ..
            }
        ),
        "{err:?}"
    );
    let t = stack()
        .shape("中1", TextDirection::Auto, None, MissingGlyphPolicy::Notdef)
        .unwrap();
    assert_eq!(t.notdef, vec![0x4E2D]);
}

#[test]
fn outlines_are_deterministic_and_positioned() {
    let s = stack();
    let t = s
        .shape("I", TextDirection::Auto, None, MissingGlyphPolicy::Error)
        .unwrap();
    let a = s.outline_path(&t, 10.0, 5.0, 20.0);
    let b = s.outline_path(&t, 10.0, 5.0, 20.0);
    assert_eq!(a, b);
    assert!(a.starts_with('M') && a.ends_with('Z'));
    // "I" is a rectangle stem: every y lies between cap height and baseline.
    let nums: Vec<f64> = a
        .split(|c: char| c.is_ascii_alphabetic() || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap())
        .collect();
    for pair in nums.chunks(2) {
        assert!(pair[1] <= 20.0 + 1e-9 && pair[1] >= 12.0, "{a}");
        assert!(pair[0] >= 5.0, "{a}");
    }
}

#[test]
fn whitespace_has_advance_but_no_ink() {
    let t = stack()
        .shape(" ", TextDirection::Auto, None, MissingGlyphPolicy::Error)
        .unwrap();
    assert!(t.advance > 0.0);
    assert!(t.ink.is_none());
}
