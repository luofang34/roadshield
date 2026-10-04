use super::*;

fn spec(json: &str) -> ShieldSpec {
    serde_json::from_str(json).unwrap()
}

const OPTIONS: &str = r#""options": {"bannerTextColor": "black", "bannerTextHaloColor": "white",
    "bannerHeight": 9, "bannerPadding": 1, "shieldFont": "sans-serif", "shieldSize": 20}"#;

#[test]
fn unknown_definition_fields_are_rejected() {
    let json = format!(
        r#"{{"networks": {{"X": {{"textColor": "black", "glowColor": "red"}}}}, {OPTIONS}}}"#
    );
    let err = serde_json::from_str::<ShieldSpec>(&json).unwrap_err();
    assert!(err.to_string().contains("glowColor"), "{err}");
}

#[test]
fn unknown_shape_params_and_options_are_rejected() {
    let json = format!(
        r#"{{"networks": {{"X": {{"shapeBlank": {{"drawFunc": "pill", "params": {{"wobble": 1}}}}}}}}, {OPTIONS}}}"#
    );
    assert!(serde_json::from_str::<ShieldSpec>(&json).is_err());
    let json = r#"{"networks": {}, "options": {"bannerTextColor": "a", "bannerTextHaloColor": "b",
        "bannerHeight": 9, "bannerPadding": 1, "shieldFont": "x", "shieldSize": 20, "extra": 1}}"#;
    assert!(serde_json::from_str::<ShieldSpec>(json).is_err());
}

#[test]
fn sprite_blank_accepts_string_or_list() {
    let s = spec(&format!(
        r#"{{"networks": {{"A": {{"spriteBlank": "one"}}, "B": {{"spriteBlank": ["b_2", "b_3"]}}}}, {OPTIONS}}}"#
    ));
    assert_eq!(
        s.networks["A"].as_ref().unwrap().sprite_blank,
        Some(SpriteBlank::One("one".into()))
    );
    assert_eq!(
        s.networks["B"]
            .as_ref()
            .unwrap()
            .sprite_blank
            .as_ref()
            .unwrap()
            .ids(),
        vec!["b_2", "b_3"]
    );
}

#[test]
fn overlay_matches_object_spread() {
    let base = ShieldDef {
        text_color: Some("black".into()),
        notext: Some(true),
        padding: Some(Padding {
            left: 1.0,
            ..Padding::default()
        }),
        ..ShieldDef::default()
    };
    let over = ShieldDef {
        text_color: Some("white".into()),
        sprite_blank: Some(SpriteBlank::One("x".into())),
        ..ShieldDef::default()
    };
    let merged = base.overlay(&over);
    assert_eq!(merged.text_color.as_deref(), Some("white"));
    assert_eq!(merged.notext, Some(true));
    assert_eq!(merged.padding.map(|p| p.left), Some(1.0));
    assert!(merged.sprite_blank.is_some());
}

#[test]
fn banner_maps_expand_into_networks() {
    let mut s = spec(&format!(
        r#"{{"networks": {{"US:US": {{"textColor": "black", "bannerMap": {{"US:US:Truck": ["TRK"], "US:US:Truck:Bypass": ["TRK", "BYP"]}}}}}}, {OPTIONS}}}"#
    ));
    s.expand_banner_maps();
    let truck = s.networks["US:US:Truck:Bypass"].as_ref().unwrap();
    assert_eq!(
        truck.banners.as_deref(),
        Some(&["TRK".to_owned(), "BYP".to_owned()][..])
    );
    assert_eq!(truck.text_color.as_deref(), Some("black"));
    assert_eq!(s.networks.len(), 3);
}

#[test]
fn banner_map_expansion_uses_a_snapshot_and_keeps_positions() {
    // B's own map still expands although A's map overwrote B first.
    let mut s = spec(&format!(
        r#"{{"networks": {{
            "A": {{"textColor": "red", "bannerMap": {{"B": ["ONE"]}}}},
            "B": {{"textColor": "blue", "bannerMap": {{"C": ["TWO"]}}}}
        }}, {OPTIONS}}}"#
    ));
    s.expand_banner_maps();
    let keys: Vec<&str> = s.networks.keys().map(String::as_str).collect();
    assert_eq!(keys, ["A", "B", "C"]);
    assert_eq!(
        s.networks["B"].as_ref().unwrap().text_color.as_deref(),
        Some("red")
    );
    assert_eq!(
        s.networks["C"].as_ref().unwrap().text_color.as_deref(),
        Some("blue")
    );
}
