//! The README's code block must stay identical to `examples/readme.rs`,
//! which CI compiles.

#[test]
fn readme_example_matches_examples_readme_rs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = std::fs::read_to_string(root.join("../../README.md")).unwrap();
    let example = std::fs::read_to_string(root.join("examples/readme.rs")).unwrap();
    let block = readme
        .split("```rust\n")
        .nth(1)
        .and_then(|b| b.split("```").next())
        .unwrap();
    let body = example.split_once("\n\n").map(|(_, b)| b).unwrap();
    assert_eq!(
        block, body,
        "README code block and examples/readme.rs differ"
    );
}
