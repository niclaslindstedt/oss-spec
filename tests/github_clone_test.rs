//! Hermetic test for `git::clone_repo` + `validate::run`: bootstrap a
//! compliant repo into a temp dir, `git init`/commit it, then clone it via a
//! `file://` URL (no network) and validate the clone.

use oss_spec::bootstrap;
use oss_spec::git;
use oss_spec::manifest::{Kind, Language, License, ProjectManifest};
use oss_spec::validate;
use std::process::Command;

fn fixture() -> ProjectManifest {
    let mut m = ProjectManifest::skeleton("clone-check", "Clone+check smoke project.");
    m.language = Language::Rust;
    m.kind = Kind::Cli;
    m.license = License::Mit;
    m.author_name = "Test User".into();
    m.author_email = "test@example.com".into();
    m.github_owner = "test-owner".into();
    m.year = 2026;
    m
}

fn run_git(cwd: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("spawn git");
    assert!(status.success(), "git {args:?} failed: {status}");
}

#[test]
fn clone_repo_then_check_passes_on_file_url() {
    // 1. Bootstrap a compliant repo into a temp dir.
    let src = tempfile::tempdir().expect("tempdir");
    let src_repo = src.path().join("clone-check");
    bootstrap::write(&fixture(), &src_repo).expect("bootstrap write");

    // 2. `git init` + a single commit so it's a valid clone source. Disable
    //    gpg/commit signing in case the test environment has it globally
    //    enabled — this test only needs a local, throwaway commit.
    run_git(&src_repo, &["init", "-q", "-b", "main"]);
    let commit_args: &[&str] = &[
        "-c",
        "user.email=t@t",
        "-c",
        "user.name=t",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "tag.gpgsign=false",
    ];
    let mut add_args = commit_args.to_vec();
    add_args.extend_from_slice(&["add", "."]);
    run_git(&src_repo, &add_args);
    let mut commit = commit_args.to_vec();
    commit.extend_from_slice(&["commit", "-qm", "init"]);
    run_git(&src_repo, &commit);

    // 3. Clone via file:// URL using the generic clone helper. We can't use
    //    --depth 1 against a file:// source without file://-protocol support,
    //    so request a full (non-shallow) clone.
    let url = format!("file://{}", src_repo.display());
    let dest = git::clone_repo(&url, None, false, "oss-spec-test").expect("clone_repo");

    // 4. The cloned repo must pass `check`.
    let report = validate::run(&dest).expect("validate::run");
    assert!(
        report.is_clean(),
        "cloned repo should pass validate: {:?}",
        report.violations
    );

    std::fs::remove_dir_all(&dest).ok();
}
