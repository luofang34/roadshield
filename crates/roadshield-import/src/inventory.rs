//! Upstream source inventory: compares what the pinned TypeScript engine
//! declares (shape functions, text constraints, definition fields,
//! numbering systems) with what roadshield implements.

use std::collections::BTreeSet;
use std::path::Path;

use roadshield::{Padding, ShapeParams, ShieldDef, ShieldOptions, TextLayoutOptions};
use serde::Serialize;

use crate::error::ImportError;
use crate::files::read_blocking;

/// Names declared upstream versus implemented here, per category.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Inventory {
    /// Category → names upstream declares that roadshield lacks.
    pub missing: Vec<(String, Vec<String>)>,
    /// Category → names roadshield knows that upstream no longer declares.
    pub stale: Vec<(String, Vec<String>)>,
}

impl Inventory {
    /// One line per missing name, for error reporting.
    pub fn issues(&self) -> Vec<String> {
        self.missing
            .iter()
            .flat_map(|(cat, names)| {
                names
                    .iter()
                    .map(move |n| format!("upstream {cat} {n:?} is not implemented"))
            })
            .collect()
    }
}

fn quoted_after(src: &str, marker: &str) -> BTreeSet<String> {
    src.match_indices(marker)
        .filter_map(|(i, _)| {
            let rest = src.get(i + marker.len()..)?;
            let end = rest.find('"')?;
            rest.get(..end).map(str::to_owned)
        })
        .collect()
}

fn interface_fields(src: &str, name: &str) -> BTreeSet<String> {
    let header = format!("interface {name} {{");
    let Some(start) = src.find(&header) else {
        return BTreeSet::new();
    };
    let body = src.get(start + header.len()..).unwrap_or("");
    let body = body
        .get(..body.find("\n}").unwrap_or(body.len()))
        .unwrap_or("");
    body.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.starts_with("//") || l.starts_with('*') || l.starts_with("/*") {
                return None;
            }
            let colon = l.find(':')?;
            let field = l.get(..colon)?.trim_end_matches('?');
            field
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
                .then(|| field.to_owned())
        })
        .filter(|f| !f.is_empty())
        .collect()
}

fn serde_keys<T: Serialize>(value: &T) -> BTreeSet<String> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        _ => BTreeSet::new(),
    }
}

/// Every ShieldJSON definition field the engine models.
pub fn known_def_fields() -> BTreeSet<String> {
    let s = || Some(String::new());
    let full = ShieldDef {
        sprite_blank: Some(roadshield::SpriteBlank::One(String::new())),
        shape_blank: Some(roadshield::ShapeBlank {
            draw_func: String::new(),
            params: ShapeParams::default(),
        }),
        text_color: s(),
        text_halo_color: s(),
        banner_text_color: s(),
        banner_text_halo_color: s(),
        padding: Some(Padding::default()),
        text_layout: Some(roadshield::TextLayoutDef {
            constraint_func: String::new(),
            options: None,
        }),
        banners: Some(Vec::new()),
        banner_map: Some(Default::default()),
        notext: Some(false),
        max_font_size: Some(0.0),
        refs_by_name: Some(Default::default()),
        ref_: s(),
        numbering_system: s(),
        vertical_reflect: Some(false),
        color_lighten: s(),
        color_darken: s(),
        override_by_name: Some(Default::default()),
        override_by_ref: Some(Default::default()),
        noref: Some(Box::default()),
    };
    serde_keys(&full)
}

/// Every shape parameter the engine models.
pub fn known_param_fields() -> BTreeSet<String> {
    let f = Some(0.0);
    let s = || Some(String::new());
    serde_keys(&ShapeParams {
        fill_color: s(),
        stroke_color: s(),
        rect_width: f,
        radius: f,
        radius1: f,
        radius2: f,
        y_offset: f,
        outline_width: f,
        point_up: Some(false),
        short_side_up: Some(false),
        side_angle: f,
    })
}

fn known_option_fields() -> BTreeSet<String> {
    serde_keys(&ShieldOptions {
        banner_text_color: String::new(),
        banner_text_halo_color: String::new(),
        banner_height: 0.0,
        banner_padding: 0.0,
        shield_font: String::new(),
        shield_size: 0.0,
    })
}

fn compare(cat: &str, upstream: &BTreeSet<String>, ours: &BTreeSet<String>, inv: &mut Inventory) {
    let missing: Vec<String> = upstream.difference(ours).cloned().collect();
    let stale: Vec<String> = ours.difference(upstream).cloned().collect();
    if !missing.is_empty() {
        inv.missing.push((cat.into(), missing));
    }
    if !stale.is_empty() {
        inv.stale.push((cat.into(), stale));
    }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// Reads the pinned checkout and compares declarations.
pub fn inventory_blocking(checkout: &Path) -> Result<Inventory, ImportError> {
    let read = |rel: &str| -> Result<String, ImportError> {
        Ok(String::from_utf8_lossy(&read_blocking(&checkout.join(rel))?).into_owned())
    };
    let draw =
        read("shieldlib/src/shield_canvas_draw.ts")? + &read("shieldlib/src/custom_shields.ts")?;
    let text = read("shieldlib/src/shield_text.ts")?;
    let shield = read("shieldlib/src/shield.ts")?;
    let types = read("shieldlib/src/types.ts")?;
    let mut inv = Inventory::default();
    compare(
        "shape",
        &quoted_after(&draw, "registerDrawFunction(\""),
        &set(roadshield::SHAPES),
        &mut inv,
    );
    compare(
        "text constraint",
        &quoted_after(&text, "registerDrawTextFunction(\""),
        &set(roadshield::CONSTRAINTS),
        &mut inv,
    );
    compare(
        "numbering system",
        &quoted_after(&shield, "numberingSystem === \""),
        &set(&["roman"]),
        &mut inv,
    );
    let mut def_fields = interface_fields(&types, "ShieldDefinitionBase");
    def_fields.extend(["spriteBlank".to_owned(), "shapeBlank".to_owned()]);
    compare(
        "definition field",
        &def_fields,
        &known_def_fields(),
        &mut inv,
    );
    compare(
        "shape parameter",
        &interface_fields(&types, "ShapeBlankParams"),
        &known_param_fields(),
        &mut inv,
    );
    compare(
        "option",
        &interface_fields(&types, "ShieldOptions"),
        &known_option_fields(),
        &mut inv,
    );
    compare(
        "padding field",
        &interface_fields(&types, "BoxPadding"),
        &serde_keys(&Padding::default()),
        &mut inv,
    );
    let layout_opts = serde_keys(&TextLayoutOptions { radius: Some(0.0) });
    compare(
        "text layout option",
        &interface_fields(&types, "TextLayoutParameters"),
        &layout_opts,
        &mut inv,
    );
    Ok(inv)
}

#[cfg(test)]
mod tests;
