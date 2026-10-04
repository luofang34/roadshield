use std::collections::HashMap;

use super::*;
use crate::model::{ShapeBlank, ShapeParams, ShieldDef, SpriteBlank, TextLayoutDef};

#[test]
fn reports_unsupported_and_missing_semantics() {
    let def = ShieldDef {
        shape_blank: Some(ShapeBlank {
            draw_func: "hexagram".into(),
            params: ShapeParams::default(),
        }),
        text_layout: Some(TextLayoutDef {
            constraint_func: "spiral".into(),
            options: None,
        }),
        numbering_system: Some("greek".into()),
        text_color: Some("nope".into()),
        sprite_blank: Some(SpriteBlank::Many(vec!["here".into(), "gone".into()])),
        ..ShieldDef::default()
    };
    let issues = check_def(&def, &|id| id == "here");
    let kinds: Vec<IssueKind> = issues.iter().map(|i| i.0).collect();
    assert_eq!(
        kinds,
        vec![
            IssueKind::Invalid,
            IssueKind::Unsupported,
            IssueKind::Unsupported,
            IssueKind::Unsupported,
            IssueKind::MissingBlank
        ]
    );
}

#[test]
fn walks_nested_overrides() {
    let mut by_ref = indexmap::IndexMap::new();
    by_ref.insert(
        "66".to_owned(),
        ShieldDef {
            sprite_blank: Some(SpriteBlank::One("gone".into())),
            ..ShieldDef::default()
        },
    );
    let spec = ShieldSpec {
        networks: [(
            "US:US".to_owned(),
            Some(ShieldDef {
                override_by_ref: Some(by_ref),
                ..ShieldDef::default()
            }),
        )]
        .into_iter()
        .collect(),
        options: crate::model::ShieldOptions {
            banner_text_color: "black".into(),
            banner_text_halo_color: "white".into(),
            banner_height: 9.0,
            banner_padding: 1.0,
            shield_font: String::new(),
            shield_size: 20.0,
        },
    };
    let issues = validate(&spec, &HashMap::<String, ()>::new());
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].path, "overrideByRef.66");
    assert_eq!(issues[0].kind, IssueKind::MissingBlank);
}
