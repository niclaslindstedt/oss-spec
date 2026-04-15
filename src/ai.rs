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

/// Thin wrapper so we can share an `Arc<OutputProgress>` with zag while
/// retaining access to the spinner after `.exec()` returns.
struct ProgressForwarder(std::sync::Arc<OutputProgress>);

impl zag::progress::ProgressHandler for ProgressForwarder {
    fn on_status(&self, m: &str) {
        self.0.on_status(m);
    }
    fn on_success(&self, m: &str) {
        self.0.on_success(m);
    }
    fn on_warning(&self, m: &str) {
        self.0.on_warning(m);
    }
    fn on_error(&self, m: &str) {
        self.0.on_error(m);
    }
    fn on_spinner_start(&self, m: &str) {
        self.0.on_spinner_start(m);
    }
    fn on_spinner_finish(&self) {
        self.0.on_spinner_finish();
    }
    fn on_debug(&self, m: &str) {
        self.0.on_debug(m);
    }
}

/// Routes zag progress callbacks through `crate::output`.
///
/// Manages an optional spinner: zag calls `on_spinner_start` /
/// `on_spinner_finish` around the agent-creation phase, then
/// `on_success` once the agent is ready. When `auto_spinner` is true
/// we start a "Waiting for AI response…" spinner after the init
/// phase completes so the user sees activity during the API call.
struct OutputProgress {
    spinner: std::sync::Mutex<Option<crate::output::Spinner>>,
    /// Start a "Waiting for AI response…" spinner after init success.
    auto_spinner: bool,
}

impl OutputProgress {
    fn new(auto_spinner: bool) -> Self {
        Self {
            spinner: std::sync::Mutex::new(None),
            auto_spinner,
        }
    }

    /// Stop the spinner with a success or failure message.
    fn finish_spinner(&self, msg: &str, success: bool) {
        if let Some(spinner) = self.spinner.lock().unwrap().take() {
            if success {
                spinner.finish(msg);
            } else {
                spinner.fail(msg);
            }
        }
    }
}

impl zag::progress::ProgressHandler for OutputProgress {
    fn on_status(&self, message: &str) {
        crate::output::info(message);
    }
    fn on_success(&self, message: &str) {
        crate::output::status(message);
        if self.auto_spinner {
            // Agent init is done — start a spinner for the API call.
            let mut guard = self.spinner.lock().unwrap();
            if guard.is_none() {
                *guard = Some(crate::output::Spinner::start("Waiting for AI response..."));
            }
        }
    }
    fn on_warning(&self, message: &str) {
        crate::output::warn(message);
    }
    fn on_error(&self, message: &str) {
        crate::output::error(message);
    }
    fn on_spinner_start(&self, message: &str) {
        let mut guard = self.spinner.lock().unwrap();
        *guard = Some(crate::output::Spinner::start(message));
    }
    fn on_spinner_finish(&self) {
        if let Some(spinner) = self.spinner.lock().unwrap().take() {
            spinner.clear();
        }
    }
    fn on_debug(&self, message: &str) {
        log::debug!("[zag] {message}");
    }
}

/// Parse a freeform user prompt (e.g. "create a python cli for finding stock buys")
/// into a partially-filled ProjectManifest.
pub async fn interpret_prompt(prompt: &str) -> Result<ProjectManifest> {
    log::debug!("interpret_prompt: sending freeform prompt to zag");
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
    log::debug!(
        "interpret_prompt: system prompt length={}, user prompt length={}",
        p.system.len(),
        p.user.len()
    );
    let raw = run_zag_json(&p.system, &p.user, schema).await?;
    log::debug!("interpret_prompt: zag response: {raw}");

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
    log::debug!("draft_readme_why: generating bullets for {name}");
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
    log::debug!(
        "draft_readme_why: system prompt length={}, user prompt length={}",
        p.system.len(),
        p.user.len()
    );
    let raw = run_zag_json(&p.system, &p.user, schema).await?;
    log::debug!("draft_readme_why: zag response: {raw}");

    #[derive(Deserialize)]
    struct Wire {
        bullets: Vec<String>,
    }
    let wire: Wire = serde_json::from_str(&raw)
        .with_context(|| format!("zag returned non-conforming JSON: {raw}"))?;
    Ok(wire.bullets)
}

/// Drive a zag agent loop in `repo` to remove every §19 violation
/// and address AI quality findings.
pub async fn fix_conformance(
    repo: &Path,
    report: &crate::check::Report,
    max_turns: u32,
) -> Result<()> {
    let violations_text = if report.ai_findings.is_empty() {
        format_violations(report)
    } else {
        format_full_report(report)
    };
    let p = crate::prompts::load(
        "fix-conformance",
        context! { violations => violations_text },
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

/// Format the full report (structural violations + AI findings) for the
/// fix agent prompt.
fn format_full_report(report: &crate::check::Report) -> String {
    let mut out = String::new();
    if !report.violations.is_empty() {
        out.push_str("## Structural violations\n\n");
        for (i, v) in report.violations.iter().enumerate() {
            out.push_str(&format!(
                "{:>2}. [{}] {}\n",
                i + 1,
                v.spec_section,
                v.message
            ));
        }
    }
    if !report.ai_findings.is_empty() {
        out.push_str("\n## Quality findings (from AI review)\n\n");
        for (i, f) in report.ai_findings.iter().enumerate() {
            out.push_str(&format!(
                "{:>2}. [{}] [{}] {}: {}\n    Suggestion: {}\n",
                i + 1,
                f.severity,
                f.spec_section,
                f.file,
                f.message,
                f.suggestion
            ));
        }
    }
    out
}

/// One-shot JSON review of file contents against OSS_SPEC.md. Returns
/// structured quality findings that the CLI can display alongside
/// structural violations.
pub async fn verify_conformance(
    file_contents: &[(String, String)],
    existing_violations: &[crate::check::Violation],
) -> Result<Vec<crate::check::AiFinding>> {
    log::debug!(
        "verify_conformance: {} files to review",
        file_contents.len()
    );

    // Format file contents for the prompt.
    let mut contents_block = String::new();
    for (name, content) in file_contents {
        contents_block.push_str(&format!(
            "=== FILE: {name} ===\n{content}\n=== END: {name} ===\n\n"
        ));
    }

    // Format existing violations so the AI can skip them.
    let violations_text = if existing_violations.is_empty() {
        "(none)".to_string()
    } else {
        existing_violations
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{:>2}. [{}] {}", i + 1, v.spec_section, v.message))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let schema = serde_json::json!({
        "type": "object",
        "required": ["findings"],
        "properties": {
            "findings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["file", "spec_section", "severity", "message", "suggestion"],
                    "properties": {
                        "file": { "type": "string", "description": "Relative path of the file" },
                        "spec_section": { "type": "string", "description": "Spec section, e.g. §3" },
                        "severity": { "type": "string", "enum": ["error", "warning"] },
                        "message": { "type": "string", "description": "What is wrong" },
                        "suggestion": { "type": "string", "description": "How to fix it" }
                    }
                }
            }
        }
    });

    let p = crate::prompts::load(
        "verify-conformance",
        context! {
            spec => crate::embedded::OSS_SPEC,
            violations => violations_text,
            file_contents => contents_block,
        },
    )?;
    log::debug!(
        "verify_conformance: system={}B, user={}B",
        p.system.len(),
        p.user.len()
    );

    let raw = match run_zag_json(&p.system, &p.user, schema).await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("AI verification failed (non-fatal): {e:#}");
            return Ok(vec![]);
        }
    };
    log::debug!("verify_conformance: response: {raw}");

    #[derive(Deserialize)]
    struct Wire {
        findings: Vec<crate::check::AiFinding>,
    }

    match serde_json::from_str::<Wire>(&raw) {
        Ok(wire) => Ok(wire.findings),
        Err(e) => {
            log::warn!("AI verification returned unparseable JSON: {e}");
            Ok(vec![])
        }
    }
}

/// One-shot JSON request — used by `interpret_prompt` and `draft_readme_why`.
async fn run_zag_json(system: &str, prompt: &str, schema: serde_json::Value) -> Result<String> {
    use zag::builder::AgentBuilder;

    log::debug!("starting zag one-shot JSON request");
    log::debug!(
        "zag request: system_prompt={}B, user_prompt={}B, schema={}",
        system.len(),
        prompt.len(),
        schema
    );
    let progress = std::sync::Arc::new(OutputProgress::new(true));
    let result = AgentBuilder::new()
        .system_prompt(system)
        .auto_approve(true)
        .json_schema(schema)
        .verbose(true)
        .on_progress(Box::new(ProgressForwarder(progress.clone())))
        .exec(prompt)
        .await
        .context("zag agent execution failed");

    match result {
        Ok(output) => {
            progress.finish_spinner("AI response received", true);
            log::debug!(
                "zag output: agent={}, session={}, is_error={}, usage={:?}, result={:?}",
                output.agent,
                output.session_id,
                output.is_error,
                output.usage,
                output.result
            );
            output
                .result
                .ok_or_else(|| anyhow!("zag returned no result text"))
        }
        Err(e) => {
            progress.finish_spinner("AI request failed", false);
            log::debug!("zag error: {e:#}");
            Err(e)
        }
    }
}

/// Agentic loop with a working root and a turn budget — used by the
/// `fix` subcommand. Runs interactively so the user can observe (and
/// participate in) the agent's activity.
async fn run_zag_agent(system: &str, user_prompt: &str, root: &Path, max_turns: u32) -> Result<()> {
    use zag::builder::AgentBuilder;

    log::debug!(
        "starting zag agent loop in {} (max_turns={})",
        root.display(),
        max_turns
    );
    let root_str = root.to_string_lossy();
    crate::output::info(&format!(
        "Starting agent loop in {} (max {} turns)...",
        root.display(),
        max_turns
    ));
    log::debug!(
        "zag agent: system_prompt={}B, user_prompt={}B",
        system.len(),
        user_prompt.len()
    );
    AgentBuilder::new()
        .system_prompt(system)
        .root(&root_str)
        .auto_approve(true)
        .max_turns(max_turns)
        .verbose(true)
        .on_progress(Box::new(OutputProgress::new(false)))
        .run(Some(user_prompt))
        .await
        .context("zag agent execution failed")?;
    log::debug!("zag agent loop completed");
    Ok(())
}
