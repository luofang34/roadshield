//! Structured diagnostics.

use serde::Serialize;

/// Why the rules decided that no shield is drawn. This is a normal outcome,
/// not an error: the map should show no symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NoShieldReason {
    /// The descriptor has no network.
    NoNetwork,
    /// Unknown network and no valid ref to show on the generic fallback.
    UnknownNetworkWithoutRef {
        /// The unknown network.
        network: String,
    },
    /// The rule needs text but the ref is missing, empty or too long.
    InvalidRef {
        /// The network whose rule was selected.
        network: String,
        /// Length of the ref in UTF-16 code units (0 when absent).
        utf16_len: usize,
    },
    /// The pack's rule for this network is explicitly null.
    NetworkDisabled {
        /// The network.
        network: String,
    },
}

/// Hard failures. A blank image is never returned in place of one of these.
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShieldError {
    /// No rule for the network and the policy forbids the generic fallback.
    #[error("no shield rule for network {network:?}")]
    UnknownNetwork {
        /// The unknown network.
        network: String,
    },
    /// The network exists upstream but this pack is a subset without it.
    #[error("network {network:?} is not included in pack subset {pack:?}")]
    NetworkNotInPack {
        /// The excluded network.
        network: String,
        /// Pack ID.
        pack: String,
    },
    /// A blank referenced by the selected rule is missing from the pack.
    #[error("blank {blank:?} required by network {network:?} is missing from the pack")]
    MissingBlank {
        /// Network whose rule needs the blank.
        network: String,
        /// Missing blank ID.
        blank: String,
    },
    /// No font in the stack has a glyph for a character.
    #[error("no font in stack {font_stack:?} covers U+{codepoint:04X} in {text:?}")]
    MissingGlyph {
        /// The text being drawn.
        text: String,
        /// The uncovered code point.
        codepoint: u32,
        /// Font IDs that were tried.
        font_stack: Vec<String>,
    },
    /// The rule uses a feature this engine does not implement.
    #[error("network {network:?} uses unsupported rule feature: {detail}")]
    UnsupportedRule {
        /// Network whose rule is unsupported.
        network: String,
        /// What is unsupported.
        detail: String,
    },
    /// The descriptor or context is malformed or out of bounds.
    #[error("invalid input field {field}: {reason}")]
    InvalidInput {
        /// Offending field.
        field: String,
        /// Why it was rejected.
        reason: String,
    },
    /// Required resources are not loaded, or a different pack is loaded.
    #[error("resources not ready: {detail}")]
    ResourceNotReady {
        /// What is missing or mismatched.
        detail: String,
    },
    /// Rendering failed after a rule was selected.
    #[error("rendering network {network:?} failed: {detail}")]
    Render {
        /// Network being rendered.
        network: String,
        /// Failure detail.
        detail: String,
    },
}

/// Non-fatal observations attached to a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Warning {
    /// The generic `default` rule was used for an unknown network.
    GenericFallback {
        /// The unknown network.
        network: String,
    },
    /// A `.notdef` glyph was drawn under [`crate::MissingGlyphPolicy::Notdef`].
    NotdefDrawn {
        /// The uncovered code point.
        codepoint: u32,
    },
    /// A blank contains raster image content that colour operations cannot
    /// change and that is not vector data.
    RasterContent {
        /// The blank ID.
        blank: String,
    },
}

/// Problems found when loading or validating a pack.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PackError {
    /// A pack file is missing.
    #[error("pack file {path:?} is missing")]
    MissingFile {
        /// Path within the pack.
        path: String,
    },
    /// A pack file does not match its manifest hash.
    #[error("pack file {path:?} hash mismatch: manifest {expected}, actual {actual}")]
    HashMismatch {
        /// Path within the pack.
        path: String,
        /// Manifest hash.
        expected: String,
        /// Computed hash.
        actual: String,
    },
    /// JSON could not be parsed (includes unknown `ShieldJSON` fields).
    #[error("{path:?} is not valid: {detail}")]
    Json {
        /// Path within the pack.
        path: String,
        /// Parser message.
        detail: String,
    },
    /// A font could not be parsed.
    #[error("font {id:?} could not be parsed")]
    Font {
        /// Font ID.
        id: String,
    },
    /// A blank SVG could not be parsed or exceeds limits.
    #[error("blank {id:?} is invalid: {detail}")]
    Blank {
        /// Blank ID.
        id: String,
        /// Failure detail.
        detail: String,
    },
    /// The manifest is internally inconsistent.
    #[error("manifest is invalid: {detail}")]
    Manifest {
        /// Failure detail.
        detail: String,
    },
    /// Extension rules redefine networks the upstream rules already define.
    /// Upstream adopted them: remove or reconcile the extension.
    #[error("extension rules redefine upstream networks: {}", networks.join(", "))]
    ExtensionConflict {
        /// Networks both define.
        networks: Vec<String>,
    },
    /// A resource exceeds a size limit.
    #[error("{path:?} is {size} bytes, over the {limit} byte limit")]
    TooLarge {
        /// Path within the pack.
        path: String,
        /// Actual size.
        size: usize,
        /// Limit.
        limit: usize,
    },
}
