use super::*;
use crate::model::ShapeParams;

fn env_with(advance: &dyn Fn(&str, f64) -> Result<f64, crate::ShieldError>) -> ShapeEnv<'_> {
    ShapeEnv {
        size: 20.0,
        px: 1.0,
        canvas_width: 20.0,
        text: "287",
        advance,
    }
}

/// Advance of 0.5 em per character.
fn half_em(t: &str, px: f64) -> Result<f64, crate::ShieldError> {
    Ok(t.chars().count() as f64 * px * 0.5)
}

#[test]
fn width_from_text_advance_is_clamped() {
    let env = env_with(&half_em);
    // ceil(3 * 9) + 2 = 29
    assert_eq!(
        compute_width(&env, &ShapeParams::default(), None).unwrap(),
        29.0
    );
    let wide = ShapeEnv {
        text: "1234567",
        ..env_with(&half_em)
    };
    assert_eq!(
        compute_width(&wide, &ShapeParams::default(), None).unwrap(),
        34.0
    );
    let narrow = ShapeEnv {
        text: "1",
        ..env_with(&half_em)
    };
    assert_eq!(
        compute_width(&narrow, &ShapeParams::default(), None).unwrap(),
        20.0
    );
}

#[test]
fn width_adjustments_per_shape() {
    let narrow = ShapeEnv {
        text: "1",
        ..env_with(&half_em)
    };
    let p = ShapeParams::default();
    assert_eq!(compute_width(&narrow, &p, Some("triangle")).unwrap(), 22.0);
    assert_eq!(compute_width(&narrow, &p, Some("diamond")).unwrap(), 24.0);
    assert_eq!(
        compute_width(&narrow, &p, Some("hexagonHorizontal")).unwrap(),
        24.0
    );
    let angled = ShapeParams {
        side_angle: Some(15f64.to_radians()),
        y_offset: Some(3.0),
        ..ShapeParams::default()
    };
    let env = env_with(&half_em);
    let expect = 29.0 + (17.0 * 15f64.to_radians().tan()) / 2.0;
    assert!((compute_width(&env, &angled, Some("pentagon")).unwrap() - expect).abs() < 1e-9);
    let expect = 29.0 + (20.0 * 15f64.to_radians().tan()) / 2.0;
    assert!((compute_width(&env, &angled, Some("trapezoid")).unwrap() - expect).abs() < 1e-9);
}

#[test]
fn fixed_and_explicit_widths_skip_measurement() {
    let fail = |_: &str, _: f64| -> Result<f64, crate::ShieldError> {
        Err(crate::ShieldError::Render {
            network: String::new(),
            detail: "measured".into(),
        })
    };
    let env = env_with(&fail);
    assert_eq!(
        compute_width(&env, &ShapeParams::default(), Some("paBelt")).unwrap(),
        20.0
    );
    let p = ShapeParams {
        rect_width: Some(26.0),
        ..ShapeParams::default()
    };
    assert_eq!(compute_width(&env, &p, None).unwrap(), 26.0);
    assert!(compute_width(&env, &ShapeParams::default(), None).is_err());
}

#[test]
fn diamond_is_taller() {
    assert_eq!(shape_height(20.0, 1.0, "diamond"), 24.0);
    assert_eq!(shape_height(40.0, 2.0, "diamond"), 48.0);
    assert_eq!(shape_height(20.0, 1.0, "pentagon"), 20.0);
}

#[test]
fn every_registered_shape_draws_closed_paths() {
    let env = env_with(&half_em);
    let params = ShapeParams {
        radius: Some(2.0),
        radius1: Some(2.0),
        radius2: Some(1.0),
        y_offset: Some(4.0),
        side_angle: Some(0.2),
        ..ShapeParams::default()
    };
    for shape in SHAPES {
        let ops = draw(&env, shape, &params)
            .unwrap()
            .unwrap_or_else(|| panic!("{shape} not drawn"));
        assert!(!ops.is_empty(), "{shape}");
        for op in ops {
            assert!(
                op.d.starts_with('M') && op.d.ends_with('Z'),
                "{shape}: {}",
                op.d
            );
            assert!(
                !op.d.contains("NaN") && !op.d.contains("inf"),
                "{shape}: {}",
                op.d
            );
        }
    }
    assert!(draw(&env, "hexagram", &params).unwrap().is_none());
}

#[test]
fn rounded_rectangle_geometry_matches_canvas() {
    let env = env_with(&half_em);
    let p = ShapeParams {
        radius: Some(2.0),
        rect_width: Some(20.0),
        ..ShapeParams::default()
    };
    let ops = draw(&env, "roundedRectangle", &p).unwrap().unwrap();
    assert_eq!(
        ops[0].d,
        "M17.5 0.5 L17.5 0.5 A2 2 0 0 1 19.5 2.5 L19.5 17.5 A2 2 0 0 1 17.5 19.5 L2.5 19.5 A2 2 0 0 1 0.5 17.5 L0.5 2.5 A2 2 0 0 1 2.5 0.5 Z"
    );
    assert_eq!(ops[0].fill, "white");
    assert_eq!(ops[0].stroke, Some(("black".into(), 1.0)));
}

#[test]
fn pill_uses_half_height_radius_and_ellipse_centres_on_canvas() {
    let env = env_with(&half_em);
    let p = ShapeParams {
        rect_width: Some(30.0),
        ..ShapeParams::default()
    };
    let pill = draw(&env, "pill", &p).unwrap().unwrap();
    assert!(pill[0].d.contains("A10 10"), "{}", pill[0].d);
    let wide_canvas = ShapeEnv {
        canvas_width: 40.0,
        ..env_with(&half_em)
    };
    let e = draw(&wide_canvas, "ellipse", &p).unwrap().unwrap();
    // centre x = 40 / 2, rx = 30 / 2 - 1
    assert!(e[0].d.starts_with("M34 10 A14 9"), "{}", e[0].d);
}

#[test]
fn device_grid_scales_lengths_but_rounds_in_device_pixels() {
    // 2x: ceil(3 * 18) + 4 = 58, versus 2 * (ceil(27) + 2) = 58 here, but
    // fractional advances round differently on each grid.
    let third = |t: &str, px: f64| -> Result<f64, crate::ShieldError> {
        Ok(t.chars().count() as f64 * px * 0.47)
    };
    let one = ShapeEnv {
        text: "287",
        ..env_with(&third)
    };
    let two = ShapeEnv {
        text: "287",
        size: 40.0,
        px: 2.0,
        canvas_width: 40.0,
        advance: &third,
    };
    let w1 = compute_width(&one, &ShapeParams::default(), None).unwrap();
    let w2 = compute_width(&two, &ShapeParams::default(), None).unwrap();
    assert_eq!(w1, (3.0f64 * 18.0 * 0.47).ceil() + 2.0);
    assert_eq!(w2, (3.0f64 * 36.0 * 0.47).ceil() + 4.0);
    assert_ne!(w2, 2.0 * w1);
    let p = ShapeParams {
        radius: Some(2.0),
        rect_width: Some(20.0),
        ..ShapeParams::default()
    };
    let ops = draw(&two, "roundedRectangle", &p).unwrap().unwrap();
    assert!(ops[0].d.contains("A4 4"), "radius scaled: {}", ops[0].d);
    assert_eq!(ops[0].stroke.as_ref().map(|s| s.1), Some(2.0));
}
