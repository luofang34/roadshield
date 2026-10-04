use super::*;

#[test]
fn value_diff_reports_paths() {
    let old = serde_json::json!({"a": 1, "b": {"c": "x", "d": true}});
    let new = serde_json::json!({"a": 1, "b": {"c": "y"}, "e": [1]});
    let mut out = Vec::new();
    value_diff("", &old, &new, &mut out);
    let paths: Vec<&str> = out.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, ["b.c", "b.d", "e"]);
}

#[test]
fn base64_matches_reference() {
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"<svg/>"), "PHN2Zy8+");
}

#[test]
fn id_changes_classify() {
    let m = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect()
    };
    let c = IdChanges::from_maps(&m(&[("a", "1"), ("b", "2")]), &m(&[("b", "3"), ("c", "4")]));
    assert_eq!(
        (c.added, c.removed, c.changed),
        (vec!["c".into()], vec!["a".into()], vec!["b".into()])
    );
}
