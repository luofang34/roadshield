//! Fixed-size custom shapes from `custom_shields.ts`.

use super::{DrawOp, ShapeEnv, rounded_rectangle, style};
use crate::geometry::Path;
use crate::model::ShapeParams;

fn fixed_square(
    env: &ShapeEnv<'_>,
    fill: &str,
    stroke: &str,
) -> Result<DrawOp, crate::ShieldError> {
    let p = ShapeParams {
        fill_color: Some(fill.into()),
        stroke_color: Some(stroke.into()),
        outline_width: Some(env.px),
        radius: Some(2.0 * env.px),
        rect_width: Some(20.0 * env.px),
        ..ShapeParams::default()
    };
    rounded_rectangle(env, &p)
}

pub(super) fn pa_belt(
    env: &ShapeEnv<'_>,
    p: &ShapeParams,
) -> Result<Vec<DrawOp>, crate::ShieldError> {
    let base = fixed_square(env, "white", "black")?;
    let lw = 0.5 * env.px;
    let radius = env.size / 3.0 - lw;
    let mut path = Path::new();
    path.ellipse(env.size / 2.0, env.size / 2.0, radius, radius);
    let s = style(p);
    Ok(vec![
        base,
        DrawOp {
            d: path.data().to_owned(),
            fill: s.fill,
            stroke: Some((s.outline, lw)),
        },
    ])
}

pub(super) fn branson(
    env: &ShapeEnv<'_>,
    p: &ShapeParams,
) -> Result<Vec<DrawOp>, crate::ShieldError> {
    let base = fixed_square(env, "#006747", "white")?;
    let lw = 0.5 * env.px;
    let mut path = Path::new();
    path.rect(
        0.15 * env.size + lw,
        0.4 * env.size + lw,
        0.7 * env.size - 2.0 * lw,
        0.45 * env.size - 2.0 * lw,
    );
    let s = style(p);
    Ok(vec![
        base,
        DrawOp {
            d: path.data().to_owned(),
            fill: s.fill,
            stroke: Some((s.outline, lw)),
        },
    ])
}
