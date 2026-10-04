use super::*;

#[test]
fn parses_css_forms() {
    assert_eq!(Rgba::parse("#003f87"), Some(Rgba([0, 0x3f, 0x87, 255])));
    assert_eq!(Rgba::parse("white"), Some(Rgba::WHITE));
    assert_eq!(Rgba::parse("hsl(30, 44%, 96%)").map(|c| c.0[3]), Some(255));
    assert_eq!(Rgba::parse("not-a-colour"), None);
    assert_eq!(Rgba([1, 2, 3, 255]).hex(), "#010203");
}

#[test]
fn recolor_maps_black_and_white_to_lighten_and_darken() {
    let rc = Recolor::from_css(Some("white"), Some("#006747"))
        .unwrap()
        .unwrap();
    assert_eq!(rc.apply(Rgba::BLACK), Rgba::WHITE);
    assert_eq!(rc.apply(Rgba::WHITE), Rgba([0, 0x67, 0x47, 255]));
    // Alpha is untouched and mid-tones interpolate linearly.
    let mid = rc.apply(Rgba([128, 128, 128, 77]));
    assert_eq!(mid.0[3], 77);
    assert_eq!(mid.0[0], 127);
}

#[test]
fn recolor_defaults_and_absence() {
    assert_eq!(Recolor::from_css(None, None), Ok(None));
    assert_eq!(Recolor::from_css(Some(""), None), Ok(None));
    let only_darken = Recolor::from_css(None, Some("red")).unwrap().unwrap();
    assert_eq!(only_darken.lighten, Rgba::BLACK);
    assert!(Recolor::from_css(Some("bogus"), None).is_err());
}

#[test]
fn recolor_commutes_with_blending() {
    // Affine maps preserve convex combinations, which is why recolouring
    // paints equals recolouring composited pixels.
    let rc = Recolor::from_css(Some("#ffcd00"), Some("#003f87"))
        .unwrap()
        .unwrap();
    let (a, b) = (Rgba([10, 200, 30, 255]), Rgba([250, 20, 140, 255]));
    let blend = |x: Rgba, y: Rgba| {
        let m = |i: usize| ((u16::from(x.0[i]) + u16::from(y.0[i])) / 2) as u8;
        Rgba([m(0), m(1), m(2), 255])
    };
    let lhs = rc.apply(blend(a, b));
    let rhs = blend(rc.apply(a), rc.apply(b));
    for i in 0..3 {
        assert!((i32::from(lhs.0[i]) - i32::from(rhs.0[i])).abs() <= 1);
    }
}
