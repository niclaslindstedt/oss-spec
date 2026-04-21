//! Tests for the §23 interactive init-tailoring flow (`src/tailor.rs`).
//!
//! The tailoring agent talks to `zag` and is interactive, so full
//! end-to-end runs cannot live in CI. These tests cover what can be
//! verified deterministically:
//!
//! 1. The `tailor-init` prompt loads and renders (guarantees §13.5
//!    front-matter parity with its caller's Jinja context).
//! 2. `tailor::run` short-circuits on an empty / TODO-placeholder
//!    description without invoking zag.
//! 3. `--no-tailor` is surfaced by the CLI layer (both `--help` and
//!    the §12 agent-help dump).

use oss_spec::manifest::{Kind, Language, License, ProjectManifest};
use oss_spec::prompts;
use oss_spec::tailor;

fn tmpdir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn fixture(description: &str) -> ProjectManifest {
    let mut m = ProjectManifest::skeleton("demo", description);
    m.language = Language::Rust;
    m.kind = Kind::Cli;
    m.license = License::Mit;
    m
}

#[test]
fn tailor_init_prompt_loads_and_renders() {
    let p = prompts::load(
        "tailor-init",
        minijinja::context! {
            name => "demo",
            description => "A CLI for parsing structured logs from Kubernetes pods.",
            language => "rust",
            kind => "cli",
            license => "MIT",
            github_owner => "example",
            why_bullets => vec!["structured over grep", "pod-aware"],
            target_tree => "README.md\nsrc/\n  main.rs\n",
        },
    )
    .expect("tailor-init prompt should load");

    // §23.1 allow/denylist must be reiterated in the system prompt so the
    // agent has a steering guard alongside the human approval gate.
    assert!(
        p.system.contains("README.md"),
        "system prompt should enumerate README as an allowed edit target"
    );
    assert!(
        p.system.contains("src/**"),
        "system prompt should forbid src/"
    );
    assert!(
        p.system.contains("tests/**"),
        "system prompt should forbid tests/"
    );
    // The user section must render the manifest.
    assert!(p.user.contains("demo"));
    assert!(p.user.contains("Kubernetes"));
    assert!(p.user.contains("rust"));
    // No front matter should leak through.
    assert!(!p.system.contains("version: 1.0.0"));
}

#[tokio::test]
async fn tailor_run_skips_empty_description() {
    let tmp = tmpdir();
    let m = fixture("");
    // Expect Ok(()) and no zag invocation — nothing to assert about
    // the target beyond "no panic".
    tailor::run(&m, tmp.path(), /*assume_yes=*/ true)
        .await
        .expect("empty description should short-circuit");
}

#[tokio::test]
async fn tailor_run_skips_todo_placeholder_description() {
    let tmp = tmpdir();
    let m = fixture("TODO: describe demo");
    tailor::run(&m, tmp.path(), true)
        .await
        .expect("TODO placeholder description should short-circuit");
}

/// `--no-tailor` must be visible both in the clap `init --help` output
/// and in the §12 agent-help dump, so either surface is an acceptable
/// way for an agent to discover it.
#[test]
fn no_tailor_flag_is_discoverable() {
    use clap::CommandFactory;

    let mut cmd = oss_spec::cli::Cli::command();
    let init = cmd
        .find_subcommand_mut("init")
        .expect("init subcommand should exist");
    let help = init.render_long_help().to_string();
    assert!(
        help.contains("--no-tailor"),
        "--no-tailor should appear in `init --help`: {help}"
    );
    assert!(
        oss_spec::agent_help::HELP_AGENT.contains("--no-tailor"),
        "--no-tailor should be listed in the §12 agent-help dump"
    );
}
