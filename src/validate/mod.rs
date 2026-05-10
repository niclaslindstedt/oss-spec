//! `oss-spec validate` — validate an existing repo against the §19 checklist.
//!
//! Two layers of checking:
//!
//! 1. **Structural** — file-presence, symlink, and directory checks (deterministic).
//! 2. **AI quality** — one-shot LLM review of file *contents* against OSS_SPEC.md
//!    (enabled by default, skipped with `--no-ai`).
//!
//! ## Parity with `scripts/validate.sh`
//!
//! Every deterministic check in this module is mirrored in
//! [`scripts/validate.sh`](../../scripts/validate.sh) so that agents working
//! in environments where the Rust binary cannot be installed (sandboxed
//! sessions, ephemeral CI runners) can still verify §19 conformance via
//! `curl … | bash`. **Whenever you add, remove, or modify a rule in any
//! submodule below — `structural`, `content`, `toolchain`, `agent_skills`
//! — make the equivalent edit in `scripts/validate.sh` in the same PR.**
//! There is no automated drift check between the two implementations;
//! reviewers verify parity by hand.

use anyhow::{Context, Result};
use std::path::Path;

mod agent_skills;
mod content;
mod structural;
mod toolchain;

pub use agent_skills::{extract_front_matter, has_yaml_key, is_kebab_case};
pub use toolchain::{
    check_local_toolchain_pin, check_toolchain_versions, find_rust_ci_version, find_setup_version,
    parse_go_toolchain, parse_rust_channel, version_ge, versions_same_major_minor,
};

#[derive(Debug, Clone)]
pub struct Violation {
    pub spec_section: &'static str,
    pub message: String,
}

/// A quality/content finding produced by AI review. Unlike [`Violation`],
/// these represent issues that require human or AI judgment to detect
/// (e.g. placeholder text, missing required sections in README).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiFinding {
    pub file: String,
    pub spec_section: String,
    pub severity: String,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Default)]
pub struct Report {
    pub violations: Vec<Violation>,
    pub ai_findings: Vec<AiFinding>,
}

impl Report {
    /// Returns `true` if there are no structural violations.
    /// AI findings do **not** affect this — they are advisory only.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn print(&self) {
        if self.violations.is_empty() && self.ai_findings.is_empty() {
            crate::output::status("repo conforms to OSS_SPEC.md");
            return;
        }
        if !self.violations.is_empty() {
            crate::output::error(&format!("{} violations:", self.violations.len()));
            for (i, v) in self.violations.iter().enumerate() {
                crate::output::info(&format!(
                    "  {:>2}. [{}] {}",
                    i + 1,
                    v.spec_section,
                    v.message
                ));
            }
        }
        if !self.ai_findings.is_empty() {
            crate::output::header("AI quality findings:");
            let errors: Vec<_> = self
                .ai_findings
                .iter()
                .filter(|f| f.severity == "error")
                .collect();
            let warnings: Vec<_> = self
                .ai_findings
                .iter()
                .filter(|f| f.severity != "error")
                .collect();
            let mut idx = 0;
            for f in errors.iter().chain(warnings.iter()) {
                idx += 1;
                let sev = if f.severity == "error" { "ERR" } else { "WARN" };
                crate::output::info(&format!(
                    "  {:>2}. [{sev}] [{}] {}: {}",
                    idx, f.spec_section, f.file, f.message
                ));
                crate::output::info(&format!("      Suggestion: {}", f.suggestion));
            }
        }
    }
}

pub fn run(path: &Path) -> Result<Report> {
    log::debug!("checking conformance at {}", path.display());
    crate::output::info(
        "note: bringing a repo fully into spec usually takes a few `oss-spec validate` runs — \
         fixing one violation (e.g. creating a missing file) often uncovers the next layer \
         (e.g. that file not yet being complete). Re-run until the report is clean.",
    );
    let path = path
        .canonicalize()
        .with_context(|| format!("cannot canonicalize {}", path.display()))?;

    let mut report = Report::default();

    structural::check(&path, &mut report)?;
    content::check(&path, &mut report)?;
    agent_skills::check(&path, &mut report);

    Ok(report)
}

/// Maximum number of lines to include per file in the AI verification prompt.
const MAX_LINES_PER_FILE: usize = 200;

/// All spec-relevant files whose content should be sent to the AI for quality
/// review. Paths are relative to the repo root.
const VERIFIABLE_FILES: &[&str] = &[
    "LICENSE",
    "README.md",
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md",
    "SECURITY.md",
    "AGENTS.md",
    "CHANGELOG.md",
    ".editorconfig",
    "Makefile",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    ".github/workflows/version-bump.yml",
    ".github/workflows/pages.yml",
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/ISSUE_TEMPLATE/feature_request.md",
    ".github/dependabot.yml",
];

/// Read the content of every spec-relevant file that exists on disk.
/// Each file is truncated to [`MAX_LINES_PER_FILE`] lines to keep the
/// prompt size manageable.
pub fn gather_file_contents(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for &relpath in VERIFIABLE_FILES {
        let full = root.join(relpath);
        if let Ok(raw) = std::fs::read_to_string(&full) {
            let truncated: String = raw
                .lines()
                .take(MAX_LINES_PER_FILE)
                .collect::<Vec<_>>()
                .join("\n");
            out.push((relpath.to_string(), truncated));
        }
    }
    out
}
