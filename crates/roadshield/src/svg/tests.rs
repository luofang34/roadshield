use super::*;

#[test]
fn halo_joins_serialise_as_svg_attributes() {
    assert_eq!(
        halo_stroke_style(2.0, TextHaloJoin::Round),
        r#"stroke-width="2" stroke-linejoin="round""#
    );
    assert_eq!(
        halo_stroke_style(4.0, TextHaloJoin::Bevel),
        r#"stroke-width="4" stroke-linejoin="bevel""#
    );
    assert_eq!(
        halo_stroke_style(2.0, TextHaloJoin::AMERICANA),
        r#"stroke-width="2" stroke-linejoin="miter" stroke-miterlimit="10""#
    );
}

#[test]
fn shape_strokes_keep_canvas_miter_joins() {
    assert_eq!(
        stroke_style(1.0),
        r#"stroke-width="1" stroke-linejoin="miter" stroke-miterlimit="10""#
    );
}
