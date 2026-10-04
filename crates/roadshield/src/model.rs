//! Serde model of Americana ShieldJSON.
//!
//! Every struct rejects unknown fields so that upstream schema additions fail
//! loudly at pack load instead of being silently ignored.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Top-level ShieldJSON document (`shields.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShieldSpec {
    /// Network key → shield definition. Order matches the upstream document,
    /// which matters for `bannerMap` expansion collisions.
    pub networks: IndexMap<String, Option<ShieldDef>>,
    /// Global rendering options.
    pub options: ShieldOptions,
}

/// Global options shared by every shield.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ShieldOptions {
    /// Default banner text colour.
    pub banner_text_color: String,
    /// Default banner text halo colour.
    pub banner_text_halo_color: String,
    /// Height of one banner row, in 1x pixels.
    pub banner_height: f64,
    /// Gap between banner rows, in 1x pixels.
    pub banner_padding: f64,
    /// CSS font stack used upstream; informational, the pack font stack is
    /// authoritative.
    pub shield_font: String,
    /// Nominal shield height (and minimum width), in 1x pixels.
    pub shield_size: f64,
}

/// A shield definition for one network (or one override).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ShieldDef {
    /// SVG blank artwork, either one ID or a list ordered by text width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_blank: Option<SpriteBlank>,
    /// Programmatically drawn background shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape_blank: Option<ShapeBlank>,
    /// Text colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    /// Text halo colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_halo_color: Option<String>,
    /// Banner text colour override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_text_color: Option<String>,
    /// Banner halo colour override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_text_halo_color: Option<String>,
    /// Minimum padding around the text, in 1x pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<Padding>,
    /// Text fitting constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_layout: Option<TextLayoutDef>,
    /// Banner rows drawn above the shield, top first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banners: Option<Vec<String>>,
    /// Additional network keys that are bannered variants of this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_map: Option<IndexMap<String, Vec<String>>>,
    /// Draw no text on this shield.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notext: Option<bool>,
    /// Upper bound for the fitted font size, in 1x pixels (capped at 14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_font_size: Option<f64>,
    /// Route name → text to draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refs_by_name: Option<IndexMap<String, String>>,
    /// Hard-coded shield text.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,
    /// Alternative numbering system for the text (`"roman"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbering_system: Option<String>,
    /// Flip the blank artwork vertically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_reflect: Option<bool>,
    /// Colour that black in the blank is mapped to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_lighten: Option<String>,
    /// Colour that white in the blank is mapped to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_darken: Option<String>,
    /// Per-name partial overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_by_name: Option<IndexMap<String, ShieldDef>>,
    /// Per-ref partial overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_by_ref: Option<IndexMap<String, ShieldDef>>,
    /// Replacement definition used when the route has no valid ref.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noref: Option<Box<ShieldDef>>,
}

/// One blank ID or a width-ordered list of blank IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpriteBlank {
    /// A single blank.
    One(String),
    /// Blanks whose `_N` suffix states the ideal character count.
    Many(Vec<String>),
}

impl SpriteBlank {
    /// All blank IDs referenced.
    pub fn ids(&self) -> Vec<&str> {
        match self {
            Self::One(id) => vec![id.as_str()],
            Self::Many(ids) => ids.iter().map(String::as_str).collect(),
        }
    }
}

/// A programmatically drawn shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ShapeBlank {
    /// Shape function name.
    pub draw_func: String,
    /// Shape parameters.
    pub params: ShapeParams,
}

/// Shape parameters; absent values take each shape's upstream default.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ShapeParams {
    /// Fill colour (default white).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<String>,
    /// Outline colour (default black).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    /// Fixed width; variable width from the text when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect_width: Option<f64>,
    /// Corner radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    /// Radius of the pointed-side corners.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius1: Option<f64>,
    /// Radius of the flat-side corners.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius2: Option<f64>,
    /// Distance from the top/bottom edge to the side vertices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_offset: Option<f64>,
    /// Outline width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_width: Option<f64>,
    /// Point the shape upward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_up: Option<bool>,
    /// Put the short side on top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_side_up: Option<bool>,
    /// Side deviation from vertical, in radians.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side_angle: Option<f64>,
}

/// Box padding in 1x pixels; missing sides are zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Padding {
    /// Left padding.
    #[serde(default)]
    pub left: f64,
    /// Right padding.
    #[serde(default)]
    pub right: f64,
    /// Top padding.
    #[serde(default)]
    pub top: f64,
    /// Bottom padding.
    #[serde(default)]
    pub bottom: f64,
}

/// Text fitting constraint reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TextLayoutDef {
    /// Constraint function name.
    pub constraint_func: String,
    /// Constraint options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<TextLayoutOptions>,
}

/// Options for text constraints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextLayoutOptions {
    /// Corner radius of a rounded-rectangle constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
}

impl ShieldDef {
    /// JS object spread `{...self, ...over}`: fields present in `over` win.
    pub fn overlay(&self, over: &ShieldDef) -> ShieldDef {
        macro_rules! pick {
            ($($f:ident),*) => { ShieldDef { $($f: over.$f.clone().or_else(|| self.$f.clone()),)* } };
        }
        pick!(
            sprite_blank,
            shape_blank,
            text_color,
            text_halo_color,
            banner_text_color,
            banner_text_halo_color,
            padding,
            text_layout,
            banners,
            banner_map,
            notext,
            max_font_size,
            refs_by_name,
            ref_,
            numbering_system,
            vertical_reflect,
            color_lighten,
            color_darken,
            override_by_name,
            override_by_ref,
            noref
        )
    }
}

impl ShieldSpec {
    /// Expands every `bannerMap` into extra network entries, as the upstream
    /// renderer does when it loads ShieldJSON.
    ///
    /// Iterates a snapshot of the original entries, so a definition that is
    /// overwritten by an earlier expansion still expands its own map.
    pub fn expand_banner_maps(&mut self) {
        let snapshot: Vec<ShieldDef> = self.networks.values().flatten().cloned().collect();
        for def in snapshot {
            let Some(map) = &def.banner_map else { continue };
            for (key, banners) in map {
                let mut variant = def.clone();
                variant.banners = Some(banners.clone());
                self.networks.insert(key.clone(), Some(variant));
            }
        }
    }
}

#[cfg(test)]
mod tests;
