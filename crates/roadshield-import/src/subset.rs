//! Verifiable regional/network subsets of a full pack.
//!
//! A subset keeps the selected networks (plus their banner variants and the
//! `default` rule), the blanks they reference, and all fonts. Every other
//! network of the parent is listed as excluded, so the engine reports
//! `NetworkNotInPack` instead of falling back to a generic or wrong rule.

use std::collections::{BTreeSet, HashMap};

use roadshield::{MANIFEST_PATH, Manifest, ResourcePack, ShieldDef, ShieldSpec, Subset};

use crate::error::ImportError;

/// What to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsetRequest {
    /// Network keys or prefixes; `US:NJ` keeps `US:NJ` and `US:NJ:*`.
    pub networks: Vec<String>,
}

/// The generic fallback rule, always kept.
const DEFAULT_RULE: &str = "default";

fn matches(key: &str, prefixes: &[String]) -> bool {
    key == DEFAULT_RULE
        || prefixes
            .iter()
            .any(|p| key == p || key.starts_with(&format!("{p}:")))
}

fn blanks_of(def: &ShieldDef, out: &mut BTreeSet<String>) {
    if let Some(b) = &def.sprite_blank {
        out.extend(b.ids().into_iter().map(str::to_owned));
    }
    for d in def
        .override_by_ref
        .iter()
        .chain(def.override_by_name.iter())
        .flat_map(|m| m.values())
    {
        blanks_of(d, out);
    }
    if let Some(d) = &def.noref {
        blanks_of(d, out);
    }
}

fn file_ref(path: &str, bytes: &[u8]) -> roadshield::FileRef {
    roadshield::FileRef {
        path: path.into(),
        blake3: blake3::hash(bytes).to_hex().to_string(),
        bytes: bytes.len(),
    }
}

/// Cuts a subset from a full pack's files, returning the new pack's files
/// (manifest included). The result is verified by loading it.
///
/// # Errors
///
/// Fails with [`ImportError::Subset`] for a parent that is itself a subset,
/// missing files or a request that matches no network, and with
/// [`ImportError::Pack`] if the parent or the result does not verify.
pub fn cut_subset<S: std::hash::BuildHasher>(
    parent: &HashMap<String, Vec<u8>, S>,
    req: &SubsetRequest,
) -> Result<HashMap<String, Vec<u8>>, ImportError> {
    let full = ResourcePack::load(parent)?;
    if full.manifest.subset.is_some() {
        return Err(ImportError::Subset(
            "cannot cut a subset of a subset".into(),
        ));
    }
    let raw = parent
        .get(&full.manifest.rules.path)
        .ok_or_else(|| ImportError::Subset("rules file missing".into()))?;
    let mut spec: ShieldSpec =
        serde_json::from_slice(raw).map_err(|e| ImportError::Subset(e.to_string()))?;
    spec.networks.retain(|k, _| matches(k, &req.networks));
    if spec.networks.len() <= 1 {
        return Err(ImportError::Subset(format!(
            "no networks match {:?}",
            req.networks
        )));
    }
    let mut kept = spec.clone();
    kept.expand_banner_maps();
    let kept_keys: BTreeSet<&String> = kept.networks.keys().collect();
    let excluded: Vec<String> = full
        .rules
        .networks
        .keys()
        .filter(|k| !kept_keys.contains(k))
        .cloned()
        .collect();
    let mut blank_ids = BTreeSet::new();
    for def in spec.networks.values().flatten() {
        blanks_of(def, &mut blank_ids);
    }
    let rules_bytes = serde_json::to_vec(&spec).map_err(|e| ImportError::Subset(e.to_string()))?;
    let mut out: HashMap<String, Vec<u8>> = HashMap::new();
    let mut manifest: Manifest = full.manifest.clone();
    manifest.rules = file_ref(&manifest.rules.path, &rules_bytes);
    out.insert(manifest.rules.path.clone(), rules_bytes);
    manifest.blanks.retain(|b| blank_ids.contains(&b.id));
    let mut copy = |path: &str| -> Result<(), ImportError> {
        let bytes = parent
            .get(path)
            .ok_or_else(|| ImportError::Subset(format!("{path} missing")))?;
        out.insert(path.into(), bytes.clone());
        Ok(())
    };
    for b in &manifest.blanks {
        copy(&b.file.path)?;
    }
    for f in &manifest.fonts {
        copy(&f.file.path)?;
    }
    for l in &manifest.licenses {
        copy(&l.file.path)?;
    }
    manifest.subset = Some(Subset {
        parent_content_hash: full.manifest.content_hash.clone(),
        excluded_networks: excluded,
    });
    manifest.content_hash = manifest.compute_content_hash()?;
    let mut json =
        serde_json::to_vec_pretty(&manifest).map_err(|e| ImportError::Subset(e.to_string()))?;
    json.push(b'\n');
    out.insert(MANIFEST_PATH.into(), json);
    ResourcePack::load(&out)?;
    Ok(out)
}
