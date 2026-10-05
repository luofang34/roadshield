//! Programmatic shield shapes, ported from `shield_canvas_draw.ts` and
//! `custom_shields.ts`. All coordinates are 1x logical pixels.

mod angular;
mod custom;

use crate::geometry::Path;
use crate::model::ShapeParams;

const MIN_GENERIC_WIDTH: f64 = 20.0;
const MAX_GENERIC_WIDTH: f64 = 34.0;
/// Font size used to size variable-width shapes.
pub const GENERIC_FONT_SIZE: f64 = 18.0;

/// Every shape name the engine can draw.
pub const SHAPES: &[&str] = &[
    "diamond",
    "ellipse",
    "escutcheon",
    "fishhead",
    "hexagonVertical",
    "hexagonHorizontal",
    "octagonVertical",
    "pentagon",
    "pill",
    "roundedRectangle",
    "trapezoid",
    "triangle",
    "branson",
    "paBelt",
];

fn fixed_width(shape: &str) -> Option<f64> {
    matches!(shape, "branson" | "paBelt").then_some(20.0)
}

/// One filled (and optionally stroked) path.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawOp {
    /// SVG path data.
    pub d: String,
    /// CSS fill colour.
    pub fill: String,
    /// CSS stroke colour and width.
    pub stroke: Option<(String, f64)>,
}

/// Inputs shared by all shapes.
pub struct ShapeEnv<'a> {
    /// Shield size (height of most shapes), in device pixels.
    pub size: f64,
    /// Device pixels per layout pixel (upstream `r.px(1)`).
    pub px: f64,
    /// Width of the canvas the shape is drawn on.
    pub canvas_width: f64,
    /// Text being drawn, for variable-width shapes.
    pub text: &'a str,
    /// Advance width of `text` at a font size, in pixels.
    pub advance: &'a dyn Fn(&str, f64) -> Result<f64, crate::ShieldError>,
}

/// Shape lengths converted to device pixels (upstream applies `r.px` to
/// each at use).
fn scaled(p: &ShapeParams, px: f64) -> ShapeParams {
    let k = |v: Option<f64>| v.map(|v| v * px);
    ShapeParams {
        rect_width: k(p.rect_width),
        radius: k(p.radius),
        radius1: k(p.radius1),
        radius2: k(p.radius2),
        y_offset: k(p.y_offset),
        outline_width: Some(p.outline_width.unwrap_or(1.0) * px),
        ..p.clone()
    }
}

/// `computeWidth`: fixed widths, explicit `rectWidth`, or the text advance at
/// 18px plus 2, clamped to [20, 34] with shape-specific adjustments; all in
/// device pixels.
pub fn compute_width(
    env: &ShapeEnv<'_>,
    params: &ShapeParams,
    shape: Option<&str>,
) -> Result<f64, crate::ShieldError> {
    width(env, &scaled(params, env.px), shape)
}

/// [`compute_width`] for parameters already in device pixels.
fn width(
    env: &ShapeEnv<'_>,
    params: &ShapeParams,
    shape: Option<&str>,
) -> Result<f64, crate::ShieldError> {
    let px = env.px;
    if let Some(w) = shape.and_then(fixed_width) {
        return Ok(w * px);
    }
    if let Some(w) = params.rect_width {
        return Ok(w);
    }
    let tangent = params.side_angle.unwrap_or(0.0).tan();
    let mut width = (env.advance)(env.text, GENERIC_FONT_SIZE * px)?.ceil() + 2.0 * px;
    let mut min = MIN_GENERIC_WIDTH * px;
    match shape {
        Some("pentagon") => {
            width += ((env.size - params.y_offset.unwrap_or(0.0)) * tangent) / 2.0;
        }
        Some("trapezoid") => width += (env.size * tangent) / 2.0,
        Some("triangle") => min += 2.0 * px,
        Some("diamond" | "hexagonHorizontal") => min += 4.0 * px,
        _ => {}
    }
    Ok(min.max((MAX_GENERIC_WIDTH * px).min(width)))
}

/// `shapeHeight`.
pub fn shape_height(size: f64, px: f64, shape: &str) -> f64 {
    if shape == "diamond" {
        size + 4.0 * px
    } else {
        size
    }
}

struct Style {
    fill: String,
    outline: String,
    line_thick: f64,
}

fn style(p: &ShapeParams) -> Style {
    Style {
        fill: p.fill_color.clone().unwrap_or_else(|| "white".into()),
        outline: p.stroke_color.clone().unwrap_or_else(|| "black".into()),
        line_thick: p.outline_width.unwrap_or(1.0),
    }
}

fn op(path: &Path, s: &Style) -> DrawOp {
    DrawOp {
        d: path.data().to_owned(),
        fill: s.fill.clone(),
        stroke: Some((s.outline.clone(), s.line_thick)),
    }
}

/// Draws `shape`, returning its paths. `None` for an unknown shape name.
pub fn draw(
    env: &ShapeEnv<'_>,
    shape: &str,
    params: &ShapeParams,
) -> Result<Option<Vec<DrawOp>>, crate::ShieldError> {
    let params = &scaled(params, env.px);
    let ops = match shape {
        "ellipse" => vec![ellipse(env, params)?],
        "pill" => {
            let mut p = params.clone();
            p.radius = Some(env.size / 2.0);
            vec![rounded_rectangle(env, &p)?]
        }
        "roundedRectangle" => vec![rounded_rectangle(env, params)?],
        "escutcheon" => vec![escutcheon(env, params)?],
        "fishhead" => vec![fishhead(env, params)?],
        "triangle" => vec![angular::triangle(env, params)?],
        "trapezoid" => vec![angular::trapezoid(env, params)?],
        "diamond" => vec![angular::diamond(env, params)?],
        "pentagon" => vec![angular::pentagon(env, params)?],
        "hexagonVertical" => vec![angular::hexagon_vertical(env, params)?],
        "hexagonHorizontal" => vec![angular::hexagon_horizontal(env, params)?],
        "octagonVertical" => vec![angular::octagon_vertical(env, params)?],
        "paBelt" => custom::pa_belt(env, params)?,
        "branson" => custom::branson(env, params)?,
        _ => return Ok(None),
    };
    Ok(Some(ops))
}

fn ellipse(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = Style {
        line_thick: env.px,
        ..style(p)
    };
    let width = width(env, p, None)?;
    let rx = width / 2.0 - s.line_thick;
    let ry = env.size / 2.0 - s.line_thick;
    let mut path = Path::new();
    path.ellipse(env.canvas_width / 2.0, env.size / 2.0, rx, ry);
    Ok(op(&path, &s))
}

fn rounded_rectangle(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let r = p.radius.unwrap_or(0.0);
    let width = width(env, p, None)?;
    let lw = s.line_thick / 2.0;
    let (x0, x1, x2, x3) = (lw, lw + r, width - lw - r, width - lw);
    let (y0, y1, y2, y3) = (lw, lw + r, env.size - lw - r, env.size - lw);
    let mut path = Path::new();
    path.move_to(x2, y0);
    path.arc_to(x3, y0, x3, y1, r);
    path.arc_to(x3, y3, x2, y3, r);
    path.arc_to(x0, y3, x0, y2, r);
    path.arc_to(x0, y0, x1, y0, r);
    path.close();
    Ok(op(&path, &s))
}

fn escutcheon(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let r = p.radius.unwrap_or(0.0);
    let off = p.y_offset.unwrap_or(0.0);
    let width = width(env, p, None)?;
    let lw = s.line_thick / 2.0;
    let (x0, x5, y0, y5) = (lw, width - lw, lw, env.size - lw);
    let x1 = x0 + r;
    let x3 = (x0 + x5) / 2.0;
    let y1 = y0 + r;
    let y2 = y5 - off;
    let x2 = (2.0 * x0 + x3) / 3.0;
    let x4 = (x3 + 2.0 * x5) / 3.0;
    let y3 = (y2 + y5) / 2.0;
    let y4 = (y3 + 2.0 * y5) / 3.0;
    let mut path = Path::new();
    path.move_to(x3, y5);
    path.bezier_to(x2, y4, x0, y3, x0, y2);
    path.arc_to(x0, y0, x1, y0, r);
    path.arc_to(x5, y0, x5, y1, r);
    path.line_to(x5, y2);
    path.bezier_to(x5, y3, x4, y4, x3, y5);
    path.close();
    Ok(op(&path, &s))
}

fn fishhead(env: &ShapeEnv<'_>, p: &ShapeParams) -> Result<DrawOp, crate::ShieldError> {
    let s = style(p);
    let up = p.point_up.unwrap_or(false);
    let sign = if up { -1.0 } else { 1.0 };
    let width = width(env, p, None)?;
    let lw = s.line_thick / 2.0;
    let (x0, x8) = (lw, width - lw);
    let y0 = if up { env.size - lw } else { lw };
    let y6 = if up { lw } else { env.size - lw };
    let k = env.px;
    let (x1, x2, x4) = (x0 + k, x0 + 2.5 * k, (x0 + x8) / 2.0);
    let (x6, x7) = (x8 - 2.5 * k, x8 - k);
    let (y1, y2, y3) = (
        y0 + sign * 2.0 * k,
        y0 + sign * 4.5 * k,
        y0 + sign * 7.0 * k,
    );
    let (y4, y5) = (y6 - sign * 6.0 * k, y6 - sign * k);
    let x3 = (x0 + x4) / 2.0;
    let x5 = (x4 + x8) / 2.0;
    let mut path = Path::new();
    path.move_to(x4, y6);
    path.bezier_to(x3, y5, x0, y4, x0, y3);
    path.bezier_to(x0, y2, x1, y1, x2, y0);
    path.line_to(x6, y0);
    path.bezier_to(x7, y1, x8, y2, x8, y3);
    path.bezier_to(x8, y4, x5, y5, x4, y6);
    path.close();
    Ok(op(&path, &s))
}

#[cfg(test)]
mod tests;
