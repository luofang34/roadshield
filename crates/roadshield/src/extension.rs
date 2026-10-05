//! Extension rules: networks a pack defines beyond its upstream rules.
//!
//! Upstream rules stay byte-identical to the upstream generator's output;
//! extensions live in their own file and are merged at load time. A
//! network that upstream already defines (directly or through a
//! `bannerMap`) is a conflict, never an override: when upstream adopts a
//! network, the extension must be removed or reconciled.

use std::collections::BTreeSet;

use indexmap::IndexMap;
use serde::Serialize;

use crate::model::{ExtensionSpec, ShieldDef, expand_banner_maps};

/// Where the rule that drew a symbol comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleOrigin {
    /// The pinned upstream rules (including the generic `default` rule).
    Upstream,
    /// The pack's extension rules.
    Extension,
}

/// Network keys an extension defines after `bannerMap` expansion.
#[must_use]
pub fn extension_networks(ext: &ExtensionSpec) -> IndexMap<String, Option<ShieldDef>> {
    let mut networks = ext.networks.clone();
    expand_banner_maps(&mut networks);
    networks
}

/// Extension networks that upstream (already expanded) also defines.
#[must_use]
pub fn conflicts(
    upstream: &IndexMap<String, Option<ShieldDef>>,
    extension: &IndexMap<String, Option<ShieldDef>>,
) -> Vec<String> {
    extension
        .keys()
        .filter(|k| upstream.contains_key(*k))
        .cloned()
        .collect()
}

/// Appends extension networks to `upstream`, returning the keys added, or
/// the conflicting keys.
///
/// # Errors
///
/// Returns the sorted list of networks both sides define.
pub fn merge(
    upstream: &mut IndexMap<String, Option<ShieldDef>>,
    ext: &ExtensionSpec,
) -> Result<BTreeSet<String>, Vec<String>> {
    let networks = extension_networks(ext);
    let mut clashes = conflicts(upstream, &networks);
    if !clashes.is_empty() {
        clashes.sort();
        return Err(clashes);
    }
    let keys = networks.keys().cloned().collect();
    upstream.extend(networks);
    Ok(keys)
}

#[cfg(test)]
mod tests;
