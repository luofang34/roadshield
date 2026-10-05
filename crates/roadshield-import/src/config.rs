//! Import configuration (`packs/<id>.import.json`): every upstream input
//! with its URL and pinned SHA-256.

use serde::{Deserialize, Serialize};

/// A downloaded input file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedInput {
    /// File name inside the inputs directory.
    pub file: String,
    /// Download URL (recorded as provenance).
    pub url: String,
    /// Pinned SHA-256.
    pub sha256: String,
}

/// Upstream repository pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamPin {
    /// Repository URL.
    pub repository: String,
    /// Commit the checkout must be at.
    pub commit: String,
}

/// Licence for blanks and rules, taken from the upstream checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamLicense {
    /// SPDX ID.
    pub id: String,
    /// Path of the licence text in the checkout.
    pub upstream_path: String,
    /// Attribution notice.
    pub attribution: String,
}

/// Licence text downloaded alongside a font.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontLicense {
    /// SPDX ID.
    pub id: String,
    /// File name inside the inputs directory.
    pub file: String,
    /// Download URL.
    pub url: String,
    /// Pinned SHA-256.
    pub sha256: String,
    /// Attribution notice.
    pub attribution: String,
}

/// A font input (WOFF2 or TrueType).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontInput {
    /// Font ID used in stacks.
    pub id: String,
    /// Family name.
    pub family: String,
    /// File name inside the inputs directory.
    pub file: String,
    /// Download URL.
    pub url: String,
    /// Pinned SHA-256.
    pub sha256: String,
    /// Licence.
    pub license: FontLicense,
}

/// Extension rules maintained in this repository: networks the pack adds
/// beyond upstream. Their data must stay CC0 so it can be contributed
/// upstream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionInput {
    /// `ExtensionSpec` JSON, relative to the import config's directory.
    pub file: String,
    /// SPDX licence ID of the extension data (the licence text is taken from
    /// the upstream checkout's licence of the same ID).
    pub license: String,
    /// Attribution notice.
    pub attribution: String,
}

/// Whole import configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportConfig {
    /// Pack ID.
    pub pack_id: String,
    /// Themes the pack provides.
    pub themes: Vec<String>,
    /// Upstream repository pin.
    pub upstream: UpstreamPin,
    /// `ShieldJSON` input.
    pub rules: PinnedInput,
    /// Sprite sheet layout (1x) used to cross-check blank sizes.
    pub sprite_sizes: PinnedInput,
    /// Licence covering rules and blanks.
    pub blank_license: UpstreamLicense,
    /// Fonts.
    pub fonts: Vec<FontInput>,
    /// Default font stack.
    pub font_stack: Vec<String>,
    /// Upstream engine sources to pin by hash.
    pub engine_sources: Vec<String>,
    /// Networks added beyond upstream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<ExtensionInput>,
}
