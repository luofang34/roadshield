use super::*;
use crate::testing::{edit_manifest, pack_files, replace_file};

#[test]
fn loads_and_expands_the_americana_pack() {
    let pack = ResourcePack::load(&pack_files()).unwrap();
    assert_eq!(pack.manifest.id, "americana");
    assert!(pack.rules.networks.len() > 1900, "bannerMap expanded");
    assert!(pack.rules.networks.contains_key("US:US:Alternate"));
    assert_eq!(pack.blanks.len(), pack.manifest.blanks.len());
}

#[test]
fn tampered_files_fail_hash_checks() {
    let mut files = pack_files();
    let blank = files
        .keys()
        .find(|k| k.starts_with("blanks/"))
        .unwrap()
        .clone();
    files.get_mut(&blank).unwrap().push(b' ');
    assert!(
        matches!(ResourcePack::load(&files), Err(PackError::HashMismatch { path, .. }) if path == blank)
    );
}

#[test]
fn missing_files_are_reported() {
    let mut files = pack_files();
    files.remove("fonts/noto-sans-condensed-medium.ttf");
    assert!(matches!(
        ResourcePack::load(&files),
        Err(PackError::MissingFile { .. })
    ));
    files.remove(MANIFEST_PATH);
    assert!(
        matches!(ResourcePack::load(&files), Err(PackError::MissingFile { path }) if path == MANIFEST_PATH)
    );
}

#[test]
fn manifest_edits_without_resealing_are_detected() {
    let mut files = pack_files();
    let mut m: Manifest = serde_json::from_slice(&files[MANIFEST_PATH]).unwrap();
    m.id = "forged".into();
    files.insert(MANIFEST_PATH.into(), serde_json::to_vec(&m).unwrap());
    assert!(
        matches!(ResourcePack::load(&files), Err(PackError::HashMismatch { path, .. }) if path == MANIFEST_PATH)
    );
}

#[test]
fn unknown_rule_fields_fail_loading() {
    let mut files = pack_files();
    let mut rules: serde_json::Value =
        serde_json::from_slice(&files["rules/shields.json"]).unwrap();
    rules["networks"]["US:I"]["sparkle"] = serde_json::json!(true);
    replace_file(
        &mut files,
        "rules/shields.json",
        serde_json::to_vec(&rules).unwrap(),
    );
    let err = ResourcePack::load(&files).unwrap_err();
    assert!(
        matches!(&err, PackError::Json { detail, .. } if detail.contains("sparkle")),
        "{err}"
    );
}

#[test]
fn manifest_format_and_stack_are_checked() {
    let mut files = pack_files();
    edit_manifest(&mut files, |m| m.format = 99);
    assert!(matches!(
        ResourcePack::load(&files),
        Err(PackError::Manifest { .. })
    ));
    let mut files = pack_files();
    edit_manifest(&mut files, |m| m.font_stack = vec!["comic-sans".into()]);
    assert!(matches!(
        ResourcePack::load(&files),
        Err(PackError::Manifest { .. })
    ));
}

#[test]
fn extension_rules_merge_and_may_not_redefine_upstream() {
    let pack = ResourcePack::load(&pack_files()).unwrap();
    assert!(pack.extension_networks.contains("BAB") && pack.extension_networks.contains("AH"));
    assert!(pack.rules.networks.contains_key("BAB"));

    let mut files = pack_files();
    let mut ext: serde_json::Value =
        serde_json::from_slice(&files["rules/extensions.json"]).unwrap();
    ext["networks"]["e-road"] = ext["networks"]["AH"].clone();
    let bytes = serde_json::to_vec(&ext).unwrap();
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let len = bytes.len();
    files.insert("rules/extensions.json".into(), bytes);
    edit_manifest(&mut files, |m| {
        if let Some(f) = m.extension_rules.as_mut() {
            f.blake3 = hash;
            f.bytes = len;
        }
    });
    let err = ResourcePack::load(&files).unwrap_err();
    assert!(
        matches!(&err, PackError::ExtensionConflict { networks } if networks == &["e-road".to_owned()]),
        "{err}"
    );
}

#[test]
fn packs_without_extensions_keep_their_manifest_hash() {
    let mut files = pack_files();
    edit_manifest(&mut files, |m| m.extension_rules = None);
    let m: Manifest = serde_json::from_slice(&files[MANIFEST_PATH]).unwrap();
    let json = serde_json::to_string(&m).unwrap();
    assert!(
        !json.contains("extension_rules"),
        "absent field is not serialised"
    );
    let pack = ResourcePack::load(&files).unwrap();
    assert!(pack.extension_networks.is_empty());
    assert!(!pack.rules.networks.contains_key("BAB"));
}
