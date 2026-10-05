//! Resource packs: rules, blank SVGs, fonts and licences with a manifest
//! that pins every file by BLAKE3 hash.
//!
//! The engine never touches the filesystem or network; callers supply bytes
//! through a [`ResourceResolver`].

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::error::PackError;
use crate::model::{ExtensionSpec, ShieldSpec};

/// Manifest format version understood by this engine.
pub const MANIFEST_FORMAT: u32 = 1;
/// Manifest path inside a pack.
pub const MANIFEST_PATH: &str = "manifest.json";
const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;

/// Supplies pack files by relative path.
pub trait ResourceResolver {
    /// Returns the file's bytes, or `None` if absent.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
}

impl<S: std::hash::BuildHasher> ResourceResolver for HashMap<String, Vec<u8>, S> {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.get(path).cloned()
    }
}

/// A file listed in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    /// Relative path.
    pub path: String,
    /// BLAKE3 hex digest.
    pub blake3: String,
    /// Size in bytes.
    pub bytes: usize,
}

/// External origin of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    /// Where it was obtained.
    pub url: String,
    /// SHA-256 hex digest of the original bytes.
    pub sha256: String,
}

/// A blank SVG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlankEntry {
    /// Blank ID referenced by rules.
    pub id: String,
    /// File.
    pub file: FileRef,
    /// Sprite width in 1x pixels.
    pub width: f64,
    /// Sprite height in 1x pixels.
    pub height: f64,
    /// SPDX licence ID.
    pub license: String,
}

/// A font.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontEntry {
    /// Font ID used in stacks.
    pub id: String,
    /// Family name.
    pub family: String,
    /// File (TrueType/OpenType).
    pub file: FileRef,
    /// SPDX licence ID.
    pub license: String,
    /// Original download.
    pub source: Option<SourceRef>,
}

/// A licence text and what it covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicenseEntry {
    /// SPDX licence ID.
    pub id: String,
    /// Licence text file.
    pub file: FileRef,
    /// Components covered (`rules`, `blanks`, font IDs).
    pub applies_to: Vec<String>,
    /// Attribution notice.
    pub attribution: String,
}

/// Upstream provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upstream {
    /// Upstream repository URL.
    pub repository: String,
    /// Pinned commit.
    pub commit: String,
    /// Where the `ShieldJSON` came from.
    pub rules_source: SourceRef,
    /// SHA-256 of the upstream engine sources this port mirrors, so pack
    /// diffs flag upstream logic changes that need porting.
    pub engine_sources: Vec<UpstreamFile>,
}

/// An upstream source file pinned by hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamFile {
    /// Path relative to the upstream repository root.
    pub path: String,
    /// SHA-256 hex digest.
    pub sha256: String,
}

/// Subset description: networks deliberately left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subset {
    /// Content hash of the full pack this was cut from.
    pub parent_content_hash: String,
    /// Network keys present upstream but excluded here.
    pub excluded_networks: Vec<String>,
}

/// Pack manifest (`manifest.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Manifest format version.
    pub format: u32,
    /// Pack ID, e.g. `americana`.
    pub id: String,
    /// BLAKE3 of the manifest with this field empty; identifies content.
    pub content_hash: String,
    /// Upstream provenance.
    pub upstream: Upstream,
    /// `ShieldJSON` file.
    pub rules: FileRef,
    /// Blank SVGs.
    pub blanks: Vec<BlankEntry>,
    /// Fonts.
    pub fonts: Vec<FontEntry>,
    /// Default font stack (font IDs).
    pub font_stack: Vec<String>,
    /// Licences.
    pub licenses: Vec<LicenseEntry>,
    /// Themes provided; the first is the default.
    pub themes: Vec<String>,
    /// Present for subset packs.
    pub subset: Option<Subset>,
    /// Networks the pack adds beyond upstream (`ExtensionSpec` JSON).
    /// Omitted when absent, so packs without extensions keep their hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension_rules: Option<FileRef>,
}

impl Manifest {
    /// Computes the content hash (manifest serialised with an empty
    /// `content_hash`).
    ///
    /// # Errors
    ///
    /// Fails with [`PackError::Manifest`] if the manifest cannot be serialised.
    pub fn compute_content_hash(&self) -> Result<String, PackError> {
        let mut m = self.clone();
        m.content_hash = String::new();
        let bytes = serde_json::to_vec(&m).map_err(|e| PackError::Manifest {
            detail: e.to_string(),
        })?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }

    /// Every file the manifest lists.
    #[must_use]
    pub fn files(&self) -> Vec<&FileRef> {
        let mut out = vec![&self.rules];
        out.extend(self.extension_rules.iter());
        out.extend(self.blanks.iter().map(|b| &b.file));
        out.extend(self.fonts.iter().map(|f| &f.file));
        out.extend(self.licenses.iter().map(|l| &l.file));
        out
    }
}

/// A verified, loaded pack.
#[derive(Debug, Clone)]
pub struct ResourcePack {
    /// The manifest.
    pub manifest: Manifest,
    /// Upstream rules with `bannerMap` expanded, followed by extension
    /// networks.
    pub rules: ShieldSpec,
    /// Network keys that come from the extension rules.
    pub extension_networks: BTreeSet<String>,
    /// Blank ID → SVG bytes.
    pub blanks: HashMap<String, Vec<u8>>,
    /// Font ID → font bytes.
    pub fonts: HashMap<String, Vec<u8>>,
}

fn read_verified(resolver: &dyn ResourceResolver, f: &FileRef) -> Result<Vec<u8>, PackError> {
    let bytes = resolver
        .read(&f.path)
        .ok_or_else(|| PackError::MissingFile {
            path: f.path.clone(),
        })?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(PackError::TooLarge {
            path: f.path.clone(),
            size: bytes.len(),
            limit: MAX_FILE_BYTES,
        });
    }
    let actual = blake3::hash(&bytes).to_hex().to_string();
    if actual != f.blake3 {
        return Err(PackError::HashMismatch {
            path: f.path.clone(),
            expected: f.blake3.clone(),
            actual,
        });
    }
    Ok(bytes)
}

impl ResourcePack {
    /// Loads and verifies a pack. Every listed file must be present and
    /// match its hash; the manifest must match its content hash.
    ///
    /// # Errors
    ///
    /// Fails with [`PackError`] when the manifest or a listed file is missing,
    /// oversized or does not match its hash, the manifest format is unknown or
    /// inconsistent, or the rules are not valid `ShieldJSON` (including unknown
    /// fields).
    pub fn load(resolver: &dyn ResourceResolver) -> Result<Self, PackError> {
        let raw = resolver
            .read(MANIFEST_PATH)
            .ok_or_else(|| PackError::MissingFile {
                path: MANIFEST_PATH.into(),
            })?;
        let manifest: Manifest = serde_json::from_slice(&raw).map_err(|e| PackError::Json {
            path: MANIFEST_PATH.into(),
            detail: e.to_string(),
        })?;
        if manifest.format != MANIFEST_FORMAT {
            return Err(PackError::Manifest {
                detail: format!("format {} is not {MANIFEST_FORMAT}", manifest.format),
            });
        }
        let computed = manifest.compute_content_hash()?;
        if computed != manifest.content_hash {
            return Err(PackError::HashMismatch {
                path: MANIFEST_PATH.into(),
                expected: manifest.content_hash.clone(),
                actual: computed,
            });
        }
        if manifest.font_stack.is_empty() || manifest.themes.is_empty() {
            return Err(PackError::Manifest {
                detail: "font_stack and themes must be non-empty".into(),
            });
        }
        let rules_bytes = read_verified(resolver, &manifest.rules)?;
        let mut rules: ShieldSpec =
            serde_json::from_slice(&rules_bytes).map_err(|e| PackError::Json {
                path: manifest.rules.path.clone(),
                detail: e.to_string(),
            })?;
        rules.expand_banner_maps();
        let extension_networks = match &manifest.extension_rules {
            None => BTreeSet::new(),
            Some(file) => {
                let bytes = read_verified(resolver, file)?;
                let ext: ExtensionSpec =
                    serde_json::from_slice(&bytes).map_err(|e| PackError::Json {
                        path: file.path.clone(),
                        detail: e.to_string(),
                    })?;
                crate::extension::merge(&mut rules.networks, &ext)
                    .map_err(|networks| PackError::ExtensionConflict { networks })?
            }
        };
        let mut blanks = HashMap::new();
        for b in &manifest.blanks {
            blanks.insert(b.id.clone(), read_verified(resolver, &b.file)?);
        }
        let mut fonts = HashMap::new();
        for f in &manifest.fonts {
            fonts.insert(f.id.clone(), read_verified(resolver, &f.file)?);
        }
        for l in &manifest.licenses {
            read_verified(resolver, &l.file)?;
        }
        for id in &manifest.font_stack {
            if !fonts.contains_key(id) {
                return Err(PackError::Manifest {
                    detail: format!("font stack names unknown font {id:?}"),
                });
            }
        }
        Ok(Self {
            manifest,
            rules,
            extension_networks,
            blanks,
            fonts,
        })
    }
}

#[cfg(test)]
mod tests;
