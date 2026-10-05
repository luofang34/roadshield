use roadshield::{ShieldDef, ShieldOptions};

use super::*;

fn spec(keys: &[&str]) -> ShieldSpec {
    ShieldSpec {
        networks: keys
            .iter()
            .map(|k| ((*k).to_owned(), Some(ShieldDef::default())))
            .collect(),
        options: ShieldOptions {
            banner_text_color: "black".into(),
            banner_text_halo_color: "white".into(),
            banner_height: 9.0,
            banner_padding: 1.0,
            shield_font: String::new(),
            shield_size: 20.0,
        },
    }
}

#[test]
fn reports_both_definitions_of_a_redefined_network() {
    let upstream = spec(&["BAB", "e-road"]);
    let ext = ExtensionSpec {
        networks: [(
            "BAB".to_owned(),
            Some(ShieldDef {
                text_color: Some("white".into()),
                ..ShieldDef::default()
            }),
        )]
        .into_iter()
        .collect(),
    };
    let conflicts = extension_conflicts(&upstream, &ext);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].network, "BAB");
    assert_eq!(conflicts[0].extension["textColor"], "white");
    assert_eq!(conflicts[0].upstream, serde_json::json!({}));
    assert!(!conflicts[0].identical);
}

#[test]
fn new_networks_do_not_conflict() {
    let ext = ExtensionSpec {
        networks: [("AH".to_owned(), Some(ShieldDef::default()))]
            .into_iter()
            .collect(),
    };
    assert!(extension_conflicts(&spec(&["e-road"]), &ext).is_empty());
}
