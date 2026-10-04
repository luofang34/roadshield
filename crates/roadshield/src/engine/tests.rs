use super::*;
use crate::testing::{edit_manifest, pack_files};
use crate::{PackExpectation, Subset};

fn engine_from(files: &HashMap<String, Vec<u8>>) -> Engine {
    Engine::new(ResourcePack::load(files).unwrap()).unwrap()
}

#[test]
fn missing_blank_is_an_error_not_an_empty_image() {
    let mut files = pack_files();
    edit_manifest(&mut files, |m| {
        m.blanks
            .retain(|b| !b.id.starts_with("shield_us_interstate_"))
    });
    let engine = engine_from(&files);
    let err = engine
        .render(
            &RouteDescriptor::new("US:I", "287"),
            &DisplayContext::default(),
        )
        .unwrap_err();
    assert!(
        matches!(err, ShieldError::MissingBlank { ref blank, .. } if blank == "shield_us_interstate_3"),
        "{err}"
    );
    assert!(
        engine
            .validate()
            .iter()
            .any(|i| i.network == "US:I" && i.kind == IssueKind::MissingBlank)
    );
}

#[test]
fn subset_exclusions_never_fall_back() {
    let mut files = pack_files();
    edit_manifest(&mut files, |m| {
        m.subset = Some(Subset {
            parent_content_hash: "parent".into(),
            excluded_networks: vec!["US:I".into()],
        });
    });
    let engine = engine_from(&files);
    let err = engine
        .render(
            &RouteDescriptor::new("US:I", "287"),
            &DisplayContext::default(),
        )
        .unwrap_err();
    assert!(matches!(err, ShieldError::NetworkNotInPack { .. }), "{err}");
}

#[test]
fn pack_expectations_and_themes_are_enforced() {
    let engine = engine_from(&pack_files());
    let route = RouteDescriptor::new("US:I", "287");
    let wrong = DisplayContext {
        expected_pack: Some(PackExpectation {
            id: "americana".into(),
            content_hash: Some("0".into()),
        }),
        ..DisplayContext::default()
    };
    assert!(matches!(
        engine.render(&route, &wrong),
        Err(ShieldError::ResourceNotReady { .. })
    ));
    let right = DisplayContext {
        expected_pack: Some(PackExpectation {
            id: "americana".into(),
            content_hash: Some(engine.manifest().content_hash.clone()),
        }),
        ..DisplayContext::default()
    };
    assert!(engine.render(&route, &right).is_ok());
    let theme = DisplayContext {
        theme: Some("night".into()),
        ..DisplayContext::default()
    };
    assert!(matches!(
        engine.render(&route, &theme),
        Err(ShieldError::InvalidInput { .. })
    ));
}

#[test]
fn unknown_font_ids_are_invalid_input() {
    let engine = engine_from(&pack_files());
    let ctx = DisplayContext {
        font_stack: Some(vec!["comic-sans".into()]),
        ..DisplayContext::default()
    };
    assert!(matches!(
        engine.render(&RouteDescriptor::new("US:I", "287"), &ctx),
        Err(ShieldError::InvalidInput { .. })
    ));
}
