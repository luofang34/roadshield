//! Extension rules: loading, conflict detection against upstream, staging.

use std::path::Path;

use roadshield::{ExtensionSpec, ShieldSpec};
use serde::Serialize;
use serde_json::Value;

use crate::config::ImportConfig;
use crate::error::ImportError;
use crate::files::read_blocking;

/// A network both upstream and the extensions define.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExtensionConflict {
    /// Network key (after `bannerMap` expansion).
    pub network: String,
    /// Upstream's definition.
    pub upstream: Value,
    /// The extension's definition.
    pub extension: Value,
    /// Both definitions are equal: upstream adopted the extension as is,
    /// so deleting it changes nothing.
    pub identical: bool,
}

/// Loaded extension rules with their source bytes.
pub struct LoadedExtension {
    /// File bytes, staged into the pack unchanged.
    pub bytes: Vec<u8>,
    /// Parsed rules.
    pub spec: ExtensionSpec,
}

/// Reads the extension rules named by `config`, if any.
///
/// # Errors
///
/// Fails with [`ImportError::Io`] if the file cannot be read and
/// [`ImportError::Json`] if it is not valid `ExtensionSpec` JSON.
pub fn load_extension_blocking(
    config: &ImportConfig,
    config_dir: &Path,
) -> Result<Option<LoadedExtension>, ImportError> {
    let Some(ext) = &config.extensions else {
        return Ok(None);
    };
    let path = config_dir.join(&ext.file);
    let bytes = read_blocking(&path)?;
    let spec = serde_json::from_slice(&bytes).map_err(|e| ImportError::Json {
        path,
        detail: e.to_string(),
    })?;
    Ok(Some(LoadedExtension { bytes, spec }))
}

/// Networks the extension redefines, with both definitions, after
/// `bannerMap` expansion on both sides.
#[must_use]
pub fn extension_conflicts(upstream: &ShieldSpec, ext: &ExtensionSpec) -> Vec<ExtensionConflict> {
    let mut expanded = upstream.clone();
    expanded.expand_banner_maps();
    let mut ext_spec = ShieldSpec {
        networks: ext.networks.clone(),
        options: upstream.options.clone(),
    };
    ext_spec.expand_banner_maps();
    let json = |d: Option<&Option<roadshield::ShieldDef>>| {
        d.and_then(|d| serde_json::to_value(d).ok())
            .unwrap_or(Value::Null)
    };
    ext_spec
        .networks
        .keys()
        .filter(|k| expanded.networks.contains_key(*k))
        .map(|k| {
            let upstream = json(expanded.networks.get(k));
            let extension = json(ext_spec.networks.get(k));
            ExtensionConflict {
                network: k.clone(),
                identical: upstream == extension,
                upstream,
                extension,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
