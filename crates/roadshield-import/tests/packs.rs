//! Subset and diff behaviour on the generated Americana pack.

use std::path::Path;

use roadshield::{DisplayContext, Rendering, ResourcePack, RouteDescriptor, ShieldError};
use roadshield_import::{
    SubsetRequest, cut_subset, diff_packs, load_pack_dir_blocking, visual_report_html,
};

#[cfg(test)]
fn pack_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana")
}

#[test]
fn subsets_keep_selected_networks_and_refuse_the_rest() {
    let full = load_pack_dir_blocking(&pack_dir()).unwrap();
    let files = cut_subset(
        &full,
        &SubsetRequest {
            networks: vec!["US:NJ".into(), "US:I".into()],
        },
    )
    .unwrap();
    let pack = ResourcePack::load(&files).unwrap();
    let subset = pack.manifest.subset.clone().unwrap();
    assert!(subset.excluded_networks.iter().any(|n| n == "US:US"));
    assert!(
        pack.manifest.blanks.len() < 30,
        "{}",
        pack.manifest.blanks.len()
    );
    let engine = roadshield::Engine::new(pack).unwrap();
    let ctx = DisplayContext::default();
    for (n, r) in [
        ("US:I", "287"),
        ("US:NJ:CR", "609"),
        ("US:I:Business:Loop", "80"),
    ] {
        assert!(
            matches!(
                engine.render(&RouteDescriptor::new(n, r), &ctx),
                Ok(Rendering::Symbol(_))
            ),
            "{n}"
        );
    }
    let err = engine
        .render(&RouteDescriptor::new("US:US", "22"), &ctx)
        .unwrap_err();
    assert!(matches!(err, ShieldError::NetworkNotInPack { .. }), "{err}");
    // Networks unknown to the parent still use the generic rule.
    assert!(
        engine
            .render(&RouteDescriptor::new("ZZ", "1"), &ctx)
            .is_ok()
    );
    assert!(
        cut_subset(
            &files,
            &SubsetRequest {
                networks: vec!["US:I".into()]
            }
        )
        .is_err(),
        "no nested subsets"
    );
    assert!(
        cut_subset(
            &full,
            &SubsetRequest {
                networks: vec!["Nowhere".into()]
            }
        )
        .is_err()
    );
}

#[test]
fn diff_of_subset_against_full_pack() {
    let full_files = load_pack_dir_blocking(&pack_dir()).unwrap();
    let full = roadshield::Engine::new(ResourcePack::load(&full_files).unwrap()).unwrap();
    let sub_files = cut_subset(
        &full_files,
        &SubsetRequest {
            networks: vec!["US:NJ".into()],
        },
    )
    .unwrap();
    let sub = roadshield::Engine::new(ResourcePack::load(&sub_files).unwrap()).unwrap();
    assert!(diff_packs(&full, &full).is_empty());
    let d = diff_packs(&full, &sub);
    assert!(d.networks.removed.contains(&"US:I".to_owned()));
    assert!(
        d.blanks
            .removed
            .contains(&"shield_us_interstate_3".to_owned())
    );
    assert!(d.fonts.added.is_empty() && d.engine_sources.changed.is_empty());
    let md = d.to_markdown();
    assert!(md.contains("## Networks") && md.contains("removed"));
    let rev = diff_packs(&sub, &full);
    let html = visual_report_html(&sub, &full, &rev);
    assert!(html.contains("data:image/svg+xml;base64,"));
}
