//! Text fitting, ported from `shield_text.ts`.
//!
//! Reproduces canvas `measureText` with `textAlign = "left"` and
//! `textBaseline = "top"`, including upstream's use of absolute values for
//! the bounding-box extents, so fitted sizes match the TS implementation.

use crate::font::ShapedText;
use crate::model::{Padding, TextLayoutDef};

/// Font size at which text is first measured; also the size below which a
/// wider blank would be preferred upstream.
pub const FONT_SIZE_THRESHOLD: f64 = 11.8;
/// Default and maximum fitted font size.
pub const DEFAULT_MAX_FONT_SIZE: f64 = 14.0;
/// Constraint names the engine implements.
pub const CONSTRAINTS: &[&str] = &[
    "diamond",
    "ellipse",
    "rect",
    "roundedRect",
    "southHalfEllipse",
    "triangleDown",
];

#[derive(Debug, Clone, Copy, PartialEq)]
struct Dim {
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VAlign {
    Middle,
    Top,
}

/// Where and how large to draw text (`textAlign = "center"`,
/// `textBaseline = "top"`).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct TextPlacement {
    /// Horizontal centre of the advance box.
    pub x_center: f64,
    /// Y of the em-box top.
    pub y_top: f64,
    /// Font size in px.
    pub font_px: f64,
}

/// `measureText` quantities at `size`, relative to a left/top alignment
/// point. Signs follow the canvas spec.
struct Measured {
    left: f64,
    right: f64,
    ascent: f64,
    descent: f64,
}

fn measure(text: &ShapedText, top_em: f64, size: f64) -> Measured {
    let top = crate::font::top_px(top_em, size);
    match text.ink {
        Some(ink) => Measured {
            left: -ink.min_x * size,
            right: ink.max_x * size,
            ascent: ink.max_y * size - top,
            descent: top - ink.min_y * size,
        },
        None => Measured {
            left: 0.0,
            right: 0.0,
            ascent: 0.0,
            descent: 0.0,
        },
    }
}

fn ellipse_scale(space: Dim, text: Dim) -> f64 {
    let (a, b, x0, y0) = (space.width, space.height, text.width, text.height);
    (a * b) / (a * a * y0 * y0 + b * b * x0 * x0).sqrt()
}

fn rect_scale(space: Dim, text: Dim) -> f64 {
    (space.width / text.width).min(space.height / text.height)
}

fn diamond_scale(space: Dim, text: Dim) -> f64 {
    let (a, b, x0, y0) = (space.width, space.height, text.width, text.height);
    (a * b) / (b * x0 + a * y0)
}

fn constrain(def: &TextLayoutDef, space: Dim, text: Dim) -> Option<(f64, VAlign)> {
    let out = match def.constraint_func.as_str() {
        "rect" => (rect_scale(space, text), VAlign::Middle),
        "roundedRect" => {
            let r = def.options.and_then(|o| o.radius).unwrap_or(2.0);
            let shrink = r * (2.0 - std::f64::consts::SQRT_2);
            let s = Dim {
                width: space.width - shrink,
                height: space.height - shrink,
            };
            (rect_scale(s, text), VAlign::Middle)
        }
        "ellipse" => (ellipse_scale(space, text), VAlign::Middle),
        "southHalfEllipse" => {
            let turned = Dim {
                height: text.width / 2.0,
                width: text.height,
            };
            (ellipse_scale(space, turned), VAlign::Top)
        }
        "diamond" => (diamond_scale(space, text), VAlign::Middle),
        "triangleDown" => (diamond_scale(space, text), VAlign::Top),
        _ => return None,
    };
    Some(out)
}

/// Error from [`layout`].
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutError {
    /// Constraint function is not implemented.
    UnknownConstraint(String),
    /// The fitted size is not a positive finite number.
    NoFit(f64),
}

/// `layoutShieldText`: fits `text` into `bounds` minus `padding`.
pub fn layout(
    text: &ShapedText,
    top_em: f64,
    padding: Padding,
    bounds: (f64, f64),
    def: &TextLayoutDef,
    max_font: f64,
) -> Result<TextPlacement, LayoutError> {
    let m = measure(text, top_em, FONT_SIZE_THRESHOLD);
    let measured = Dim {
        width: m.left.abs() + m.right.abs(),
        // Upstream trims excess descender height across browsers.
        height: (m.descent.abs() + m.ascent.abs()) * 0.9,
    };
    let avail = Dim {
        width: bounds.0 - padding.left - padding.right,
        height: bounds.1 - padding.top - padding.bottom,
    };
    let x_center = padding.left + avail.width / 2.0;
    let (scale, valign) = constrain(def, avail, measured)
        .ok_or_else(|| LayoutError::UnknownConstraint(def.constraint_func.clone()))?;
    let font_px = max_font.min(FONT_SIZE_THRESHOLD * scale);
    if !(font_px.is_finite() && font_px > 0.0) {
        return Err(LayoutError::NoFit(font_px));
    }
    let m = measure(text, top_em, font_px);
    let height = m.descent.abs() + m.ascent.abs();
    let y_top = match valign {
        VAlign::Top => padding.top + m.ascent,
        VAlign::Middle => padding.top + (avail.height - height) / 2.0,
    };
    Ok(TextPlacement {
        x_center,
        y_top,
        font_px,
    })
}

#[cfg(test)]
mod tests;
