use super::*;
use crate::color::Recolor;

const SVG: &str = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20">
  <script>alert(1)</script>
  <defs><mask id="m"><rect width="20" height="20" fill="#ffffff"/></mask></defs>
  <rect width="20" height="10" style="fill:#000000" mask="url(#m)"/>
  <rect y="10" width="20" height="10" fill="#ffffff"/>
</svg>"##;

#[test]
fn prepares_and_embeds_without_scripts() {
    let b = PreparedBlank::prepare("shield_x_2", SVG.as_bytes(), 20.0, 20.0).unwrap();
    let out = b.embed(0.0, false, None, 1.0).unwrap();
    assert!(
        out.starts_with(
            "<svg width=\"20\" height=\"20\" viewBox=\"0 0 20 20\" preserveAspectRatio=\"none\""
        ),
        "{out}"
    );
    assert!(!out.contains("script"));
    assert!(
        !out.contains("xmlns="),
        "root namespace comes from the outer document: {out}"
    );
    assert!(
        out.contains("rs-shield-x-2-"),
        "ids are prefixed per blank: {out}"
    );
    assert!(!b.has_raster);
}

#[test]
fn recolor_changes_paints_but_not_masks() {
    let b = PreparedBlank::prepare("b", SVG.as_bytes(), 20.0, 20.0).unwrap();
    let rc = Recolor::from_css(Some("#ff0000"), Some("#0000ff")).unwrap();
    let out = b.embed(0.0, false, rc, 1.0).unwrap();
    let mask = &out[out.find("<mask").unwrap()..out.find("</mask>").unwrap()];
    assert!(mask.contains("#ffffff"), "mask content untouched: {mask}");
    let body = &out[out.find("</mask>").unwrap()..];
    assert!(
        body.contains("#ff0000") && body.contains("#0000ff"),
        "{body}"
    );
    assert!(
        !body.contains("#000000") && !body.contains("#ffffff"),
        "{body}"
    );
}

#[test]
fn reflection_and_offset_wrap_the_blank() {
    let b = PreparedBlank::prepare("b", SVG.as_bytes(), 20.0, 20.0).unwrap();
    assert!(
        b.embed(9.0, true, None, 1.0)
            .unwrap()
            .starts_with("<g transform=\"translate(0 29) scale(1 -1)\">")
    );
    assert!(
        b.embed(9.0, false, None, 1.0)
            .unwrap()
            .starts_with("<g transform=\"translate(0 9)\">")
    );
}

#[test]
fn external_images_are_never_fetched() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="4" height="4"><image width="4" height="4" xlink:href="/etc/passwd"/><rect width="1" height="1"/></svg>"#;
    let b = PreparedBlank::prepare("b", svg.as_bytes(), 4.0, 4.0).unwrap();
    assert!(!b.has_raster);
    assert!(!b.embed(0.0, false, None, 1.0).unwrap().contains("passwd"));
}

#[test]
fn embedded_raster_images_are_flagged() {
    // 1x1 transparent PNG.
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="4" height="4"><image width="4" height="4" xlink:href="data:image/png;base64,{png}"/></svg>"#
    );
    let b = PreparedBlank::prepare("b", svg.as_bytes(), 4.0, 4.0).unwrap();
    assert!(b.has_raster);
}

#[test]
fn limits_are_enforced() {
    let big = vec![b' '; MAX_BLANK_BYTES + 1];
    assert!(matches!(
        PreparedBlank::prepare("b", &big, 1.0, 1.0),
        Err(PackError::Blank { .. })
    ));
    assert!(PreparedBlank::prepare("b", SVG.as_bytes(), 0.0, 20.0).is_err());
    assert!(PreparedBlank::prepare("b", b"<not svg", 20.0, 20.0).is_err());
}

#[test]
fn caps_are_dropped_only_on_fully_closed_paths() {
    assert!(only_closed_subpaths("M 0 0 L 1 1 Z M 2 2 L 3 3 Z"));
    assert!(!only_closed_subpaths("M 0 0 L 1 1 M 2 2 L 3 3 Z"));
    assert!(!only_closed_subpaths("M 0 0 L 1 1"));
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4">
        <path d="M0.5 0.5H3V3Z" stroke="#000" stroke-linecap="round"/>
        <path d="M0 0L4 4" stroke="#000" stroke-linecap="round"/></svg>"##;
    let out = PreparedBlank::prepare("b", svg.as_bytes(), 4.0, 4.0)
        .unwrap()
        .embed(0.0, false, None, 1.0)
        .unwrap();
    assert_eq!(out.matches("stroke-linecap").count(), 1, "{out}");
}

#[test]
fn embedding_scales_to_the_layout_grid() {
    let b = PreparedBlank::prepare("b", SVG.as_bytes(), 20.0, 20.0).unwrap();
    assert!(
        b.embed(0.0, false, None, 2.0)
            .unwrap()
            .starts_with("<svg width=\"40\" height=\"40\" viewBox=\"0 0 20 20\"")
    );
    assert!(
        b.embed(0.0, true, None, 2.0)
            .unwrap()
            .starts_with("<g transform=\"translate(0 40) scale(1 -1)\">")
    );
}
