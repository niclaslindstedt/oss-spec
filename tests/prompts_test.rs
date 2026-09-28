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
    // fix-conformance ships multiple versioned templates; the loader must
    // pick the highest. Every version since 1.1.0 contains the "Quality
    // findings" block, and 1.2.0 adds §20.5 guidance.
    let p = prompts::load(
        "fix-conformance",
        minijinja::context! {
            spec => "SPEC",
            spec_version => "2.1.0",
            violations => "(test)",
        },
    )
    .unwrap();
    assert!(
        p.system.contains("Quality findings"),
        "highest-version picker should retain the Quality findings block"
    );
    assert!(
        p.system.contains("§20.5"),
        "highest-version picker should have selected the 1.2.0 template with §20.5 guidance"
    );
    assert!(
        p.system.contains("§24") && p.system.contains("do not invent it"),
        "the 1.4.0 template must carry the §24 guidance, including the no-fabrication rule"
    );
    assert!(
        p.system.contains("`summary` line"),
        "the 1.5.0 template must carry the §24.2 optional-field guidance"
    );
    assert!(
        p.system.contains("oss-spec:unlisted-website:") && p.system.contains("§11.3.12"),
        "the 1.6.0 template must carry the §11.3.12 unlisted-website guidance"
    );
    assert!(
        p.system.contains("oss-spec:no-llm-prompts:") && p.system.contains("§13.5.1"),
        "the 1.7.0 template must carry the §13.5.1 prompt-free guidance"
    );
}

#[test]
fn validate_sh_agent_knows_prompt_free_projects() {
    let p = prompts::load(
        "validate-sh-agent",
        minijinja::context! { spec_ref => "OSS_SPEC.md" },
    )
    .unwrap();
    assert!(
        p.user.contains("oss-spec:no-llm-prompts:") && p.user.contains("§13.5.1"),
        "the 1.6.0 checklist must tell the agent to verify a prompt-free claim"
    );
}

#[test]
fn verify_conformance_knows_unlisted_websites() {
    let p = prompts::load(
        "verify-conformance",
        minijinja::context! { spec => "SPEC", spec_version => "2.13.0", violations => "(test)", file_contents => "(test)" },
    )
    .unwrap();
    assert!(
        p.system.contains("oss-spec:unlisted-website:") && p.system.contains("noindex"),
        "the 1.2.0 template must exempt unlisted websites from the §11.3 review"
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
