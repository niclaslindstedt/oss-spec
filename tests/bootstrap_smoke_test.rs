//! Smoke test: bootstrap a deterministic Rust CLI into a tempdir, then assert
//! `check::run` reports zero violations and the expected key files exist.

use oss_spec::bootstrap;
use oss_spec::check;
use oss_spec::manifest::{Kind, Language, License, ProjectManifest};

fn fixture() -> ProjectManifest {
    let mut m = ProjectManifest::skeleton("smoke", "Smoke test project.");
    m.language = Language::Rust;
    m.kind = Kind::Cli;
    m.license = License::Mit;
    m.author_name = "Test User".into();
    m.author_email = "test@example.com".into();
    m.github_owner = "test-owner".into();
    m.year = 2026;
    m
}

#[test]
fn bootstrap_then_check_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("smoke");
    bootstrap::write(&fixture(), &target).expect("bootstrap write");

    // Spot-check critical paths.
    for rel in [
        "LICENSE",
        "README.md",
        "AGENTS.md",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        "SECURITY.md",
        "CHANGELOG.md",
        ".gitignore",
        ".editorconfig",
        "Makefile",
        "Cargo.toml",
        "src/main.rs",
        "src/lib.rs",
        ".github/workflows/ci.yml",
        ".github/workflows/version-bump.yml",
        ".github/workflows/release.yml",
        ".github/workflows/pages.yml",
        ".github/PULL_REQUEST_TEMPLATE.md",
        ".github/dependabot.yml",
        "docs/getting-started.md",
        "scripts/release.sh",
        "website/package.json",
        "man/main.md",
    ] {
        assert!(target.join(rel).exists(), "missing {rel}");
    }

    // Symlinks.
    for link in [
        "CLAUDE.md",
        ".cursorrules",
        ".windsurfrules",
        "GEMINI.md",
        ".github/copilot-instructions.md",
    ] {
        let p = target.join(link);
        assert!(p.is_symlink(), "{link} should be a symlink");
    }

    let report = check::run(&target).expect("check run");
    assert!(
        report.is_clean(),
        "generated repo should pass check: {:?}",
        report.violations
    );
}
