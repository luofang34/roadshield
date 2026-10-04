//! Angular shapes: polygons whose corners are rounded with canvas `arcTo`.

use std::f64::consts::PI;

use super::{DrawOp, ShapeEnv, op, shape_height, style, width};
use crate::geometry::Path;
use crate::model::ShapeParams;

pub(super) fn triangle(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let up = p.point_up.unwrap_or(false);
    let sign = if up { -1.0 } else { 1.0 };
    let r = p.radius.unwrap_or(0.0);
    let width = width(env, p, Some("triangle"))?;
    let lw = s.line_thick / 2.0;
    let (x0, x8) = (lw, width - lw);
    let y0 = if up { env.size - lw } else { lw };
    let y5 = if up { lw } else { env.size - lw };
    let x2 = x0 + r;
    let x4 = (x0 + x8) / 2.0;
    let x6 = x8 - r;
    let y1 = y0 + sign * r;
    let angle = ((x4 - x2) / (y5 - r - y1).abs()).atan();
    let (sine, cosine) = angle.sin_cos();
    let half_tan = (angle / 2.0).tan();
    let half_comp_tan = (PI / 4.0 - angle / 2.0).tan();
    let x1 = x2 - r * cosine;
    let x3 = x4 - r * half_comp_tan;
    let x5 = x4 + r * half_comp_tan;
    let x7 = x6 + r * cosine;
    let y2 = y1 + sign * r * half_tan;
    let y3 = y1 + sign * r * sine;
    let mut path = Path::new();
    path.move_to(x4, y5);
    path.arc_to(x3, y5, x1, y3, r);
    path.arc_to(x0, y2, x0, y1, r);
    path.arc_to(x0, y0, x2, y0, r);
    path.arc_to(x8, y0, x8, y1, r);
    path.arc_to(x8, y2, x7, y3, r);
    path.arc_to(x5, y5, x4, y5, r);
    path.close();
    Ok(op(path, &s))
}

pub(super) fn trapezoid(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let short_up = p.short_side_up.unwrap_or(false);
    let side = p.side_angle.unwrap_or(0.0);
    let r = p.radius.unwrap_or(0.0);
    let sign = if short_up { -1.0 } else { 1.0 };
    let (sine, cosine) = side.sin_cos();
    let tangent = side.tan();
    let width = width(env, p, Some("trapezoid"))?;
    let lw = s.line_thick / 2.0;
    let (x0, x9) = (lw, width - lw);
    let y0 = if short_up { env.size - lw } else { lw };
    let y3 = if short_up { lw } else { env.size - lw };
    let y1 = y0 + sign * r * (1.0 + sine);
    let y2 = y3 - sign * r * (1.0 - sine);
    let x1 = x0 + (y1 - y0) * tangent;
    let x2 = x1 + r * cosine;
    let x3 = x0 + sign * (y2 - y0) * tangent;
    let x4 = x0 + sign * (y3 - y0) * tangent;
    let x5 = x3 + sign * r * cosine;
    let (x6, x7, x8) = (width - x4, width - x3, width - x2);
    let mut path = Path::new();
    path.move_to(x8, y0);
    path.arc_to(x9, y0, x7, y2, r);
    path.arc_to(x6, y3, x5, y3, r);
    path.arc_to(x4, y3, x1, y1, r);
    path.arc_to(x0, y0, x8, y0, r);
    path.close();
    Ok(op(path, &s))
}

pub(super) fn diamond(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let r = p.radius.unwrap_or(0.0);
    let height = shape_height(env.size, env.px, "diamond");
    let width = width(env, p, Some("diamond"))?;
    let lw = s.line_thick / 2.0;
    let (x0, x8, y0, y8) = (lw, width - lw, lw, height - lw);
    let x4 = (x0 + x8) / 2.0;
    let y4 = (y0 + y8) / 2.0;
    let angle = ((x4 - r - x0) / (y8 - r - y4)).atan();
    let (sine, cosine) = angle.sin_cos();
    let half_tan = (angle / 2.0).tan();
    let half_comp_tan = (PI / 4.0 - angle / 2.0).tan();
    let x1 = x0 + r * (1.0 - cosine);
    let x2 = x4 - r * cosine;
    let x3 = x4 - r * half_comp_tan;
    let x5 = x4 + r * half_comp_tan;
    let x6 = x4 + r * cosine;
    let x7 = x8 - r * (1.0 - cosine);
    let y1 = y0 + r * (1.0 - sine);
    let y2 = y4 - r * sine;
    let y3 = y4 - r * half_tan;
    let y5 = y4 + r * half_tan;
    let y6 = y4 + r * sine;
    let y7 = y8 - r * (1.0 - sine);
    let mut path = Path::new();
    path.move_to(x4, y8);
    path.arc_to(x3, y8, x1, y6, r);
    path.arc_to(x0, y5, x0, y4, r);
    path.arc_to(x0, y3, x2, y1, r);
    path.arc_to(x3, y0, x4, y0, r);
    path.arc_to(x5, y0, x7, y2, r);
    path.arc_to(x8, y3, x8, y4, r);
    path.arc_to(x8, y5, x6, y7, r);
    path.arc_to(x5, y8, x4, y8, r);
    path.close();
    Ok(op(path, &s))
}

pub(super) fn pentagon(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let up = p.point_up.unwrap_or(true);
    let off = p.y_offset.unwrap_or(0.0);
    let side = p.side_angle.unwrap_or(0.0);
    let r1 = p.radius1.unwrap_or(0.0);
    let r2 = p.radius2.unwrap_or(0.0);
    let sign = if up { -1.0 } else { 1.0 };
    let (sine, cosine) = side.sin_cos();
    let tangent = side.tan();
    let width = width(env, p, Some("pentagon"))?;
    let lw = s.line_thick / 2.0;
    let (x0, x8) = (lw, width - lw);
    let y0 = if up { env.size - lw } else { lw };
    let y3 = if up { lw } else { env.size - lw };
    let y2 = y3 - sign * off;
    let x2 = x0 + sign * (y2 - y0) * tangent;
    let x4 = (x0 + x8) / 2.0;
    let x6 = x8 - sign * (y2 - y0) * tangent;
    let offset_angle = (off / (x4 - x0)).atan();
    let hct1 = ((PI / 2.0 - offset_angle + side) / 2.0).tan();
    let hct2 = ((PI / 2.0 - side) / 2.0).tan();
    let x1 = x0 + r1 * hct1 * sine;
    let x3 = x2 + r2 * hct2;
    let x5 = x6 - r2 * hct2;
    let x7 = x8 - r1 * hct1 * sine;
    let y1 = y2 - sign * r1 * hct1 * cosine;
    let mut path = Path::new();
    path.move_to(x4, y3);
    path.arc_to(x0, y2, x1, y1, r1);
    path.arc_to(x2, y0, x3, y0, r2);
    path.line_to(x5, y0);
    path.arc_to(x6, y0, x7, y1, r2);
    path.arc_to(x8, y2, x4, y3, r1);
    path.close();
    Ok(op(path, &s))
}

pub(super) fn hexagon_vertical(
    env: &ShapeEnv<'_>,
    p: &ShapeParams,
) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let off = p.y_offset.unwrap_or(0.0);
    let r = p.radius.unwrap_or(0.0);
    let width = width(env, p, None)?;
    let lw = s.line_thick / 2.0;
    let (x0, x2, y0, y5) = (lw, width - lw, lw, env.size - lw);
    let x1 = (x0 + x2) / 2.0;
    let y1 = y0 + off;
    let y4 = y5 - off;
    let t = r * (PI / 4.0 - (off / (x1 - x0)).asin() / 2.0).tan();
    let y2 = y1 + t;
    let y3 = y4 - t;
    let mut path = Path::new();
    path.move_to(x1, y5);
    path.arc_to(x0, y4, x0, y3, r);
    path.arc_to(x0, y1, x1, y0, r);
    path.line_to(x1, y0);
    path.arc_to(x2, y1, x2, y2, r);
    path.arc_to(x2, y4, x1, y5, r);
    path.line_to(x1, y5);
    path.close();
    Ok(op(path, &s))
}

pub(super) fn hexagon_horizontal(
    env: &ShapeEnv<'_>,
    p: &ShapeParams,
) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let side = p.side_angle.unwrap_or(0.0);
    let r = p.radius.unwrap_or(0.0);
    let (sine, cosine) = side.sin_cos();
    let tangent = side.tan();
    let hct = (PI / 4.0 - side / 2.0).tan();
    let width = width(env, p, Some("hexagonHorizontal"))?;
    let lw = s.line_thick / 2.0;
    let (x0, x9, y0, y6) = (lw, width - lw, lw, env.size - lw);
    let y3 = (y0 + y6) / 2.0;
    let y1 = y0 + r * hct * cosine;
    let y2 = y3 - r * sine;
    let y4 = y3 + r * sine;
    let y5 = y6 - r * hct * cosine;
    let x1 = x0 + (y3 - y2) * tangent;
    let x3 = x0 + (y3 - y0) * tangent;
    let x6 = x9 - (y3 - y0) * tangent;
    let x8 = x9 - (y3 - y2) * tangent;
    let x2 = x3 - r * hct * sine;
    let x4 = x3 + r * hct;
    let x5 = x6 - r * hct;
    let x7 = x6 + r * hct * sine;
    let mut path = Path::new();
    path.move_to(x4, y0);
    path.arc_to(x6, y0, x7, y1, r);
    path.arc_to(x9, y3, x8, y4, r);
    path.arc_to(x6, y6, x5, y6, r);
    path.arc_to(x3, y6, x2, y5, r);
    path.arc_to(x0, y3, x1, y2, r);
    path.arc_to(x3, y0, x4, y0, r);
    path.close();
    Ok(op(path, &s))
}

pub(super) fn octagon_vertical(
    env: &ShapeEnv<'_>,
    p: &ShapeParams,
) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let off = p.y_offset.unwrap_or(0.0);
    let side = p.side_angle.unwrap_or(0.0);
    let r = p.radius.unwrap_or(0.0);
    let (sine, cosine) = side.sin_cos();
    let tangent = side.tan();
    let width = width(env, p, None)?;
    let lw = s.line_thick / 2.0;
    let (x0, x10, y0, y10) = (lw, width - lw, lw, env.size - lw);
    let x1 = x0 + r * tangent * sine;
    let x5 = (x0 + x10) / 2.0;
    let x9 = x10 - r * tangent * sine;
    let y2 = y0 + off;
    let y5 = (y0 + y10) / 2.0;
    let y8 = y10 - off;
    let x3 = x0 + (y5 - y2) * tangent;
    let x7 = x10 - (y5 - y2) * tangent;
    let y4 = y5 - r * tangent * cosine;
    let y6 = y5 + r * tangent * cosine;
    let offset_angle = (off / (x5 - x3)).atan();
    let (off_sin, off_cos) = offset_angle.sin_cos();
    let hca = (PI / 2.0 - side - offset_angle) / 2.0;
    let hcc = hca.cos();
    let dx = (r * (side + hca).cos()) / hcc;
    let dy = (r * (side + hca).sin()) / hcc;
    let x2 = x3 + dx - r * cosine;
    let x4 = x3 + dx - r * off_sin;
    let x6 = x7 - dx + r * off_sin;
    let x8 = x7 - dx + r * cosine;
    let y1 = y2 + dy - r * off_cos;
    let y3 = y2 + dy - r * sine;
    let y7 = y8 - dy + r * sine;
    let y9 = y8 - dy + r * off_cos;
    let mut path = Path::new();
    path.move_to(x5, y10);
    path.arc_to(x3, y8, x2, y7, r);
    path.arc_to(x0, y5, x1, y4, r);
    path.arc_to(x3, y2, x4, y1, r);
    path.line_to(x5, y0);
    path.arc_to(x7, y2, x8, y3, r);
    path.arc_to(x10, y5, x9, y6, r);
    path.arc_to(x7, y8, x6, y9, r);
    path.line_to(x5, y10);
    path.close();
    Ok(op(path, &s))
}
