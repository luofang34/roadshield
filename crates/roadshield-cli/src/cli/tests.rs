use super::command;

#[test]
fn command_tree_is_consistent() {
    command().debug_assert();
}

#[test]
fn render_requires_network() {
    let err = command().try_get_matches_from(["roadshield", "render"]);
    assert!(err.is_err());
    let ok = command().try_get_matches_from([
        "roadshield",
        "render",
        "--network",
        "US:I",
        "--ref",
        "287",
    ]);
    assert!(ok.is_ok());
}
