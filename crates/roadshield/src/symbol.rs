//! Engine outputs.

use serde::Serialize;

use crate::error::{NoShieldReason, Warning};
use crate::extension::RuleOrigin;
use crate::geometry::Rect;
use crate::select::AppliedOverride;

/// Result of a successful render call.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Rendering {
    /// A symbol to display.
    Symbol(Box<ShieldSymbol>),
    /// The rules say no symbol is shown.
    NoShield {
        /// Why.
        reason: NoShieldReason,
        /// Semantic cache key, so negative results can be cached too.
        semantic_key: String,
    },
}

impl Rendering {
    /// The symbol, if one was produced.
    #[must_use]
    pub fn symbol(&self) -> Option<&ShieldSymbol> {
        match self {
            Self::Symbol(s) => Some(s),
            Self::NoShield { .. } => None,
        }
    }

    /// Semantic cache key of this result.
    #[must_use]
    pub fn semantic_key(&self) -> &str {
        match self {
            Self::Symbol(s) => &s.semantic_key,
            Self::NoShield { semantic_key, .. } => semantic_key,
        }
    }
}

/// A rendered shield with layout metadata. Boxes are in output (scaled)
/// logical pixels with the origin at the top-left of the SVG.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShieldSymbol {
    /// Self-contained SVG document.
    pub svg: String,
    /// Output width in logical pixels.
    pub width: f64,
    /// Output height in logical pixels.
    pub height: f64,
    /// SVG viewBox (unscaled, Americana 1x pixels).
    pub view_box: Rect,
    /// The shield body, excluding banners.
    pub shield_box: Rect,
    /// Suggested anchor: centre of the shield body.
    pub anchor: (f64, f64),
    /// Shield text, when drawn.
    pub text: Option<TextInfo>,
    /// Banner rows, top first.
    pub banners: Vec<TextInfo>,
    /// Which rule produced this symbol.
    pub rule: RuleInfo,
    /// Versions of everything used.
    pub provenance: Provenance,
    /// Resources this symbol depends on.
    pub dependencies: Vec<Dependency>,
    /// Deterministic key over all inputs that affect the output.
    pub semantic_key: String,
    /// True if some content is raster data rather than vectors.
    pub contains_raster: bool,
    /// Non-fatal observations.
    pub warnings: Vec<Warning>,
}

/// Drawn text and where its ink is.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextInfo {
    /// Original text (also in the SVG `<title>`).
    pub text: String,
    /// Font size in output pixels.
    pub font_px: f64,
    /// Ink bounds in output pixels (empty for whitespace).
    pub ink: Rect,
}

/// Rule selection details.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuleInfo {
    /// Network key of the rule used.
    pub rule_key: String,
    /// True if the generic fallback was used.
    pub fallback: bool,
    /// Overrides applied, in order.
    pub overrides: Vec<AppliedOverride>,
    /// Blank used, if any.
    pub blank: Option<String>,
    /// Shape drawn, if any.
    pub shape: Option<String>,
    /// Whether the rule is upstream's or one of the pack's extensions.
    pub origin: RuleOrigin,
}

/// Versions used to produce a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Provenance {
    /// Engine output version.
    pub engine: String,
    /// Pack ID.
    pub pack_id: String,
    /// Pack content hash.
    pub pack_content_hash: String,
    /// Upstream commit of the rules.
    pub upstream_commit: String,
    /// Font stack used.
    pub font_stack: Vec<String>,
}

/// Kind of resource a symbol depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// `ShieldJSON` rules.
    Rules,
    /// Blank SVG.
    Blank,
    /// Font file.
    Font,
}

/// One resource dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dependency {
    /// Kind.
    pub kind: DependencyKind,
    /// Pack ID of the resource.
    pub id: String,
    /// BLAKE3 of the resource file.
    pub blake3: String,
}
