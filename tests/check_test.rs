use oss_spec::bootstrap::{symlink_dir, symlink_file};
use oss_spec::check::{
    self, check_toolchain_versions, extract_front_matter, has_yaml_key, is_kebab_case, version_ge,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn version_ge_pads_shorter_segments() {
    assert_eq!(version_ge("1.88", "1.88.0"), Some(true));
    assert_eq!(version_ge("1.88.0", "1.88"), Some(true));
    assert_eq!(version_ge("1.89", "1.88.0"), Some(true));
    assert_eq!(version_ge("1.87.9", "1.88.0"), Some(false));
    assert_eq!(version_ge("24", "24"), Some(true));
    assert_eq!(version_ge("23", "24"), Some(false));
}

#[test]
fn rust_pinned_exact_minimum_is_ok() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.88.0\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn rust_pinned_above_minimum_is_ok() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.89.0\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn rust_stable_is_floating() {
    let yml = "      - uses: dtolnay/rust-toolchain@stable\n";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].spec_section, "§10.3");
    assert!(v[0].message.contains("Rust"));
    assert!(v[0].message.contains("floating specifier 'stable'"));
}

#[test]
fn rust_below_minimum_is_violation() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.75.0\n";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("1.75.0"));
    assert!(v[0].message.contains("minimum is 1.88.0"));
}

#[test]
fn node_lts_star_is_floating() {
    let yml = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"lts/*\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Node"));
    assert!(v[0].message.contains("floating specifier 'lts/*'"));
}

#[test]
fn node_24_is_ok() {
    let yml = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"24\"
";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn node_22_is_below_minimum() {
    let yml = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"22\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Node"));
    assert!(v[0].message.contains("pinned to 22"));
    assert!(v[0].message.contains("minimum is 24"));
}

#[test]
fn python_312_is_ok() {
    let yml = "\
      - uses: actions/setup-python@v5
        with:
          python-version: \"3.12\"
";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn python_311_is_below_minimum() {
    let yml = "\
      - uses: actions/setup-python@v5
        with:
          python-version: \"3.11\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Python"));
}

#[test]
fn go_stable_is_floating() {
    let yml = "\
      - uses: actions/setup-go@v5
        with:
          go-version: \"stable\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Go"));
    assert!(v[0].message.contains("floating specifier 'stable'"));
}

#[test]
fn go_122_is_ok() {
    let yml = "\
      - uses: actions/setup-go@v5
        with:
          go-version: \"1.22\"
";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn missing_toolchain_block_is_not_a_violation() {
    let yml =
        "name: CI\njobs:\n  noop:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo hi\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn rust_short_version_is_accepted() {
    // `@1.88` (no patch) should be treated as `1.88.0` and accepted.
    let yml = "      - uses: dtolnay/rust-toolchain@1.88\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

// --- §20 test organization checks ---

/// Helper: create a minimal repo skeleton that passes all non-§20 checks,
/// so we can isolate §20 violations.
/// Cross-platform symlink removal. On Unix, any symlink is removable with
/// `remove_file`. On Windows, *directory* symlinks must be removed with
/// `remove_dir` — `remove_file` fails with `ERROR_ACCESS_DENIED`. Try both.
fn remove_symlink(p: &Path) {
    if fs::remove_file(p).is_err() {
        fs::remove_dir(p).unwrap();
    }
}

fn scaffold_minimal_repo(root: &std::path::Path) {
    // Required files
    for f in [
        "LICENSE",
        "README.md",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        "SECURITY.md",
        "AGENTS.md",
        "CHANGELOG.md",
        ".gitignore",
        ".editorconfig",
        "Makefile",
    ] {
        fs::write(root.join(f), "").unwrap();
    }
    // Symlinks
    for link in ["CLAUDE.md", ".cursorrules", ".windsurfrules", "GEMINI.md"] {
        symlink_file(Path::new("AGENTS.md"), &root.join(link)).unwrap();
    }
    // Directories
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::create_dir_all(root.join(".github/ISSUE_TEMPLATE")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("prompts")).unwrap();
    fs::create_dir_all(root.join("scripts")).unwrap();
    symlink_file(
        Path::new("../AGENTS.md"),
        &root.join(".github/copilot-instructions.md"),
    )
    .unwrap();
    // Required workflows
    for w in ["ci.yml", "version-bump.yml", "release.yml", "pages.yml"] {
        fs::write(root.join(".github/workflows").join(w), "").unwrap();
    }
    // Required templates
    fs::write(root.join(".github/PULL_REQUEST_TEMPLATE.md"), "").unwrap();
    fs::write(root.join(".github/ISSUE_TEMPLATE/bug_report.md"), "").unwrap();
    fs::write(root.join(".github/ISSUE_TEMPLATE/feature_request.md"), "").unwrap();
    fs::write(root.join(".github/ISSUE_TEMPLATE/config.yml"), "").unwrap();
    fs::write(root.join(".github/dependabot.yml"), "").unwrap();
    // §21 agent skills: README is always present, so update-readme is
    // mandatory. docs/ was created above, so update-docs is mandatory too.
    // `maintenance` is always required (§21.6).
    for skill in ["update-readme", "update-docs", "maintenance"] {
        let dir = root.join(".agent/skills").join(skill);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {skill}\ndescription: \"test skill\"\n---\n\n\
                 # {skill}\n\n## Tracking mechanism\n\n## Discovery process\n"
            ),
        )
        .unwrap();
        fs::write(dir.join(".last-updated"), "").unwrap();
    }
    // §21.2: `.claude/skills` -> `../.agent/skills`
    fs::create_dir_all(root.join(".claude")).unwrap();
    symlink_dir(Path::new("../.agent/skills"), &root.join(".claude/skills")).unwrap();
}

#[test]
fn inline_cfg_test_in_src_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
         #[cfg(test)]\nmod tests {\n    use super::*;\n\n    \
         #[test]\n    fn it_works() { assert_eq!(add(1, 2), 3); }\n}\n",
    )
    .unwrap();

    let report = check::run(root).unwrap();
    let v20: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20")
        .collect();
    assert_eq!(v20.len(), 1);
    assert!(v20[0].message.contains("src/lib.rs"));
    assert!(v20[0].message.contains("inline test block"));
}

#[test]
fn no_inline_tests_means_no_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();

    let report = check::run(root).unwrap();
    let v20: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20")
        .collect();
    assert!(v20.is_empty());
}

#[test]
fn cfg_test_importing_separate_file_is_allowed() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    fs::create_dir_all(root.join("src")).unwrap();
    // `#[cfg(test)] mod check_test;` imports a separate file — not inline.
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
         #[cfg(test)]\nmod check_test;\n",
    )
    .unwrap();

    let report = check::run(root).unwrap();
    let v20: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20")
        .collect();
    assert!(
        v20.is_empty(),
        "cfg(test) importing a file should not be flagged: {v20:?}"
    );
}

#[test]
fn cfg_test_with_use_statement_is_allowed() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
         #[cfg(test)]\nuse some_test_crate::helpers;\n",
    )
    .unwrap();

    let report = check::run(root).unwrap();
    let v20: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20")
        .collect();
    assert!(
        v20.is_empty(),
        "cfg(test) gating a use statement should not be flagged: {v20:?}"
    );
}

#[test]
fn test_file_with_bad_name_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(root.join("tests/my_checks.rs"), "#[test] fn t() {}\n").unwrap();

    let report = check::run(root).unwrap();
    let v202: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20.2")
        .collect();
    assert_eq!(v202.len(), 1);
    assert!(v202[0].message.contains("my_checks"));
}

#[test]
fn test_file_with_valid_names_pass() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    fs::create_dir_all(root.join("tests")).unwrap();
    for name in [
        "check_test.rs",
        "check_tests.rs",
        "CheckTest.rs",
        "CheckTests.rs",
    ] {
        fs::write(root.join("tests").join(name), "#[test] fn t() {}\n").unwrap();
    }

    let report = check::run(root).unwrap();
    let v202: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20.2")
        .collect();
    assert!(v202.is_empty());
}

// --- §21 agent skills checks ---

/// Collect all §21* violations from a report for easy assertions.
fn v21(report: &check::Report) -> Vec<&check::Violation> {
    report
        .violations
        .iter()
        .filter(|v| v.spec_section.starts_with("§21"))
        .collect()
}

#[test]
fn minimal_repo_passes_section_21() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    let report = check::run(root).unwrap();
    let v = v21(&report);
    assert!(v.is_empty(), "expected no §21 violations, got {v:?}");
}

#[test]
fn missing_agent_skills_dir_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    // Blow away the skills tree entirely.
    fs::remove_dir_all(root.join(".agent")).unwrap();
    remove_symlink(&root.join(".claude/skills"));

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.2")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains(".agent/skills")),
        "expected a violation about missing .agent/skills, got {v:?}"
    );
}

#[test]
fn claude_skills_must_be_symlink() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    // Replace the symlink with a real directory.
    remove_symlink(&root.join(".claude/skills"));
    fs::create_dir_all(root.join(".claude/skills")).unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.2")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains(".claude/skills")),
        "expected a violation about .claude/skills, got {v:?}"
    );
}

#[test]
fn missing_update_readme_skill_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::remove_dir_all(root.join(".agent/skills/update-readme")).unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.5")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("update-readme")),
        "expected a violation about missing update-readme, got {v:?}"
    );
}

#[test]
fn missing_maintenance_umbrella_skill_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::remove_dir_all(root.join(".agent/skills/maintenance")).unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.6")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("maintenance")),
        "expected a §21.6 violation about missing maintenance skill, got {v:?}"
    );
}

#[test]
fn missing_update_docs_required_because_docs_exists() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::remove_dir_all(root.join(".agent/skills/update-docs")).unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.5")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("update-docs")),
        "expected a violation about missing update-docs, got {v:?}"
    );
}

#[test]
fn update_manpages_required_when_man_dir_exists() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("man")).unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.5")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("update-manpages")),
        "expected update-manpages requirement once man/ exists, got {v:?}"
    );
}

#[test]
fn update_website_required_when_website_dir_exists() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("website")).unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.5")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("update-website")),
        "expected update-website requirement once website/ exists, got {v:?}"
    );
}

#[test]
fn skill_missing_skill_md_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    // Add an empty skill directory (no SKILL.md).
    fs::create_dir_all(root.join(".agent/skills/broken-skill")).unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.3")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("broken-skill")),
        "expected a §21.3 violation about missing SKILL.md, got {v:?}"
    );
}

#[test]
fn skill_missing_front_matter_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    let dir = root.join(".agent/skills/plain-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), "# plain-skill\n\nno front matter\n").unwrap();
    fs::write(dir.join(".last-updated"), "").unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.3")
        .collect();
    assert!(
        v.iter()
            .any(|v| v.message.contains("plain-skill") && v.message.contains("front matter")),
        "expected a §21.3 violation about missing front matter, got {v:?}"
    );
}

#[test]
fn skill_front_matter_missing_description_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    let dir = root.join(".agent/skills/nameless-skill");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: nameless-skill\n---\n\n# nameless-skill\n",
    )
    .unwrap();
    fs::write(dir.join(".last-updated"), "").unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.3")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("description")),
        "expected a §21.3 violation about missing description, got {v:?}"
    );
}

#[test]
fn skill_missing_last_updated_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    let dir = root.join(".agent/skills/no-tracking");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: no-tracking\ndescription: x\n---\n\n# no-tracking\n",
    )
    .unwrap();
    // No .last-updated file.

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.4")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("no-tracking")),
        "expected a §21.4 violation about missing .last-updated, got {v:?}"
    );
}

#[test]
fn skill_name_must_be_kebab_case() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    let dir = root.join(".agent/skills/BadName");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: BadName\ndescription: x\n---\n\n# BadName\n",
    )
    .unwrap();
    fs::write(dir.join(".last-updated"), "").unwrap();

    let report = check::run(root).unwrap();
    let v: Vec<_> = v21(&report)
        .into_iter()
        .filter(|v| v.spec_section == "§21.5")
        .collect();
    assert!(
        v.iter().any(|v| v.message.contains("kebab-case")),
        "expected a §21.5 kebab-case violation, got {v:?}"
    );
}

#[test]
fn is_kebab_case_examples() {
    assert!(is_kebab_case("update-readme"));
    assert!(is_kebab_case("a"));
    assert!(is_kebab_case("x1-y2-z3"));
    assert!(!is_kebab_case(""));
    assert!(!is_kebab_case("-leading"));
    assert!(!is_kebab_case("trailing-"));
    assert!(!is_kebab_case("double--hyphen"));
    assert!(!is_kebab_case("Upper"));
    assert!(!is_kebab_case("snake_case"));
}

#[test]
fn extract_front_matter_basic() {
    let src = "---\nname: x\ndescription: y\n---\n\n# body\n";
    let fm = extract_front_matter(src).unwrap();
    assert!(fm.contains("name: x"));
    assert!(fm.contains("description: y"));
}

#[test]
fn extract_front_matter_none_when_missing() {
    assert!(extract_front_matter("# body without front matter\n").is_none());
}

#[test]
fn extract_front_matter_handles_crlf() {
    // Git on Windows may normalise LF → CRLF via `core.autocrlf`; the
    // parser must still find the front matter. Covers the regression
    // that broke §21 checks on windows-latest CI.
    let src = "---\r\nname: x\r\ndescription: y\r\n---\r\n\r\n# body\r\n";
    let fm = extract_front_matter(src).expect("front matter should parse with CRLF");
    assert!(has_yaml_key(fm, "name"));
    assert!(has_yaml_key(fm, "description"));
}

#[test]
fn has_yaml_key_matches_top_level() {
    let fm = "name: skill-name\ndescription: \"use when...\"\nother: z\n";
    assert!(has_yaml_key(fm, "name"));
    assert!(has_yaml_key(fm, "description"));
    assert!(has_yaml_key(fm, "other"));
    assert!(!has_yaml_key(fm, "missing"));
}

#[test]
fn has_yaml_key_ignores_indented_lines() {
    // Key nested under another key must not count as top-level.
    let fm = "name: x\nparent:\n  nested: y\n";
    assert!(has_yaml_key(fm, "name"));
    assert!(has_yaml_key(fm, "parent"));
    assert!(!has_yaml_key(fm, "nested"));
}

// ---------------------------------------------------------------------------
// AiFinding + gather_file_contents tests
// ---------------------------------------------------------------------------

use oss_spec::check::{AiFinding, gather_file_contents};

#[test]
fn ai_finding_deserializes_from_json() {
    let json = r#"{
        "file": "README.md",
        "spec_section": "§3",
        "severity": "warning",
        "message": "Missing Why section",
        "suggestion": "Add a ## Why section with 3-5 bullet points"
    }"#;
    let f: AiFinding = serde_json::from_str(json).unwrap();
    assert_eq!(f.file, "README.md");
    assert_eq!(f.spec_section, "§3");
    assert_eq!(f.severity, "warning");
    assert!(!f.message.is_empty());
    assert!(!f.suggestion.is_empty());
}

#[test]
fn ai_finding_array_deserializes() {
    let json = r#"{"findings": [
        {"file": "A", "spec_section": "§1", "severity": "error", "message": "bad", "suggestion": "fix"},
        {"file": "B", "spec_section": "§2", "severity": "warning", "message": "meh", "suggestion": "improve"}
    ]}"#;
    #[derive(serde::Deserialize)]
    struct Wire {
        findings: Vec<AiFinding>,
    }
    let w: Wire = serde_json::from_str(json).unwrap();
    assert_eq!(w.findings.len(), 2);
}

#[test]
fn is_clean_ignores_ai_findings() {
    let mut report = oss_spec::check::Report::default();
    report.ai_findings.push(AiFinding {
        file: "README.md".into(),
        spec_section: "§3".into(),
        severity: "warning".into(),
        message: "test".into(),
        suggestion: "test".into(),
    });
    // AI findings alone do not make the report "dirty".
    assert!(report.is_clean());
}

#[test]
fn gather_file_contents_includes_existing_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Create a subset of verifiable files.
    std::fs::write(root.join("README.md"), "# Hello").unwrap();
    std::fs::write(root.join("LICENSE"), "MIT").unwrap();
    std::fs::write(root.join("Makefile"), "build:\n\tcargo build").unwrap();

    let contents = gather_file_contents(root);
    let names: Vec<&str> = contents.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"README.md"));
    assert!(names.contains(&"LICENSE"));
    assert!(names.contains(&"Makefile"));
    // Non-existent files should not appear.
    assert!(!names.contains(&"CONTRIBUTING.md"));
}

#[test]
fn gather_file_contents_truncates_long_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let long_content: String = (0..500).map(|i| format!("line {i}\n")).collect();
    std::fs::write(root.join("README.md"), &long_content).unwrap();

    let contents = gather_file_contents(root);
    let (_, content) = contents.iter().find(|(n, _)| n == "README.md").unwrap();
    let line_count = content.lines().count();
    assert_eq!(line_count, 200);
}
