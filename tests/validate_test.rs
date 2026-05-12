use oss_spec::bootstrap::{symlink_dir, symlink_file};
use oss_spec::validate::{
    self, check_local_toolchain_pin, check_toolchain_versions, extract_front_matter,
    find_rust_ci_version, find_setup_version, has_yaml_key, is_kebab_case, parse_go_toolchain,
    parse_rust_channel, version_ge, versions_same_major_minor,
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

// --- §10.5 Local/CI environment parity checks ---

fn rust_ci_yml(version: &str) -> String {
    format!("      - uses: dtolnay/rust-toolchain@{version}\n")
}

#[test]
fn parse_rust_channel_reads_toolchain_table() {
    let toml = "[toolchain]\nchannel = \"1.88.0\"\n";
    assert_eq!(parse_rust_channel(toml).as_deref(), Some("1.88.0"));
}

#[test]
fn parse_rust_channel_ignores_other_sections() {
    let toml = "[profile.dev]\nchannel = \"ignored\"\n";
    assert!(parse_rust_channel(toml).is_none());
}

#[test]
fn parse_go_toolchain_reads_directive() {
    let go_mod = "module x\n\ngo 1.22\n\ntoolchain go1.22.6\n";
    assert_eq!(parse_go_toolchain(go_mod).as_deref(), Some("1.22.6"));
}

#[test]
fn parse_go_toolchain_requires_toolchain_directive() {
    let go_mod = "module x\n\ngo 1.22\n";
    assert!(parse_go_toolchain(go_mod).is_none());
}

#[test]
fn versions_same_major_minor_matches() {
    assert!(versions_same_major_minor("3.12.3", "3.12"));
    assert!(versions_same_major_minor("3.12", "3.12.9"));
    assert!(!versions_same_major_minor("3.12", "3.13"));
    assert!(!versions_same_major_minor("24", "22"));
}

#[test]
fn find_rust_ci_version_extracts_spec() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.88.0\n";
    assert_eq!(find_rust_ci_version(yml).as_deref(), Some("1.88.0"));
}

#[test]
fn find_setup_version_extracts_python_version() {
    let yml = "\
      - uses: actions/setup-python@v5
        with:
          python-version: \"3.12\"
";
    assert_eq!(
        find_setup_version(yml, "actions/setup-python", "python-version").as_deref(),
        Some("3.12")
    );
}

#[test]
fn rust_toolchain_file_required_when_cargo_toml_present() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let ci = rust_ci_yml("1.88.0");
    let v = check_local_toolchain_pin(root, Some(&ci));
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].spec_section, "§10.5");
    assert!(v[0].message.contains("rust-toolchain.toml"));
}

#[test]
fn rust_toolchain_floating_channel_flagged() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    fs::write(
        root.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"stable\"\n",
    )
    .unwrap();
    let ci = rust_ci_yml("1.88.0");
    let v = check_local_toolchain_pin(root, Some(&ci));
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].spec_section, "§10.5");
    assert!(v[0].message.contains("floating specifier"));
}

#[test]
fn rust_toolchain_channel_must_match_ci() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    fs::write(
        root.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"1.89.0\"\n",
    )
    .unwrap();
    let ci = rust_ci_yml("1.88.0");
    let v = check_local_toolchain_pin(root, Some(&ci));
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("does not match"));
    assert!(v[0].message.contains("1.89.0"));
    assert!(v[0].message.contains("1.88.0"));
}

#[test]
fn rust_toolchain_matching_channel_is_clean() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    fs::write(
        root.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"1.88.0\"\n",
    )
    .unwrap();
    let ci = rust_ci_yml("1.88.0");
    assert!(check_local_toolchain_pin(root, Some(&ci)).is_empty());
}

#[test]
fn python_version_file_required_when_pyproject_present() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();
    let v = check_local_toolchain_pin(root, None);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].spec_section, "§10.5");
    assert!(v[0].message.contains(".python-version"));
}

#[test]
fn python_version_file_must_match_ci() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();
    fs::write(root.join(".python-version"), "3.13\n").unwrap();
    let ci = "\
      - uses: actions/setup-python@v5
        with:
          python-version: \"3.12\"
";
    let v = check_local_toolchain_pin(root, Some(ci));
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("does not match"));
}

#[test]
fn node_nvmrc_required_when_package_json_present() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("package.json"), "{\"name\":\"x\"}\n").unwrap();
    let v = check_local_toolchain_pin(root, None);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].spec_section, "§10.5");
    assert!(v[0].message.contains(".nvmrc"));
}

#[test]
fn node_nvmrc_must_match_ci_major() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("package.json"), "{\"name\":\"x\"}\n").unwrap();
    fs::write(root.join(".nvmrc"), "22\n").unwrap();
    let ci = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"24\"
";
    let v = check_local_toolchain_pin(root, Some(ci));
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains(".nvmrc '22'"));
}

#[test]
fn node_nvmrc_strips_v_prefix() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("package.json"), "{\"name\":\"x\"}\n").unwrap();
    fs::write(root.join(".nvmrc"), "v24\n").unwrap();
    let ci = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"24\"
";
    assert!(check_local_toolchain_pin(root, Some(ci)).is_empty());
}

#[test]
fn go_mod_must_have_toolchain_directive() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("go.mod"), "module x\n\ngo 1.22\n").unwrap();
    let v = check_local_toolchain_pin(root, None);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].spec_section, "§10.5");
    assert!(v[0].message.contains("toolchain"));
}

#[test]
fn go_mod_with_toolchain_directive_is_clean() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join("go.mod"),
        "module x\n\ngo 1.22\n\ntoolchain go1.22.6\n",
    )
    .unwrap();
    let ci = "\
      - uses: actions/setup-go@v5
        with:
          go-version: \"1.22\"
";
    assert!(check_local_toolchain_pin(root, Some(ci)).is_empty());
}

#[test]
fn generic_project_skips_pin_check() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    // No language manifest files at all.
    assert!(check_local_toolchain_pin(root, None).is_empty());
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
    // Required workflows (§10 + §11.3.10)
    for w in [
        "ci.yml",
        "version-bump.yml",
        "release.yml",
        "pages.yml",
        "seo.yml",
        "lighthouse.yml",
    ] {
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
    let v202: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20.2")
        .collect();
    assert!(v202.is_empty());
}

// --- §21 agent skills checks ---

/// Collect all §21* violations from a report for easy assertions.
fn v21(report: &validate::Report) -> Vec<&validate::Violation> {
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

    let report = validate::run(root).unwrap();
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

use oss_spec::validate::{AiFinding, gather_file_contents};

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
    let mut report = oss_spec::validate::Report::default();
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

// ---------------------------------------------------------------------------
// §19.4 central output module checks
// ---------------------------------------------------------------------------

/// Collect all §19.4 violations from a report.
fn v194(report: &validate::Report) -> Vec<&validate::Violation> {
    report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§19.4")
        .collect()
}

#[test]
fn output_module_rust_is_ok() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/output.rs"), "pub fn info(_: &str) {}\n").unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v194(&report).is_empty(),
        "src/output.rs should satisfy §19.4: {:?}",
        v194(&report)
    );
}

#[test]
fn output_module_node_is_ok() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("lib")).unwrap();
    fs::write(
        root.join("lib/output.ts"),
        "export const info = () => {};\n",
    )
    .unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v194(&report).is_empty(),
        "lib/output.ts should satisfy §19.4: {:?}",
        v194(&report)
    );
}

#[test]
fn output_module_directory_is_ok() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("src/output")).unwrap();
    fs::write(root.join("src/output/mod.rs"), "pub fn info(_: &str) {}\n").unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v194(&report).is_empty(),
        "src/output/ directory should satisfy §19.4: {:?}",
        v194(&report)
    );
}

#[test]
fn missing_output_module_with_src_is_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    // Create a src/ tree without an output module.
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn add() {}\n").unwrap();

    let report = validate::run(root).unwrap();
    let v = v194(&report);
    assert_eq!(v.len(), 1, "expected one §19.4 violation, got {v:?}");
    assert!(v[0].message.contains("output module"));
}

#[test]
fn missing_output_module_without_source_tree_is_skipped() {
    // A docs-only or template-only repo has no source tree, so §19.4
    // does not apply.
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);

    let report = validate::run(root).unwrap();
    assert!(
        v194(&report).is_empty(),
        "§19.4 must not fire without src/ or lib/: {:?}",
        v194(&report)
    );
}

// ---------------------------------------------------------------------------
// embedded::oss_spec_version
// ---------------------------------------------------------------------------

#[test]
fn embedded_oss_spec_version_is_parseable() {
    let v = oss_spec::embedded::oss_spec_version();
    assert_ne!(v, "unknown", "spec version front matter failed to parse");
    // Expect dotted semver — at least one dot.
    assert!(
        v.contains('.'),
        "spec version '{v}' does not look like semver"
    );
    // Each segment must parse as a number.
    for seg in v.split('.') {
        assert!(
            seg.chars().all(|c| c.is_ascii_digit()),
            "non-numeric segment '{seg}' in spec version '{v}'"
        );
    }
}

#[test]
fn embedded_oss_spec_version_matches_front_matter() {
    // Regression: on Windows, Git may check out .md files with CRLF line
    // endings (via core.autocrlf). The parser must accept both.
    let v = oss_spec::embedded::oss_spec_version();
    assert!(
        oss_spec::embedded::OSS_SPEC.contains(&format!("version: {v}")),
        "oss_spec_version() returned '{v}' which does not appear in the \
         embedded spec front matter"
    );
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

// ---------------------------------------------------------------------------
// §20.5 Source file size limit
// ---------------------------------------------------------------------------

fn v205(report: &validate::Report) -> Vec<&validate::Violation> {
    report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§20.5")
        .collect()
}

fn write_source_with_lines(path: &Path, line_count: usize) {
    let body: String = (0..line_count).map(|i| format!("// line {i}\n")).collect();
    fs::write(path, body).unwrap();
}

#[test]
fn source_file_over_1000_lines_flags_violation() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("src")).unwrap();
    write_source_with_lines(&root.join("src/huge.rs"), 1050);

    let report = validate::run(root).unwrap();
    let v = v205(&report);
    assert_eq!(v.len(), 1, "expected one §20.5 violation, got {v:?}");
    assert!(v[0].message.contains("src/huge.rs"));
    assert!(v[0].message.contains("1050"));
    assert!(v[0].message.contains("1000-line limit"));
}

#[test]
fn test_file_over_1000_lines_is_ignored() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    // tests/ directory exists in the scaffolded repo; create a fat test
    // file whose stem matches §20.2. It must not trip §20.5.
    fs::create_dir_all(root.join("tests")).unwrap();
    write_source_with_lines(&root.join("tests/huge_test.rs"), 1050);

    let report = validate::run(root).unwrap();
    assert!(
        v205(&report).is_empty(),
        "§20.5 must not fire on test files: {:?}",
        v205(&report)
    );
}

#[test]
fn allow_large_file_marker_exempts() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("src")).unwrap();
    let mut body = String::from("// oss-spec:allow-large-file: generated bindings\n");
    for i in 1..1050 {
        body.push_str(&format!("// line {i}\n"));
    }
    fs::write(root.join("src/huge.rs"), body).unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v205(&report).is_empty(),
        "marker with reason must exempt: {:?}",
        v205(&report)
    );
}

#[test]
fn allow_large_file_marker_without_reason_does_not_exempt() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("src")).unwrap();
    let mut body = String::from("// oss-spec:allow-large-file:    \n");
    for i in 1..1050 {
        body.push_str(&format!("// line {i}\n"));
    }
    fs::write(root.join("src/huge.rs"), body).unwrap();

    let report = validate::run(root).unwrap();
    assert_eq!(
        v205(&report).len(),
        1,
        "empty-reason marker must not exempt: {:?}",
        v205(&report)
    );
}

#[test]
fn file_exactly_1000_lines_is_clean() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("src")).unwrap();
    write_source_with_lines(&root.join("src/boundary.rs"), 1000);

    let report = validate::run(root).unwrap();
    assert!(
        v205(&report).is_empty(),
        "exactly 1000 lines must be clean: {:?}",
        v205(&report)
    );
}

#[test]
fn allow_large_file_marker_outside_first_20_lines_does_not_exempt() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    scaffold_minimal_repo(root);
    fs::create_dir_all(root.join("src")).unwrap();
    let mut body = String::new();
    for i in 0..25 {
        body.push_str(&format!("// line {i}\n"));
    }
    body.push_str("// oss-spec:allow-large-file: hidden too deep\n");
    for i in 26..1050 {
        body.push_str(&format!("// line {i}\n"));
    }
    fs::write(root.join("src/huge.rs"), body).unwrap();

    let report = validate::run(root).unwrap();
    assert_eq!(
        v205(&report).len(),
        1,
        "marker beyond line 20 must not exempt: {:?}",
        v205(&report)
    );
}

// §11.3 SEO and discoverability ---------------------------------------------

fn v113(report: &validate::Report) -> Vec<&validate::Violation> {
    report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§11.3")
        .collect()
}

#[test]
fn no_website_skips_seo_check() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("README.md"), "# proj\n").unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v113(&report).is_empty(),
        "projects without a website must not be flagged: {:?}",
        v113(&report)
    );
}

#[test]
fn website_with_full_seo_passes() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("pages")).unwrap();
    fs::write(
        root.join("pages/index.html"),
        r#"<!doctype html><html><head>
<meta property="og:image" content="/og.png" />
<meta name="twitter:card" content="summary_large_image" />
<script type="application/ld+json">{"@type":"SoftwareApplication"}</script>
</head><body></body></html>"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("pages/scripts")).unwrap();
    fs::write(
        root.join("pages/scripts/build-seo.mjs"),
        "// emits sitemap.xml, robots.txt, and llms.txt\n",
    )
    .unwrap();
    fs::write(
        root.join("pages/scripts/check-seo.mjs"),
        "// structural SEO check per §11.3.10\n",
    )
    .unwrap();
    fs::write(
        root.join("lighthouserc.json"),
        r#"{"ci":{"assert":{"assertions":{}}}}"#,
    )
    .unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v113(&report).is_empty(),
        "fully-equipped website must not flag §11.3: {:?}",
        v113(&report)
    );
}

#[test]
fn website_missing_all_seo_signals_flags_violation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("site")).unwrap();
    fs::write(
        root.join("site/index.html"),
        "<!doctype html><html><head><title>hi</title></head><body></body></html>",
    )
    .unwrap();

    let report = validate::run(root).unwrap();
    let v = v113(&report);
    assert_eq!(v.len(), 1, "expected one §11.3 violation, got {v:?}");
    let msg = &v[0].message;
    assert!(msg.contains("Open Graph"), "missing OG note: {msg}");
    assert!(msg.contains("Twitter Card"), "missing TC note: {msg}");
    assert!(msg.contains("JSON-LD"), "missing JSON-LD note: {msg}");
    assert!(msg.contains("sitemap.xml"), "missing sitemap note: {msg}");
    assert!(msg.contains("robots.txt"), "missing robots note: {msg}");
    assert!(msg.contains("llms.txt"), "missing llms.txt note: {msg}");
    assert!(msg.contains("check-seo"), "missing check-seo note: {msg}");
    assert!(msg.contains("lighthouse"), "missing lighthouse note: {msg}");
}

#[test]
fn deploy_pages_workflow_alone_triggers_seo_check() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::write(
        root.join(".github/workflows/pages.yml"),
        "jobs:\n  deploy:\n    steps:\n      - uses: actions/deploy-pages@v4\n",
    )
    .unwrap();

    let report = validate::run(root).unwrap();
    assert_eq!(
        v113(&report).len(),
        1,
        "deploy-pages workflow must trigger §11.3 even without index.html"
    );
}

#[test]
fn website_with_partial_seo_lists_only_missing() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("docs-site")).unwrap();
    fs::write(
        root.join("docs-site/index.html"),
        r#"<!doctype html><html><head>
<meta property="og:image" content="/og.png" />
<meta name="twitter:card" content="summary_large_image" />
<script type="application/ld+json">{}</script>
</head></html>"#,
    )
    .unwrap();
    // Bake in the remaining "site-wide discovery" signals so the test
    // isolates the §11.3.10 enforcement bits.
    fs::create_dir_all(root.join("docs-site/scripts")).unwrap();
    fs::write(
        root.join("docs-site/scripts/build.mjs"),
        "// emits sitemap.xml, robots.txt, and llms.txt\n",
    )
    .unwrap();

    let report = validate::run(root).unwrap();
    let v = v113(&report);
    assert_eq!(v.len(), 1, "expected one §11.3 violation, got {v:?}");
    let msg = &v[0].message;
    assert!(
        !msg.contains("Open Graph"),
        "OG should not be missing: {msg}"
    );
    assert!(
        !msg.contains("Twitter Card"),
        "TC should not be missing: {msg}"
    );
    assert!(
        !msg.contains("JSON-LD"),
        "JSON-LD should not be missing: {msg}"
    );
    assert!(
        !msg.contains("sitemap.xml"),
        "sitemap should not be missing: {msg}"
    );
    assert!(
        !msg.contains("robots.txt"),
        "robots should not be missing: {msg}"
    );
    assert!(
        !msg.contains("llms.txt"),
        "llms.txt should not be missing: {msg}"
    );
    assert!(msg.contains("check-seo"), "check-seo missing: {msg}");
    assert!(msg.contains("lighthouse"), "lighthouse missing: {msg}");
}

#[test]
fn lighthouse_workflow_satisfies_seo_check() {
    // §11.3.10 — a workflow that runs `lhci autorun` and a separate one
    // that invokes the structural `check:seo` npm script together cover
    // the two enforcement signals even when the project does not check
    // in a `lighthouserc.json` or a standalone `check-seo.*` script.
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::write(
        root.join(".github/workflows/pages.yml"),
        "jobs:\n  deploy:\n    steps:\n      - uses: actions/deploy-pages@v4\n",
    )
    .unwrap();
    fs::write(
        root.join(".github/workflows/seo.yml"),
        "jobs:\n  seo-check:\n    steps:\n      - run: npm run check:seo\n",
    )
    .unwrap();
    fs::write(
        root.join(".github/workflows/lighthouse.yml"),
        "jobs:\n  lighthouse:\n    steps:\n      - run: lhci autorun\n",
    )
    .unwrap();
    // The Pages workflow alone is enough to trigger the §11.3 audit, so
    // we still need to satisfy the other six signals.
    fs::create_dir_all(root.join("web/scripts")).unwrap();
    fs::write(
        root.join("web/scripts/seo.mjs"),
        "// emits og:image, twitter:card, application/ld+json,
         // sitemap.xml, robots.txt, llms.txt
",
    )
    .unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v113(&report).is_empty(),
        "lhci + check:seo workflows must satisfy §11.3.10: {:?}",
        v113(&report)
    );
}

#[test]
fn signals_in_build_scripts_count() {
    // The validator's "directory-agnostic" promise: a project that emits
    // its meta tags from a build script's string literals satisfies the
    // check just as well as one that hard-codes them in HTML.
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("web/scripts")).unwrap();
    fs::write(
        root.join("web/index.html"),
        "<!doctype html><html><head></head></html>",
    )
    .unwrap();
    fs::write(
        root.join("web/scripts/seo.mjs"),
        r#"// emits og:image, twitter:card, application/ld+json,
           // sitemap.xml, robots.txt, llms.txt, plus references the
           // check-seo step and lhci wiring at build time
           const OG_IMAGE = "og:image";
           const TWITTER = "twitter:card";
           const JSONLD  = "application/ld+json";
           const SITEMAP = "sitemap.xml";
           const ROBOTS  = "robots.txt";
           const LLMS    = "llms.txt";
           const CHECK   = "check-seo";
           const LH      = "lhci";
"#,
    )
    .unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        v113(&report).is_empty(),
        "signals in build scripts must count: {:?}",
        v113(&report)
    );
}
