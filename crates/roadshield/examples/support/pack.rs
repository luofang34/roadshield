//! Pack loading shared by the examples. The engine itself does no I/O; the
//! examples read the pack directory named by `ROADSHIELD_PACK`, defaulting
//! to the repository's `packs/americana`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use roadshield::{Engine, ResourcePack};

/// Pack directory to load.
pub fn pack_dir() -> PathBuf {
    std::env::var_os("ROADSHIELD_PACK").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana"),
        PathBuf::from,
    )
}

/// Loads and verifies the pack and prepares an engine.
pub fn engine() -> Result<Engine, Box<dyn std::error::Error>> {
    let root = pack_dir();
    let mut files = HashMap::new();
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let key = path
                    .strip_prefix(&root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(key, std::fs::read(&path)?);
            }
        }
    }
    Ok(Engine::new(ResourcePack::load(&files)?)?)
}
