use super::*;
use crate::font::{InkBox, ShapedText};
use crate::model::{Padding, TextLayoutDef, TextLayoutOptions};

fn def(name: &str) -> TextLayoutDef {
    TextLayoutDef {
        constraint_func: name.into(),
        options: None,
    }
}

/// Digits-like ink: 0.5 em wide per glyph, cap height 0.7 em, baseline at 0.
fn text(chars: usize) -> ShapedText {
    #[allow(clippy::cast_precision_loss)]
    let w = chars as f64 * 0.5;
    ShapedText::synthetic(
        w,
        Some(InkBox {
            min_x: 0.05,
            min_y: 0.0,
            max_x: w - 0.05,
            max_y: 0.7,
        }),
    )
}

const TOP: f64 = 0.8;

fn top(px: f64) -> f64 {
    crate::font::top_px(TOP, px)
}

#[test]
fn rect_fit_reproduces_measure_text_quirks() {
    let t = text(3);
    let p = layout(
        &t,
        TOP,
        Padding::default(),
        (25.0, 20.0),
        &def("rect"),
        14.0,
    )
    .unwrap();
    // width = |-0.05| + 1.45 = 1.5 em; height = (|ink top - top| + top) * 0.9
    let s: f64 = 11.8;
    let height = ((0.7 * s - top(s)).abs() + top(s)) * 0.9;
    let scale = (25.0 / (1.5 * s)).min(20.0 / height);
    assert!((p.font_px - (11.8 * scale).min(14.0)).abs() < 1e-9);
    assert_eq!(p.x_center, 12.5);
    let h = (0.7 * p.font_px - top(p.font_px)).abs() + top(p.font_px);
    assert!((p.y_top - (20.0 - h) / 2.0).abs() < 1e-9);
}

#[test]
fn max_font_caps_size() {
    let p = layout(
        &text(1),
        TOP,
        Padding::default(),
        (100.0, 100.0),
        &def("rect"),
        14.0,
    )
    .unwrap();
    assert_eq!(p.font_px, 14.0);
    let p = layout(
        &text(1),
        TOP,
        Padding::default(),
        (100.0, 100.0),
        &def("ellipse"),
        9.0,
    )
    .unwrap();
    assert_eq!(p.font_px, 9.0);
}

#[test]
fn top_aligned_constraints_use_the_ink_ascent() {
    let pad = Padding {
        left: 4.0,
        right: 4.0,
        top: 6.0,
        bottom: 5.0,
    };
    for name in ["southHalfEllipse", "triangleDown"] {
        let p = layout(&text(3), TOP, pad, (25.0, 20.0), &def(name), 14.0).unwrap();
        let ascent = 0.7 * p.font_px - top(p.font_px);
        assert!((p.y_top - (6.0 + ascent)).abs() < 1e-9, "{name}");
    }
}

#[test]
fn rounded_rect_shrinks_by_radius() {
    let small = layout(
        &text(4),
        TOP,
        Padding::default(),
        (30.0, 20.0),
        &def("rect"),
        99.0,
    )
    .unwrap();
    let rr = TextLayoutDef {
        constraint_func: "roundedRect".into(),
        options: Some(TextLayoutOptions { radius: Some(4.0) }),
    };
    let rounded = layout(&text(4), TOP, Padding::default(), (30.0, 20.0), &rr, 99.0).unwrap();
    assert!(rounded.font_px < small.font_px);
    let default_radius = layout(
        &text(4),
        TOP,
        Padding::default(),
        (30.0, 20.0),
        &def("roundedRect"),
        99.0,
    )
    .unwrap();
    assert!(default_radius.font_px > rounded.font_px);
}

#[test]
fn diamond_and_ellipse_scales() {
    let t = text(2);
    let e = layout(
        &t,
        TOP,
        Padding::default(),
        (20.0, 20.0),
        &def("ellipse"),
        99.0,
    )
    .unwrap();
    let d = layout(
        &t,
        TOP,
        Padding::default(),
        (20.0, 20.0),
        &def("diamond"),
        99.0,
    )
    .unwrap();
    let r = layout(
        &t,
        TOP,
        Padding::default(),
        (20.0, 20.0),
        &def("rect"),
        99.0,
    )
    .unwrap();
    assert!(d.font_px < e.font_px && e.font_px < r.font_px);
}

#[test]
fn failures_are_reported() {
    assert_eq!(
        layout(
            &text(2),
            TOP,
            Padding::default(),
            (20.0, 20.0),
            &def("spiral"),
            14.0
        ),
        Err(LayoutError::UnknownConstraint("spiral".into()))
    );
    let crushed = Padding {
        top: 15.0,
        bottom: 15.0,
        ..Padding::default()
    };
    assert!(matches!(
        layout(&text(2), TOP, crushed, (20.0, 20.0), &def("rect"), 14.0),
        Err(LayoutError::NoFit(_))
    ));
}

#[test]
fn whitespace_only_text_uses_the_maximum_size() {
    let empty = ShapedText::synthetic(0.3, None);
    let p = layout(
        &empty,
        TOP,
        Padding::default(),
        (20.0, 20.0),
        &def("rect"),
        14.0,
    )
    .unwrap();
    assert_eq!(p.font_px, 14.0);
}
