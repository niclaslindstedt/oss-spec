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
    // Front matter must be stripped — it is metadata, not instruction.
    assert!(!p.system.contains("---"));
    assert!(!p.user.contains("---"));
}

#[test]
fn parses_semver_stems() {
    assert_eq!(prompts::parse_version("1_0_0"), Some((1, 0, 0)));
    assert_eq!(prompts::parse_version("2_13_7"), Some((2, 13, 7)));
    assert_eq!(prompts::parse_version("10_0_0"), Some((10, 0, 0)));
}

#[test]
fn rejects_non_semver_stems() {
    // Old two-segment format is no longer valid.
    assert_eq!(prompts::parse_version("1_0"), None);
    assert_eq!(prompts::parse_version("2_13"), None);
    // Other nonsense.
    assert_eq!(prompts::parse_version("README"), None);
    assert_eq!(prompts::parse_version("1_0_0_0"), None);
    assert_eq!(prompts::parse_version(""), None);
    assert_eq!(prompts::parse_version("a_b_c"), None);
}

#[test]
fn picks_highest_version() {
    // fix-conformance ships `1_0_0.md` and `1_1_0.md`; the loader must
    // pick the newer one.
    let p = prompts::load(
        "fix-conformance",
        minijinja::context! {
            spec => "SPEC",
            spec_version => "2.1.0",
            violations => "(test)",
        },
    )
    .unwrap();
    // v1.1.0 introduced quality-findings handling.
    assert!(
        p.system.contains("Quality findings"),
        "highest-version picker should have selected 1_1_0 with quality findings block"
    );
}

#[test]
fn strip_front_matter_removes_metadata() {
    let raw = "---\nname: x\nversion: 1.0.0\n---\n\n# x\n\n## System\nhi\n\n## User\nho\n";
    let stripped = prompts::strip_front_matter(raw);
    assert!(!stripped.contains("version: 1.0.0"));
    assert!(stripped.contains("## System"));
    assert!(stripped.contains("## User"));
}

#[test]
fn strip_front_matter_passes_through_when_missing() {
    let raw = "# no front matter\n\n## System\nhi\n\n## User\nho\n";
    let stripped = prompts::strip_front_matter(raw);
    assert_eq!(stripped, raw);
}

#[test]
fn strip_front_matter_handles_crlf() {
    let raw =
        "---\r\nname: x\r\nversion: 1.0.0\r\n---\r\n\r\n## System\r\nhi\r\n\r\n## User\r\nho\r\n";
    let stripped = prompts::strip_front_matter(raw);
    assert!(!stripped.contains("version"));
    assert!(stripped.contains("## System"));
}
