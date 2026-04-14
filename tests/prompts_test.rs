use oss_spec::prompts;

#[test]
fn loads_interpret_prompt() {
    let p = prompts::load(
        "interpret-prompt",
        minijinja::context! { prompt => "make a python cli" },
    )
    .unwrap();
    assert!(p.system.contains("oss-spec"));
    assert!(p.user.contains("make a python cli"));
}

#[test]
fn picks_highest_version() {
    // 1_0 must exist for every shipped prompt; if we ever add 1_1 it
    // should win automatically. This test just guards the parser.
    assert_eq!(prompts::parse_version("1_0"), Some((1, 0)));
    assert_eq!(prompts::parse_version("2_13"), Some((2, 13)));
    assert_eq!(prompts::parse_version("README"), None);
}
