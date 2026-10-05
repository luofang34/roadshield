use super::*;
use crate::model::ShieldDef;

fn def(color: &str) -> Option<ShieldDef> {
    Some(ShieldDef {
        text_color: Some(color.into()),
        ..ShieldDef::default()
    })
}

fn map(entries: &[(&str, &str)]) -> IndexMap<String, Option<ShieldDef>> {
    entries
        .iter()
        .map(|(k, c)| ((*k).to_owned(), def(c)))
        .collect()
}

#[test]
fn merge_appends_new_networks() {
    let mut upstream = map(&[("e-road", "white")]);
    let ext = ExtensionSpec {
        networks: map(&[("AH", "white"), ("BAB", "white")]),
    };
    let added = merge(&mut upstream, &ext).unwrap();
    assert_eq!(added.into_iter().collect::<Vec<_>>(), ["AH", "BAB"]);
    assert_eq!(upstream.keys().collect::<Vec<_>>(), ["e-road", "AH", "BAB"]);
}

#[test]
fn redefining_an_upstream_network_is_a_conflict() {
    let mut upstream = map(&[("BAB", "black"), ("e-road", "white")]);
    let ext = ExtensionSpec {
        networks: map(&[("AH", "white"), ("BAB", "white")]),
    };
    assert_eq!(merge(&mut upstream, &ext), Err(vec!["BAB".to_owned()]));
    assert_eq!(upstream.len(), 2, "nothing merged on conflict");
}

#[test]
fn banner_map_variants_conflict_too() {
    let mut upstream = map(&[("BAB:Bypass", "black")]);
    let mut bab = def("white").unwrap();
    bab.banner_map = Some(
        [("BAB:Bypass".to_owned(), vec!["BYP".to_owned()])]
            .into_iter()
            .collect(),
    );
    let ext = ExtensionSpec {
        networks: [("BAB".to_owned(), Some(bab))].into_iter().collect(),
    };
    assert_eq!(
        merge(&mut upstream, &ext),
        Err(vec!["BAB:Bypass".to_owned()])
    );
}
