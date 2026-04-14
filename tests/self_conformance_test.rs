//! oss-spec must pass its own §19 conformance check.

use std::path::PathBuf;

#[test]
fn this_repo_conforms_to_oss_spec() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let report = oss_spec::check::run(&manifest_dir).expect("check run");
    assert!(
        report.is_clean(),
        "oss-spec is its own first customer; violations: {:?}",
        report.violations
    );
}
