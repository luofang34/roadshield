use super::*;

#[test]
fn extracts_registered_names() {
    let src = r#"registerDrawFunction("diamond", diamond);
registerDrawFunction("pill", pill);
ShieldDraw.registerDrawFunction("branson", bransonRoute, 20);"#;
    let names = quoted_after(src, "registerDrawFunction(\"");
    assert_eq!(
        names.into_iter().collect::<Vec<_>>(),
        ["branson", "diamond", "pill"]
    );
}

#[test]
fn extracts_interface_fields_skipping_comments() {
    let src = "export interface BoxPadding {\n  /** Minimum padding */\n  left: number;\n  right?: number;\n  // top: number;\n}\ninterface Other {\n  nope: string;\n}";
    let fields = interface_fields(src, "BoxPadding");
    assert_eq!(fields.into_iter().collect::<Vec<_>>(), ["left", "right"]);
}

#[test]
fn known_fields_cover_the_serde_model() {
    let defs = known_def_fields();
    for f in [
        "spriteBlank",
        "shapeBlank",
        "overrideByRef",
        "noref",
        "ref",
        "refsByName",
        "colorLighten",
    ] {
        assert!(defs.contains(f), "{f}");
    }
    assert!(known_param_fields().contains("sideAngle"));
}

#[test]
fn pinned_checkout_matches_engine() {
    let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.upstream/openstreetmap-americana");
    if !checkout.is_dir() {
        // Needs `scripts/fetch-upstream.sh`. CI always fetches, so this branch
        // is a local convenience, not coverage.
        eprintln!("skipping: {} not fetched", checkout.display());
        return;
    }
    let inv = inventory_blocking(&checkout).unwrap();
    assert!(inv.missing.is_empty(), "{:?}", inv.issues());
}
