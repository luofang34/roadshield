//! Minimal SVG serialisation helpers.

use crate::color::Rgba;
use crate::geometry::num;

/// Escapes text for XML content and attribute values.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// `fill="…"` (plus `fill-opacity` when translucent) for a parsed colour.
pub fn fill(c: Rgba) -> String {
    paint("fill", c)
}

/// `stroke="…"` (plus `stroke-opacity` when translucent).
pub fn stroke(c: Rgba) -> String {
    paint("stroke", c)
}

fn paint(attr: &str, c: Rgba) -> String {
    if c.0[3] == 255 {
        format!("{attr}=\"{}\"", c.hex())
    } else {
        format!(
            "{attr}=\"{}\" {attr}-opacity=\"{}\"",
            c.hex(),
            num(c.opacity())
        )
    }
}

/// Canvas stroke defaults (miter joins, limit 10, butt caps).
pub fn stroke_style(width: f64) -> String {
    format!(
        "stroke-width=\"{}\" stroke-linejoin=\"miter\" stroke-miterlimit=\"10\"",
        num(width)
    )
}
