//! Engine inputs: the route being symbolised and how it is displayed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::ShieldError;

/// Normalised route attributes. Network adapters (OpenMapTiles, Americana
/// sprite IDs, …) build this; the engine never guesses a network from a ref.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteDescriptor {
    /// Route network, e.g. `US:I` or `US:NJ:CR`.
    #[serde(default)]
    pub network: Option<String>,
    /// Route number or code as tagged.
    #[serde(default, rename = "ref")]
    pub ref_: Option<String>,
    /// Route name.
    #[serde(default)]
    pub name: Option<String>,
    /// Route colour tag. Not consulted by Americana rules; carried for
    /// diagnostics and future rule packs.
    #[serde(default)]
    pub colour: Option<String>,
    /// Other source attributes, kept for diagnostics.
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
    /// Where this descriptor came from (feature ID, sprite ID, …).
    #[serde(default)]
    pub source: Option<String>,
}

impl RouteDescriptor {
    /// Descriptor with network and ref set.
    pub fn new(network: impl Into<String>, ref_: impl Into<String>) -> Self {
        Self {
            network: Some(network.into()),
            ref_: Some(ref_.into()),
            ..Self::default()
        }
    }

    /// Sets the route name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Text direction for shaping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDirection {
    /// Derived from the text's strong characters.
    #[default]
    Auto,
    /// Left to right.
    Ltr,
    /// Right to left.
    Rtl,
}

/// What to do when the route's network has no rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownNetworkPolicy {
    /// Use the pack's explicit `default` rule (plain text with halo), as
    /// Americana does. The symbol records that a fallback was used.
    #[default]
    GenericFallback,
    /// Fail with [`ShieldError::UnknownNetwork`].
    Unsupported,
}

/// What to do when no font in the stack covers a character.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingGlyphPolicy {
    /// Fail with [`ShieldError::MissingGlyph`].
    #[default]
    Error,
    /// Draw the primary font's `.notdef` glyph and record a warning.
    Notdef,
}

/// Accessibility-related display options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accessibility {
    /// Omit the `<title>` / `aria-label` text description.
    #[serde(default)]
    pub omit_title: bool,
    /// Force a halo of this CSS colour around shield text.
    #[serde(default)]
    pub force_text_halo: Option<String>,
}

/// Pack identity the caller requires.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackExpectation {
    /// Required pack ID.
    pub id: String,
    /// Required content hash, if pinned.
    #[serde(default)]
    pub content_hash: Option<String>,
}

/// How a symbol is displayed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayContext {
    /// Logical scale relative to Americana's 1x pixel grid.
    #[serde(default = "one")]
    pub scale: f64,
    /// Device pixels per layout pixel used for upstream-compatible rounding
    /// (1 or 2, as MapLibre picks 1x or 2x sprites). Geometry is computed on
    /// this grid; the SVG's logical size is unchanged.
    #[serde(default = "one_u8")]
    pub pixel_grid: u8,
    /// Rule theme; `None` means the pack default.
    #[serde(default)]
    pub theme: Option<String>,
    /// BCP 47 language tag for shaping.
    #[serde(default)]
    pub language: Option<String>,
    /// Text direction.
    #[serde(default)]
    pub direction: TextDirection,
    /// Font IDs from the pack, in fallback order; `None` uses the pack stack.
    #[serde(default)]
    pub font_stack: Option<Vec<String>>,
    /// Required pack identity.
    #[serde(default)]
    pub expected_pack: Option<PackExpectation>,
    /// Accessibility options.
    #[serde(default)]
    pub accessibility: Accessibility,
    /// Unknown network handling.
    #[serde(default)]
    pub unknown_network: UnknownNetworkPolicy,
    /// Missing glyph handling.
    #[serde(default)]
    pub missing_glyph: MissingGlyphPolicy,
}

fn one() -> f64 {
    1.0
}

fn one_u8() -> u8 {
    1
}

impl Default for DisplayContext {
    fn default() -> Self {
        Self {
            scale: 1.0,
            pixel_grid: 1,
            theme: None,
            language: None,
            direction: TextDirection::Auto,
            font_stack: None,
            expected_pack: None,
            accessibility: Accessibility::default(),
            unknown_network: UnknownNetworkPolicy::GenericFallback,
            missing_glyph: MissingGlyphPolicy::Error,
        }
    }
}

/// Input size bounds enforced before any work is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputLimits {
    /// Maximum UTF-8 bytes of network, ref, name, colour.
    pub max_field_bytes: usize,
    /// Maximum number of `extra` entries.
    pub max_extra_entries: usize,
    /// Maximum font stack length.
    pub max_font_stack: usize,
}

impl Default for InputLimits {
    fn default() -> Self {
        Self {
            max_field_bytes: 512,
            max_extra_entries: 64,
            max_font_stack: 16,
        }
    }
}

impl InputLimits {
    /// Rejects descriptors and contexts outside these bounds.
    pub fn check(&self, route: &RouteDescriptor, ctx: &DisplayContext) -> Result<(), ShieldError> {
        let fields = [
            ("network", &route.network),
            ("ref", &route.ref_),
            ("name", &route.name),
            ("colour", &route.colour),
        ];
        for (field, value) in fields {
            if let Some(v) = value {
                if v.len() > self.max_field_bytes {
                    return Err(ShieldError::InvalidInput {
                        field: field.into(),
                        reason: format!("{} bytes exceeds limit {}", v.len(), self.max_field_bytes),
                    });
                }
                if v.chars().any(char::is_control) {
                    return Err(ShieldError::InvalidInput {
                        field: field.into(),
                        reason: "contains control characters".into(),
                    });
                }
            }
        }
        if route.extra.len() > self.max_extra_entries {
            return Err(ShieldError::InvalidInput {
                field: "extra".into(),
                reason: format!(
                    "{} entries exceeds limit {}",
                    route.extra.len(),
                    self.max_extra_entries
                ),
            });
        }
        if !(ctx.scale.is_finite() && ctx.scale > 0.0 && ctx.scale <= 64.0) {
            return Err(ShieldError::InvalidInput {
                field: "scale".into(),
                reason: format!("{} is not in (0, 64]", ctx.scale),
            });
        }
        if !matches!(ctx.pixel_grid, 1 | 2) {
            return Err(ShieldError::InvalidInput {
                field: "pixel_grid".into(),
                reason: format!("{} is not 1 or 2", ctx.pixel_grid),
            });
        }
        if ctx
            .font_stack
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > self.max_font_stack)
        {
            return Err(ShieldError::InvalidInput {
                field: "font_stack".into(),
                reason: format!("must hold 1..={} font IDs", self.max_font_stack),
            });
        }
        Ok(())
    }
}
