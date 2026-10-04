//! Final SVG document and symbol metadata from a composition.

use crate::compose::Composed;
use crate::error::Warning;
use crate::geometry::{Rect, num};
use crate::route::DisplayContext;
use crate::select::Selection;
use crate::svg::escape;
use crate::symbol::{Dependency, Provenance, RuleInfo, ShieldSymbol, TextInfo};

/// Accessible label: banners then shield text, else the network.
fn label(c: &Composed, network: &str) -> String {
    let mut parts: Vec<&str> = c.banners.iter().map(|b| b.text.as_str()).collect();
    if let Some(t) = &c.text {
        parts.push(&t.text);
    }
    let joined = parts.join(" ");
    if joined.trim().is_empty() {
        network.to_owned()
    } else {
        joined
    }
}

/// Wraps the composed body in the root `<svg>` element.
pub(crate) fn svg(c: &Composed, rule_key: &str, network: &str, ctx: &DisplayContext) -> String {
    let label = escape(&label(c, network));
    let (a11y, title) = if ctx.accessibility.omit_title {
        (" aria-hidden=\"true\"".to_owned(), String::new())
    } else {
        (
            format!(" role=\"img\" aria-label=\"{label}\""),
            format!("<title>{label}</title>"),
        )
    };
    // Geometry is in device pixels on the layout grid; the document's size
    // stays in logical pixels.
    let s = ctx.scale / f64::from(ctx.pixel_grid);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"{a11y} data-rule=\"{}\">{title}{}</svg>",
        num(c.width * s),
        num(c.height * s),
        num(c.width),
        num(c.height),
        escape(rule_key),
        c.body
    )
}

/// Metadata computed by the engine rather than the composition.
pub(crate) struct Extras {
    pub svg: String,
    pub provenance: Provenance,
    pub dependencies: Vec<Dependency>,
    pub semantic_key: String,
    pub warnings: Vec<Warning>,
}

/// Builds the public symbol; `s` converts composed (device) units to
/// output logical pixels.
pub(crate) fn symbol(c: Composed, sel: Selection, s: f64, extras: Extras) -> ShieldSymbol {
    let scale_info = |mut t: TextInfo| {
        t.font_px *= s;
        t.ink = t.ink.scaled(s);
        t
    };
    let shield_box = Rect::new(0.0, c.banner_area, c.width, c.shield_height).scaled(s);
    ShieldSymbol {
        svg: extras.svg,
        width: c.width * s,
        height: c.height * s,
        view_box: Rect::new(0.0, 0.0, c.width, c.height),
        anchor: (
            shield_box.x + shield_box.width / 2.0,
            shield_box.y + shield_box.height / 2.0,
        ),
        shield_box,
        text: c.text.map(scale_info),
        banners: c.banners.into_iter().map(scale_info).collect(),
        rule: RuleInfo {
            rule_key: sel.rule_key,
            fallback: sel.fallback,
            overrides: sel.overrides,
            blank: c.blank,
            shape: c.shape,
        },
        provenance: extras.provenance,
        dependencies: extras.dependencies,
        semantic_key: extras.semantic_key,
        contains_raster: c.raster,
        warnings: extras.warnings,
    }
}
