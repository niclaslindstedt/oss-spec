use oss_spec::bootstrap::symlink_file;
use oss_spec::check::{self, check_toolchain_versions, version_ge};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn version_ge_pads_shorter_segments() {
    assert_eq!(version_ge("1.82", "1.82.0"), Some(true));
    assert_eq!(version_ge("1.82.0", "1.82"), Some(true));
    assert_eq!(version_ge("1.83", "1.82.0"), Some(true));
    assert_eq!(version_ge("1.81.9", "1.82.0"), Some(false));
    assert_eq!(version_ge("24", "24"), Some(true));
    assert_eq!(version_ge("23", "24"), Some(false));
}

#[test]
fn rust_pinned_exact_minimum_is_ok() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.82.0\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn rust_pinned_above_minimum_is_ok() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.85.0\n";
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
    assert!(v[0].message.contains("minimum is 1.82.0"));
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
    // `@1.82` (no patch) should be treated as `1.82.0` and accepted.
    let yml = "      - uses: dtolnay/rust-toolchain@1.82\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

// --- §20 test organization checks ---

/// Helper: create a minimal repo skeleton that passes all non-§20 checks,
/// so we can isolate §20 violations.
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
