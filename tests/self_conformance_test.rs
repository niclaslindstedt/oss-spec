//! oss-spec must pass its own §19 conformance validation.

use std::path::PathBuf;

#[test]
fn this_repo_conforms_to_oss_spec() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let report = oss_spec::validate::run(&manifest_dir).expect("validate run");
    if !report.is_clean() {
        // Emit GitHub Actions error annotations so the violations show up
        // in the PR checks API even when the raw job logs aren't visible.
        for v in &report.violations {
            eprintln!("::error::[{}] {}", v.spec_section, v.message);
        }
    }
    assert!(
        report.is_clean(),
        "oss-spec is its own first customer; violations: {:?}",
        report.violations
    );
}
