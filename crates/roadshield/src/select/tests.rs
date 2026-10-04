use indexmap::IndexMap;

use super::*;
use crate::model::{ShieldDef, SpriteBlank};

fn route(network: &str, r: Option<&str>, name: Option<&str>) -> RouteDescriptor {
    RouteDescriptor {
        network: Some(network.into()),
        ref_: r.map(Into::into),
        name: name.map(Into::into),
        ..RouteDescriptor::default()
    }
}

fn blank(id: &str) -> Option<SpriteBlank> {
    Some(SpriteBlank::One(id.into()))
}

fn networks(defs: Vec<(&str, ShieldDef)>) -> IndexMap<String, Option<ShieldDef>> {
    let mut m: IndexMap<String, Option<ShieldDef>> = defs
        .into_iter()
        .map(|(k, v)| (k.to_owned(), Some(v)))
        .collect();
    m.insert(
        "default".into(),
        Some(ShieldDef {
            text_color: Some("black".into()),
            ..ShieldDef::default()
        }),
    );
    m
}

#[test]
fn valid_ref_counts_utf16_units() {
    assert!(is_valid_ref("1234567"));
    assert!(!is_valid_ref("12345678"));
    assert!(!is_valid_ref(""));
    assert!(is_valid_ref("😀😀😀"));
    assert!(!is_valid_ref("😀😀😀😀"));
}

#[test]
fn unknown_network_uses_explicit_default_only_with_valid_ref() {
    let n = networks(vec![]);
    let sel = select(&n, &route("ZZ:nowhere", Some("12"), None), true).unwrap();
    assert_eq!(sel.rule_key, "default");
    assert!(sel.fallback);
    assert_eq!(sel.text.as_deref(), Some("12"));
    assert_eq!(
        select(&n, &route("ZZ:nowhere", None, None), true),
        Err(SelectError::NoShield(
            NoShieldReason::UnknownNetworkWithoutRef {
                network: "ZZ:nowhere".into()
            }
        ))
    );
    assert_eq!(
        select(&n, &route("ZZ:nowhere", Some("12"), None), false),
        Err(SelectError::Unknown("ZZ:nowhere".into()))
    );
}

#[test]
fn ref_resolution_prefers_hardcoded_then_name_map() {
    let mut by_name = IndexMap::new();
    by_name.insert("Audubon Parkway".to_owned(), "AU".to_owned());
    let def = ShieldDef {
        refs_by_name: Some(by_name),
        ..ShieldDef::default()
    };
    assert_eq!(
        ref_for_def(&route("X", Some("9"), Some("Audubon Parkway")), &def),
        "AU"
    );
    assert_eq!(
        ref_for_def(&route("X", Some("9"), Some("Other")), &def),
        "9"
    );
    let hard = ShieldDef {
        ref_: Some("BBX".into()),
        ..def
    };
    assert_eq!(
        ref_for_def(&route("X", Some("9"), Some("Audubon Parkway")), &hard),
        "BBX"
    );
}

#[test]
fn overrides_apply_by_ref_then_by_name() {
    let mut by_ref = IndexMap::new();
    by_ref.insert(
        "QEW".to_owned(),
        ShieldDef {
            text_color: Some("blue".into()),
            ..ShieldDef::default()
        },
    );
    let mut by_name = IndexMap::new();
    by_name.insert(
        "Bridge".to_owned(),
        ShieldDef {
            text_color: Some("red".into()),
            sprite_blank: blank("b"),
            ..ShieldDef::default()
        },
    );
    let def = ShieldDef {
        sprite_blank: blank("a"),
        text_color: Some("black".into()),
        override_by_ref: Some(by_ref),
        override_by_name: Some(by_name),
        ..ShieldDef::default()
    };
    let n = networks(vec![("CA:ON:primary", def)]);
    let sel = select(&n, &route("CA:ON:primary", Some("QEW"), None), true).unwrap();
    assert_eq!(sel.def.text_color.as_deref(), Some("blue"));
    assert_eq!(sel.overrides, vec![AppliedOverride::ByRef("QEW".into())]);
    let sel = select(
        &n,
        &route("CA:ON:primary", Some("QEW"), Some("Bridge")),
        true,
    )
    .unwrap();
    assert_eq!(sel.def.text_color.as_deref(), Some("red"));
    assert_eq!(sel.def.sprite_blank, blank("b"));
    assert_eq!(sel.overrides.len(), 2);
}

#[test]
fn noref_replaces_definition_and_implies_notext() {
    let def = ShieldDef {
        sprite_blank: blank("pa_2"),
        noref: Some(Box::new(ShieldDef {
            sprite_blank: blank("pa_noref"),
            ..ShieldDef::default()
        })),
        ..ShieldDef::default()
    };
    let n = networks(vec![("US:PA:Turnpike", def)]);
    let sel = select(&n, &route("US:PA:Turnpike", None, None), true).unwrap();
    assert_eq!(sel.def.sprite_blank, blank("pa_noref"));
    assert_eq!(sel.text, None);
    assert_eq!(sel.overrides, vec![AppliedOverride::NoRef]);
    let sel = select(&n, &route("US:PA:Turnpike", Some("76"), None), true).unwrap();
    assert_eq!(sel.text.as_deref(), Some("76"));
}

#[test]
fn missing_or_long_refs_suppress_text_shields() {
    let n = networks(vec![(
        "US:I",
        ShieldDef {
            sprite_blank: blank("i_2"),
            ..ShieldDef::default()
        },
    )]);
    for r in [None, Some(""), Some("12345678")] {
        let out = select(&n, &route("US:I", r, None), true);
        assert!(
            matches!(
                out,
                Err(SelectError::NoShield(NoShieldReason::InvalidRef { .. }))
            ),
            "{r:?} → {out:?}"
        );
    }
}

#[test]
fn notext_shields_draw_without_ref_only_with_a_blank() {
    let with_blank = ShieldDef {
        notext: Some(true),
        sprite_blank: blank("x"),
        ..ShieldDef::default()
    };
    let without = ShieldDef {
        notext: Some(true),
        ..ShieldDef::default()
    };
    let n = networks(vec![("A", with_blank), ("B", without)]);
    assert!(
        select(&n, &route("A", None, None), true)
            .unwrap()
            .text
            .is_none()
    );
    assert!(matches!(
        select(&n, &route("B", None, None), true),
        Err(SelectError::NoShield(_))
    ));
}

#[test]
fn roman_numerals_follow_upstream() {
    let def = ShieldDef {
        numbering_system: Some("roman".into()),
        sprite_blank: blank("x"),
        ..ShieldDef::default()
    };
    let n = networks(vec![("IN:NE", def)]);
    assert_eq!(
        select(&n, &route("IN:NE", Some("5"), None), true)
            .unwrap()
            .text
            .as_deref(),
        Some("V")
    );
    for (input, out) in [
        ("1994", "MCMXCIV"),
        ("4A", "IVA"),
        ("49", "XLIX"),
        ("007", "VII07"),
        ("X", "X"),
        ("-3", "-3"),
        ("0", ""),
    ] {
        assert_eq!(romanize(input), out, "{input}");
    }
}

#[test]
fn blank_choice_by_weighted_width() {
    let b = SpriteBlank::Many(vec!["i_2".into(), "i_3".into()]);
    for (r, want) in [
        ("5", "i_2"),
        ("95", "i_2"),
        ("287", "i_3"),
        ("4567", "i_3"),
        ("111", "i_2"),
        ("", "i_2"),
    ] {
        assert_eq!(choose_blank(&b, r), Some(want), "{r}");
    }
    let one = SpriteBlank::One("solo".into());
    assert_eq!(choose_blank(&one, "12345"), Some("solo"));
    assert_eq!(choose_blank(&SpriteBlank::Many(Vec::new()), "1"), None);
}
