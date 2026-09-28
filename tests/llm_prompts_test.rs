//! §13.5 — `prompts/` applies only to a project that ships LLM prompts.
//!
//! A project declares that it sends no LLM prompts with an
//! `oss-spec:no-llm-prompts: <reason>` line in AGENTS.md. The validator
//! then stops requiring `prompts/` and instead reports any prompt the
//! tree ships anyway. Projects without the marker are validated exactly
//! as before: `prompts/` must exist, and every prompt in it is versioned.

use oss_spec::validate::{
    self, declares_no_llm_prompts, has_no_llm_prompts_marker, shipped_prompts,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

const MARKER_LINE: &str = "oss-spec:no-llm-prompts: the app has no AI features\n";

const PROMPT: &str = "---\nname: summarize\ndescription: \"Summarize a note.\"\n\
                      version: 1.0.0\n---\n\n# summarize\n\n## System\n\nBe brief.\n\n\
                      ## User\n\n{{ note }}\n";

fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
}

/// The §13.5 violations `oss-spec validate` reports for `root`.
fn prompt_violations(root: &Path) -> Vec<String> {
    validate::run(root)
        .unwrap()
        .violations
        .into_iter()
        .filter(|v| v.spec_section == "§13.5")
        .map(|v| v.message)
        .collect()
}

// The marker --------------------------------------------------------------

#[test]
fn marker_with_reason_declares_prompt_free() {
    assert!(has_no_llm_prompts_marker(MARKER_LINE));
    assert!(has_no_llm_prompts_marker(&format!(
        "# Agent guidance\n\n<!-- {} -->\n",
        MARKER_LINE.trim()
    )));
    assert!(has_no_llm_prompts_marker(
        "This project is `oss-spec:no-llm-prompts: no model calls`.\n"
    ));
}

#[test]
fn marker_without_reason_does_not_declare() {
    assert!(!has_no_llm_prompts_marker("oss-spec:no-llm-prompts:\n"));
    assert!(!has_no_llm_prompts_marker("oss-spec:no-llm-prompts:    \n"));
    assert!(!has_no_llm_prompts_marker(
        "# Agent guidance\n\nNo marker here.\n"
    ));
}

#[test]
fn marker_is_read_from_agents_md_only() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    assert!(!declares_no_llm_prompts(root), "no AGENTS.md at all");
    write(root, "README.md", MARKER_LINE);
    assert!(
        !declares_no_llm_prompts(root),
        "the marker counts only in AGENTS.md"
    );
    write(root, "AGENTS.md", MARKER_LINE);
    assert!(declares_no_llm_prompts(root));
}

#[test]
fn shipped_prompts_are_the_subdirectories_of_prompts() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    assert!(shipped_prompts(root).unwrap().is_empty(), "no prompts/");
    write(root, "prompts/README.md", "# prompts\n");
    assert!(
        shipped_prompts(root).unwrap().is_empty(),
        "a README alone ships no prompt"
    );
    write(root, "prompts/tag/1_0_0.md", PROMPT);
    write(root, "prompts/summarize/1_0_0.md", PROMPT);
    assert_eq!(shipped_prompts(root).unwrap(), ["summarize", "tag"]);
}

// A project that ships no prompts -----------------------------------------

#[test]
fn prompt_free_project_needs_no_prompts_dir() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    assert_eq!(prompt_violations(root), Vec::<String>::new());
}

#[test]
fn prompt_free_project_may_keep_a_prompts_readme() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    write(root, "prompts/README.md", "# prompts\n");
    assert_eq!(prompt_violations(root), Vec::<String>::new());
}

#[test]
fn prompt_free_project_that_ships_a_prompt_is_violation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    write(root, "prompts/summarize/1_0_0.md", PROMPT);
    let v = prompt_violations(root);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(
        v[0].contains("sends no LLM prompts") && v[0].contains("prompts/summarize/"),
        "{v:?}"
    );
}

// A project that has not declared itself prompt-free ----------------------

#[test]
fn undeclared_project_without_prompts_dir_is_violation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "# Agent guidance\n");
    let v = prompt_violations(root);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].starts_with("missing directory prompts"), "{v:?}");
    assert!(
        v[0].contains("oss-spec:no-llm-prompts:"),
        "the message must name the way out: {v:?}"
    );
}

#[test]
fn marker_without_reason_is_validated_as_undeclared() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "oss-spec:no-llm-prompts:\n");
    let v = prompt_violations(root);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].starts_with("missing directory prompts"), "{v:?}");
}

#[test]
fn project_that_ships_versioned_prompts_passes() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "# Agent guidance\n");
    write(root, "prompts/summarize/1_0_0.md", PROMPT);
    write(root, "prompts/summarize/1_1_0.md", PROMPT);
    assert_eq!(prompt_violations(root), Vec::<String>::new());
}

#[test]
fn shipped_prompt_without_versioned_file_is_violation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "# Agent guidance\n");
    write(root, "prompts/summarize/latest.md", PROMPT);
    let v = prompt_violations(root);
    assert_eq!(
        v,
        ["prompts/summarize/ has no versioned <major>_<minor>_<patch>.md file"]
    );
}
