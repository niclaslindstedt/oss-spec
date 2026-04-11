//! Thin wrappers around `zag` for the LLM-driven steps.
//!
//! Two patterns live here:
//!
//! * **One-shot JSON** (`interpret_prompt`, `draft_readme_why`) — used by the
//!   bootstrap flow. Sends a single prompt with a JSON schema and parses the
//!   response.
//! * **Agent loop** (`fix_conformance`, `file_conformance_issues`) — used by
//!   `oss-spec fix`. Hands zag a working root + a turn budget and lets it
//!   drive Edit/Write/Bash tools until done.
//!
//! All prompt text lives under `prompts/<name>/<version>.md` and is loaded
//! through `crate::prompts`. AI failures are non-fatal: callers always have a
//! deterministic fallback so `--no-ai` keeps working.

use anyhow::{Context, Result, anyhow};
use minijinja::context;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

use crate::manifest::{Kind, Language, License, ProjectManifest};

/// Parse a freeform user prompt (e.g. "create a python cli for finding stock buys")
/// into a partially-filled ProjectManifest.
pub async fn interpret_prompt(prompt: &str) -> Result<ProjectManifest> {
    let schema = json!({
        "type": "object",
        "required": ["name", "description", "language", "kind", "license"],
        "properties": {
            "name":        { "type": "string", "description": "Short kebab-case project name." },
            "description": { "type": "string", "description": "One concise sentence." },
            "language":    { "type": "string", "enum": ["rust","python","node","go","generic"] },
            "kind":        { "type": "string", "enum": ["lib","cli","service"] },
            "license":     { "type": "string", "enum": ["MIT","Apache-2.0","MPL-2.0"] },
            "why_bullets": {
                "type": "array",
                "items": { "type": "string" },
                "description": "3-5 concrete value propositions for the README 'Why?' section.",
                "maxItems": 5
            }
        }
    });

    let p = crate::prompts::load("interpret-prompt", context! { prompt => prompt })?;
    let raw = run_zag_json(&p.system, &p.user, schema).await?;

    #[derive(Deserialize)]
    struct Wire {
        name: String,
        description: String,
        language: String,
        kind: String,
        license: String,
        #[serde(default)]
        why_bullets: Vec<String>,
    }

    let wire: Wire = serde_json::from_str(&raw)
        .with_context(|| format!("zag returned non-conforming JSON: {raw}"))?;

    let mut m = ProjectManifest::skeleton(&wire.name, &wire.description);
    m.language = Language::parse(&wire.language).unwrap_or(Language::Generic);
    m.kind = Kind::parse(&wire.kind).unwrap_or(Kind::Cli);
    m.license = License::parse(&wire.license).unwrap_or(License::Mit);
    m.why_bullets = wire.why_bullets;
    Ok(m)
}

/// Generate 3–5 README "Why?" bullet points for a project.
pub async fn draft_readme_why(description: &str, name: &str) -> Result<Vec<String>> {
    let schema = json!({
        "type": "object",
        "required": ["bullets"],
        "properties": {
            "bullets": {
                "type": "array",
                "items": { "type": "string" },
                "minItems": 3,
                "maxItems": 5
            }
        }
    });
    let p = crate::prompts::load(
        "draft-readme-why",
        context! { name => name, description => description },
    )?;
    let raw = run_zag_json(&p.system, &p.user, schema).await?;

    #[derive(Deserialize)]
    struct Wire {
        bullets: Vec<String>,
    }
    let wire: Wire = serde_json::from_str(&raw)
        .with_context(|| format!("zag returned non-conforming JSON: {raw}"))?;
    Ok(wire.bullets)
}

/// Drive a zag agent loop in `repo` to remove every §19 violation.
pub async fn fix_conformance(
    repo: &Path,
    report: &crate::check::Report,
    max_turns: u32,
) -> Result<()> {
    let p = crate::prompts::load(
        "fix-conformance",
        context! { violations => format_violations(report) },
    )?;
    run_zag_agent(&p.system, &p.user, repo, max_turns).await
}

/// Drive a zag agent loop in `repo` to file one GitHub issue per
/// violation cluster (via `gh`).
pub async fn file_conformance_issues(
    repo: &Path,
    report: &crate::check::Report,
    max_turns: u32,
) -> Result<()> {
    let p = crate::prompts::load(
        "file-conformance-issues",
        context! { violations => format_violations(report) },
    )?;
    run_zag_agent(&p.system, &p.user, repo, max_turns).await
}

fn format_violations(report: &crate::check::Report) -> String {
    report
        .violations
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{:>2}. [{}] {}", i + 1, v.spec_section, v.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One-shot JSON request — used by `interpret_prompt` and `draft_readme_why`.
async fn run_zag_json(system: &str, prompt: &str, schema: serde_json::Value) -> Result<String> {
    use zag::builder::AgentBuilder;

    let output = AgentBuilder::new()
        .system_prompt(system)
        .auto_approve(true)
        .json_schema(schema)
        .exec(prompt)
        .await
        .context("zag agent execution failed")?;

    output
        .result
        .ok_or_else(|| anyhow!("zag returned no result text"))
}

/// Agentic loop with a working root and a turn budget — used by the
/// `fix` subcommand. No JSON schema: the agent is expected to use its
/// built-in Edit/Write/Bash tools, not return structured data.
async fn run_zag_agent(system: &str, user_prompt: &str, root: &Path, max_turns: u32) -> Result<()> {
    use zag::builder::AgentBuilder;

    let root_str = root.to_string_lossy();
    AgentBuilder::new()
        .system_prompt(system)
        .root(&root_str)
        .auto_approve(true)
        .max_turns(max_turns)
        .exec(user_prompt)
        .await
        .context("zag agent execution failed")?;
    Ok(())
}
