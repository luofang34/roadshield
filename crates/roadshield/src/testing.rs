//! Test support: loads the generated Americana pack from the repository.

use std::collections::HashMap;
use std::path::Path;

use crate::pack::{MANIFEST_PATH, Manifest};

fn walk(root: &Path, dir: &Path, out: &mut HashMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, std::fs::read(&path).unwrap());
        }
    }
}

/// Every file of `packs/americana`, keyed by relative path.
pub fn pack_files() -> HashMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana");
    let mut out = HashMap::new();
    walk(&root, &root, &mut out);
    out
}

/// Applies `edit` to the manifest and re-seals its content hash.
pub fn edit_manifest(files: &mut HashMap<String, Vec<u8>>, edit: impl FnOnce(&mut Manifest)) {
    let mut m: Manifest = serde_json::from_slice(&files[MANIFEST_PATH]).unwrap();
    edit(&mut m);
    m.content_hash = m.compute_content_hash().unwrap();
    files.insert(MANIFEST_PATH.into(), serde_json::to_vec(&m).unwrap());
}

/// Replaces a file and updates its manifest entry.
pub fn replace_file(files: &mut HashMap<String, Vec<u8>>, path: &str, bytes: Vec<u8>) {
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let len = bytes.len();
    files.insert(path.into(), bytes);
    edit_manifest(files, |m| {
        let mut all: Vec<&mut crate::pack::FileRef> = vec![&mut m.rules];
        all.extend(m.blanks.iter_mut().map(|b| &mut b.file));
        all.extend(m.fonts.iter_mut().map(|f| &mut f.file));
        for f in all.into_iter().filter(|f| f.path == path) {
            f.blake3 = hash.clone();
            f.bytes = len;
        }
    });
}
