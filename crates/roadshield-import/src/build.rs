//! Builds a self-contained pack from a pinned upstream checkout and pinned
//! input files. Never runs upstream code.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use roadshield::{
    BlankEntry, FileRef, FontEntry, LicenseEntry, MANIFEST_FORMAT, MANIFEST_PATH, Manifest,
    ShieldDef, ShieldSpec, SourceRef, Upstream, UpstreamFile,
};
use serde::Serialize;

use crate::config::ImportConfig;
use crate::error::ImportError;
use crate::files::{read_blocking, read_pinned_blocking, sha256_hex, write_blocking};
use crate::inventory::{Inventory, inventory_blocking};

/// Inputs to [`build_pack_blocking`].
pub struct BuildRequest<'a> {
    /// Import configuration.
    pub config: &'a ImportConfig,
    /// Upstream checkout at the pinned commit.
    pub checkout: &'a Path,
    /// Directory holding the pinned input files.
    pub inputs: &'a Path,
    /// Output pack directory (replaced if it already holds a pack).
    pub out_dir: &'a Path,
}

/// What a build produced.
#[derive(Debug, Clone, Serialize)]
pub struct BuildReport {
    /// Pack ID.
    pub pack_id: String,
    /// Content hash.
    pub content_hash: String,
    /// Network rules before `bannerMap` expansion.
    pub networks: usize,
    /// Blanks included (the closure of rule references).
    pub blanks: usize,
    /// Fonts included.
    pub fonts: usize,
    /// Source inventory (stale names are informational).
    pub inventory: Inventory,
}

fn checkout_commit_blocking(checkout: &Path) -> Result<String, ImportError> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|source| ImportError::Io {
            op: "run git rev-parse in",
            path: checkout.to_path_buf(),
            source,
        })?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn collect_blanks(def: &ShieldDef, out: &mut BTreeSet<String>) {
    if let Some(b) = &def.sprite_blank {
        out.extend(b.ids().into_iter().map(str::to_owned));
    }
    for d in def
        .override_by_ref
        .iter()
        .chain(def.override_by_name.iter())
        .flat_map(|m| m.values())
    {
        collect_blanks(d, out);
    }
    if let Some(d) = &def.noref {
        collect_blanks(d, out);
    }
}

fn file_ref(path: &str, bytes: &[u8]) -> FileRef {
    FileRef {
        path: path.into(),
        blake3: blake3::hash(bytes).to_hex().to_string(),
        bytes: bytes.len(),
    }
}

/// Pack files staged in memory before writing.
#[derive(Default)]
struct Staged {
    files: BTreeMap<String, Vec<u8>>,
}

impl Staged {
    fn add(&mut self, path: &str, bytes: Vec<u8>) -> FileRef {
        let r = file_ref(path, &bytes);
        self.files.insert(path.into(), bytes);
        r
    }
}

fn sprite_sizes(bytes: &[u8], path: &Path) -> Result<BTreeMap<String, (f64, f64)>, ImportError> {
    #[derive(serde::Deserialize)]
    struct Entry {
        width: f64,
        height: f64,
    }
    let map: BTreeMap<String, Entry> =
        serde_json::from_slice(bytes).map_err(|e| ImportError::Json {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })?;
    Ok(map
        .into_iter()
        .map(|(k, e)| (k, (e.width, e.height)))
        .collect())
}

fn intrinsic_size(id: &str, svg: &[u8]) -> Result<(f64, f64), ImportError> {
    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).map_err(|e| {
        ImportError::BlankSize {
            id: id.into(),
            detail: e.to_string(),
        }
    })?;
    let s = tree.size();
    Ok((f64::from(s.width()).round(), f64::from(s.height()).round()))
}

fn stage_blanks(
    req: &BuildRequest<'_>,
    spec: &ShieldSpec,
    st: &mut Staged,
) -> Result<Vec<BlankEntry>, ImportError> {
    let c = req.config;
    let sprite_path = req.inputs.join(&c.sprite_sizes.file);
    let sizes = sprite_sizes(
        &read_pinned_blocking(&sprite_path, &c.sprite_sizes.sha256)?,
        &sprite_path,
    )?;
    let mut ids = BTreeSet::new();
    for def in spec.networks.values().flatten() {
        collect_blanks(def, &mut ids);
    }
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| {
            !req.checkout
                .join("icons")
                .join(format!("{id}.svg"))
                .is_file()
        })
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(ImportError::MissingBlanks { missing });
    }
    let mut entries = Vec::new();
    for id in ids {
        let svg = read_blocking(&req.checkout.join("icons").join(format!("{id}.svg")))?;
        let intrinsic = intrinsic_size(&id, &svg)?;
        let (width, height) = match sizes.get(&id) {
            Some(&sprite) if sprite != intrinsic => {
                return Err(ImportError::BlankSize {
                    id,
                    detail: format!(
                        "sprite sheet says {sprite:?}, SVG intrinsic size is {intrinsic:?}"
                    ),
                });
            }
            _ => intrinsic,
        };
        let file = st.add(&format!("blanks/{id}.svg"), svg);
        entries.push(BlankEntry {
            id,
            file,
            width,
            height,
            license: c.blank_license.id.clone(),
        });
    }
    Ok(entries)
}

fn stage_fonts(
    req: &BuildRequest<'_>,
    st: &mut Staged,
) -> Result<(Vec<FontEntry>, Vec<LicenseEntry>), ImportError> {
    let mut fonts = Vec::new();
    let mut licenses = Vec::new();
    for f in &req.config.fonts {
        let path = req.inputs.join(&f.file);
        let raw = read_pinned_blocking(&path, &f.sha256)?;
        let ttf = if raw.starts_with(b"wOF2") {
            wuff::decompress_woff2(&raw).map_err(|e| ImportError::Font {
                path: path.clone(),
                detail: format!("{e:?}"),
            })?
        } else {
            raw
        };
        let file = st.add(&format!("fonts/{}.ttf", f.id), ttf);
        fonts.push(FontEntry {
            id: f.id.clone(),
            family: f.family.clone(),
            file,
            license: f.license.id.clone(),
            source: Some(SourceRef {
                url: f.url.clone(),
                sha256: f.sha256.clone(),
            }),
        });
        let lic_path = req.inputs.join(&f.license.file);
        let text = read_pinned_blocking(&lic_path, &f.license.sha256)?;
        licenses.push(LicenseEntry {
            id: f.license.id.clone(),
            file: st.add(&format!("licenses/{}", f.license.file), text),
            applies_to: vec![f.id.clone()],
            attribution: f.license.attribution.clone(),
        });
    }
    Ok((fonts, licenses))
}

fn manifest_json(manifest: &Manifest, out_dir: &Path) -> Result<Vec<u8>, ImportError> {
    let mut json = serde_json::to_vec_pretty(manifest).map_err(|e| ImportError::Json {
        path: out_dir.join(MANIFEST_PATH),
        detail: e.to_string(),
    })?;
    json.push(b'\n');
    Ok(json)
}

/// Loads the staged files through the engine before anything is written.
fn staged_engine(st: &Staged, manifest: &Manifest) -> Result<roadshield::Engine, ImportError> {
    let mut files: HashMap<String, Vec<u8>> = st.files.clone().into_iter().collect();
    files.insert(
        MANIFEST_PATH.into(),
        manifest_json(manifest, Path::new(""))?,
    );
    let pack = roadshield::ResourcePack::load(&files)?;
    Ok(roadshield::Engine::new(pack)?)
}

fn write_pack(out_dir: &Path, st: &Staged, manifest: &Manifest) -> Result<(), ImportError> {
    if out_dir.join(MANIFEST_PATH).is_file() {
        std::fs::remove_dir_all(out_dir).map_err(|source| ImportError::Io {
            op: "replace pack directory",
            path: out_dir.to_path_buf(),
            source,
        })?;
    }
    for (path, bytes) in &st.files {
        write_blocking(&out_dir.join(path), bytes)?;
    }
    write_blocking(
        &out_dir.join(MANIFEST_PATH),
        &manifest_json(manifest, out_dir)?,
    )
}

fn stage_blank_license(
    req: &BuildRequest<'_>,
    st: &mut Staged,
    licenses: &mut Vec<LicenseEntry>,
) -> Result<(), ImportError> {
    let c = req.config;
    let text = read_blocking(&req.checkout.join(&c.blank_license.upstream_path))?;
    licenses.insert(
        0,
        LicenseEntry {
            id: c.blank_license.id.clone(),
            file: st.add(&format!("licenses/{}.txt", c.blank_license.id), text),
            applies_to: vec!["rules".into(), "blanks".into()],
            attribution: c.blank_license.attribution.clone(),
        },
    );
    Ok(())
}

fn hash_engine_sources(req: &BuildRequest<'_>) -> Result<Vec<UpstreamFile>, ImportError> {
    req.config
        .engine_sources
        .iter()
        .map(|rel| {
            Ok(UpstreamFile {
                path: rel.clone(),
                sha256: sha256_hex(&read_blocking(&req.checkout.join(rel))?),
            })
        })
        .collect()
}

/// Verifies every input, builds the pack, writes it and reloads it through
/// the engine. Fails on any unimplemented upstream semantics.
pub fn build_pack_blocking(req: &BuildRequest<'_>) -> Result<BuildReport, ImportError> {
    let c = req.config;
    let commit = checkout_commit_blocking(req.checkout)?;
    if commit != c.upstream.commit {
        return Err(ImportError::Commit {
            path: req.checkout.to_path_buf(),
            expected: c.upstream.commit.clone(),
            actual: commit,
        });
    }
    let inventory = inventory_blocking(req.checkout)?;
    if !inventory.missing.is_empty() {
        return Err(ImportError::Incompatible {
            issues: inventory.issues(),
        });
    }
    let rules_path = req.inputs.join(&c.rules.file);
    let rules_bytes = read_pinned_blocking(&rules_path, &c.rules.sha256)?;
    let spec: ShieldSpec = serde_json::from_slice(&rules_bytes).map_err(|e| ImportError::Json {
        path: rules_path.clone(),
        detail: e.to_string(),
    })?;
    let mut st = Staged::default();
    let rules = st.add("rules/shields.json", rules_bytes);
    let blanks = stage_blanks(req, &spec, &mut st)?;
    let (fonts, mut licenses) = stage_fonts(req, &mut st)?;
    stage_blank_license(req, &mut st, &mut licenses)?;
    let engine_sources = hash_engine_sources(req)?;
    let mut manifest = Manifest {
        format: MANIFEST_FORMAT,
        id: c.pack_id.clone(),
        content_hash: String::new(),
        upstream: Upstream {
            repository: c.upstream.repository.clone(),
            commit: c.upstream.commit.clone(),
            rules_source: SourceRef {
                url: c.rules.url.clone(),
                sha256: c.rules.sha256.clone(),
            },
            engine_sources,
        },
        rules,
        blanks,
        fonts,
        font_stack: c.font_stack.clone(),
        licenses,
        themes: c.themes.clone(),
        subset: None,
    };
    manifest.content_hash = manifest.compute_content_hash()?;
    let engine = staged_engine(&st, &manifest)?;
    let issues: Vec<String> = engine
        .validate()
        .into_iter()
        .map(|i| format!("{} {}: {:?} {}", i.network, i.path, i.kind, i.detail))
        .collect();
    if !issues.is_empty() {
        return Err(ImportError::Incompatible { issues });
    }
    write_pack(req.out_dir, &st, &manifest)?;
    Ok(BuildReport {
        pack_id: manifest.id.clone(),
        content_hash: manifest.content_hash.clone(),
        networks: spec.networks.len(),
        blanks: manifest.blanks.len(),
        fonts: manifest.fonts.len(),
        inventory,
    })
}
