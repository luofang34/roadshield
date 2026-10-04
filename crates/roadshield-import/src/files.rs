//! Blocking file access with path context in errors.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::ImportError;

/// Largest file `load_pack_dir_blocking` will read.
const MAX_PACK_FILE: u64 = 16 * 1024 * 1024;
/// Most files `load_pack_dir_blocking` will read.
const MAX_PACK_FILES: usize = 10_000;

/// Reads a whole file.
pub fn read_blocking(path: &Path) -> Result<Vec<u8>, ImportError> {
    std::fs::read(path).map_err(|source| ImportError::Io {
        op: "read",
        path: path.to_path_buf(),
        source,
    })
}

/// Writes a file, creating parent directories.
pub fn write_blocking(path: &Path, bytes: &[u8]) -> Result<(), ImportError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ImportError::Io {
            op: "create directory",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, bytes).map_err(|source| ImportError::Io {
        op: "write",
        path: path.to_path_buf(),
        source,
    })
}

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Reads a file and checks it against a pinned SHA-256.
pub fn read_pinned_blocking(path: &Path, sha256: &str) -> Result<Vec<u8>, ImportError> {
    let bytes = read_blocking(path)?;
    let actual = sha256_hex(&bytes);
    if actual != sha256 {
        return Err(ImportError::Checksum {
            path: path.to_path_buf(),
            expected: sha256.into(),
            actual,
        });
    }
    Ok(bytes)
}

fn walk(root: &Path, dir: &Path, out: &mut HashMap<String, Vec<u8>>) -> Result<(), ImportError> {
    let io = |op, path: &Path| {
        let path = path.to_path_buf();
        move |source| ImportError::Io { op, path, source }
    };
    for entry in std::fs::read_dir(dir).map_err(io("list", dir))? {
        let entry = entry.map_err(io("list", dir))?;
        let path = entry.path();
        let meta = entry.metadata().map_err(io("stat", &path))?;
        if meta.is_dir() {
            walk(root, &path, out)?;
        } else if meta.is_file() {
            if out.len() >= MAX_PACK_FILES || meta.len() > MAX_PACK_FILE {
                return Err(ImportError::Io {
                    op: "load",
                    path,
                    source: std::io::Error::other("pack directory exceeds size limits"),
                });
            }
            let rel: PathBuf = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            let key = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(key, read_blocking(&path)?);
        }
    }
    Ok(())
}

/// Loads every file under a pack directory, keyed by `/`-separated
/// relative path, for use as a [`roadshield::ResourceResolver`].
pub fn load_pack_dir_blocking(dir: &Path) -> Result<HashMap<String, Vec<u8>>, ImportError> {
    let mut out = HashMap::new();
    walk(dir, dir, &mut out)?;
    Ok(out)
}

/// Loads, verifies and prepares a pack directory.
pub fn load_engine_blocking(dir: &Path) -> Result<roadshield::Engine, ImportError> {
    let files = load_pack_dir_blocking(dir)?;
    let pack = roadshield::ResourcePack::load(&files)?;
    Ok(roadshield::Engine::new(pack)?)
}
