//! Rule validation: every field value a definition uses must be one the
//! engine implements, and every referenced blank must exist.

use std::collections::HashMap;

use serde::Serialize;

use crate::color::Rgba;
use crate::model::{ShieldDef, ShieldSpec};
use crate::shapes::SHAPES;
use crate::text_layout::CONSTRAINTS;

/// Category of a rule problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    /// Field value the engine does not implement.
    Unsupported,
    /// Referenced blank is not in the pack.
    MissingBlank,
    /// Malformed value.
    Invalid,
}

/// One problem in one definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleIssue {
    /// Network key.
    pub network: String,
    /// Path within the definition, e.g. `overrideByRef.66`.
    pub path: String,
    /// Category.
    pub kind: IssueKind,
    /// Human-readable detail.
    pub detail: String,
}

/// Checks one resolved definition; used at render time too.
pub fn check_def(def: &ShieldDef, blank_exists: &dyn Fn(&str) -> bool) -> Vec<(IssueKind, String)> {
    let mut out = Vec::new();
    let mut color = |field: &str, v: &Option<String>| {
        if let Some(s) = v.as_ref().filter(|s| !s.is_empty())
            && Rgba::parse(s).is_none()
        {
            out.push((IssueKind::Invalid, format!("{field}: invalid colour {s:?}")));
        }
    };
    color("textColor", &def.text_color);
    color("textHaloColor", &def.text_halo_color);
    color("bannerTextColor", &def.banner_text_color);
    color("bannerTextHaloColor", &def.banner_text_halo_color);
    color("colorLighten", &def.color_lighten);
    color("colorDarken", &def.color_darken);
    if let Some(shape) = &def.shape_blank {
        color("shapeBlank.params.fillColor", &shape.params.fill_color);
        color("shapeBlank.params.strokeColor", &shape.params.stroke_color);
        if !SHAPES.contains(&shape.draw_func.as_str()) {
            out.push((
                IssueKind::Unsupported,
                format!("shapeBlank.drawFunc {:?}", shape.draw_func),
            ));
        }
    }
    if let Some(tl) = &def.text_layout
        && !CONSTRAINTS.contains(&tl.constraint_func.as_str())
    {
        out.push((
            IssueKind::Unsupported,
            format!("textLayout.constraintFunc {:?}", tl.constraint_func),
        ));
    }
    if let Some(ns) = &def.numbering_system
        && ns != "roman"
    {
        out.push((IssueKind::Unsupported, format!("numberingSystem {ns:?}")));
    }
    if let Some(blank) = &def.sprite_blank {
        let ids = blank.ids();
        if ids.is_empty() {
            out.push((IssueKind::Invalid, "spriteBlank is an empty list".into()));
        }
        for id in ids {
            if !blank_exists(id) {
                out.push((IssueKind::MissingBlank, format!("spriteBlank {id:?}")));
            }
        }
    }
    out
}

fn walk(
    network: &str,
    path: &str,
    def: &ShieldDef,
    blank_exists: &dyn Fn(&str) -> bool,
    out: &mut Vec<RuleIssue>,
) {
    for (kind, detail) in check_def(def, blank_exists) {
        out.push(RuleIssue {
            network: network.into(),
            path: path.into(),
            kind,
            detail,
        });
    }
    let nested = |label: &str, key: &str| {
        if path.is_empty() {
            format!("{label}.{key}")
        } else {
            format!("{path}.{label}.{key}")
        }
    };
    for (key, d) in def.override_by_ref.iter().flatten() {
        walk(network, &nested("overrideByRef", key), d, blank_exists, out);
    }
    for (key, d) in def.override_by_name.iter().flatten() {
        walk(
            network,
            &nested("overrideByName", key),
            d,
            blank_exists,
            out,
        );
    }
    if let Some(d) = &def.noref {
        let p = if path.is_empty() {
            "noref".to_owned()
        } else {
            format!("{path}.noref")
        };
        walk(network, &p, d, blank_exists, out);
    }
}

/// Validates every definition in `spec`.
pub fn validate(spec: &ShieldSpec, blanks: &HashMap<String, impl Sized>) -> Vec<RuleIssue> {
    let exists = |id: &str| blanks.contains_key(id);
    let mut out = Vec::new();
    for (network, def) in &spec.networks {
        if let Some(def) = def {
            walk(network, "", def, &exists, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests;
