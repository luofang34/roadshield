use super::*;

#[test]
fn numbers_are_compact_and_stable() {
    assert_eq!(num(1.0), "1");
    assert_eq!(num(0.123_456), "0.1235");
    assert_eq!(num(-0.00001), "0");
    assert_eq!(num(f64::NAN), "0");
    assert_eq!(num(12.5), "12.5");
}

#[test]
fn arc_to_rounds_a_right_angle_corner() {
    let mut p = Path::new();
    p.move_to(0.0, 0.0);
    p.arc_to(10.0, 0.0, 10.0, 10.0, 2.0);
    assert_eq!(p.data(), "M0 0 L8 0 A2 2 0 0 1 10 2");
}

#[test]
fn arc_to_turning_left_uses_negative_sweep() {
    let mut p = Path::new();
    p.move_to(10.0, 10.0);
    p.arc_to(10.0, 0.0, 0.0, 0.0, 2.0);
    assert_eq!(p.data(), "M10 10 L10 2 A2 2 0 0 0 8 0");
}

#[test]
fn degenerate_arc_to_draws_a_line_to_the_control_point() {
    let mut p = Path::new();
    p.move_to(0.0, 0.0);
    p.arc_to(10.0, 0.0, 20.0, 0.0, 2.0);
    p.arc_to(10.0, 5.0, 10.0, 10.0, 0.0);
    p.arc_to(10.0, 5.0, 0.0, 0.0, 1.0);
    assert_eq!(p.data(), "M0 0 L10 0 L10 5 L10 5");
}

#[test]
fn arc_to_without_current_point_moves() {
    let mut p = Path::new();
    p.arc_to(3.0, 4.0, 5.0, 6.0, 1.0);
    assert_eq!(p.data(), "M3 4");
}

#[test]
fn ellipse_and_rect_close() {
    let mut p = Path::new();
    p.ellipse(10.0, 10.0, 9.0, 9.0);
    assert_eq!(p.data(), "M19 10 A9 9 0 1 1 1 10 A9 9 0 1 1 19 10 Z");
    let mut r = Path::new();
    r.rect(1.0, 2.0, 3.0, 4.0);
    assert_eq!(r.data(), "M1 2 L4 2 L4 6 L1 6 Z");
}

#[test]
fn rect_scaling() {
    assert_eq!(
        Rect::new(1.0, 2.0, 3.0, 4.0).scaled(2.0),
        Rect::new(2.0, 4.0, 6.0, 8.0)
    );
}
