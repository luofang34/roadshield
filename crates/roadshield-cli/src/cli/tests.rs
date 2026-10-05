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

#[test]
fn context_flags_are_defined_for_render_and_batch() {
    for sub in ["render", "batch"] {
        let mut args = vec!["roadshield", sub, "--text-halo-join", "bevel"];
        if sub == "render" {
            args.extend(["--network", "BAB", "--ref", "A 115"]);
        } else {
            args.extend(["--input", "in.jsonl", "--out-dir", "out"]);
        }
        let m = command().try_get_matches_from(args).unwrap();
        let (_, sub_m) = m.subcommand().unwrap();
        assert_eq!(
            sub_m
                .get_one::<String>("text-halo-join")
                .map(String::as_str),
            Some("bevel")
        );
        crate::commands::context_for_tests(sub_m).unwrap();
    }
}
