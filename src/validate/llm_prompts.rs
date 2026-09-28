//! §13.5 — LLM prompts (`prompts/`).
//!
//! The rule applies only to a project that ships LLM prompts. The
//! validator cannot see whether source code calls a model, so the two
//! cases are told apart by what the tree and `AGENTS.md` say:
//!
//! - **A prompt in the tree** — a subdirectory of `prompts/`, one per
//!   logical prompt — means the project ships prompts, and every one of
//!   them must carry a versioned `<major>_<minor>_<patch>.md` file.
//! - **An `oss-spec:no-llm-prompts: <reason>` marker line in `AGENTS.md`**
//!   declares that the project sends no LLM prompts. `prompts/` is then
//!   not required, and a prompt subdirectory contradicts the marker.
//! - **Neither** — `prompts/` must exist, as before: a project that has
//!   not said it is prompt-free keeps the place its prompts belong.
//!
//! Mirrored in `scripts/validate.sh` (`declares_no_llm_prompts`,
//! `check_llm_prompts`) — keep both in lockstep when changing a rule (see
//! [`super`] for the parity policy).

use super::{Report, Violation};
use anyhow::{Context, Result};
use std::path::Path;

/// The declaration token. Same shape as the §11.3.12
/// `oss-spec:unlisted-website:` marker: the token, then a non-empty reason.
pub const NO_LLM_PROMPTS_MARKER: &str = "oss-spec:no-llm-prompts:";

/// Spec section every violation from this module is filed under.
const SECTION: &str = "§13.5";

/// `true` if `AGENTS.md` at the repository root declares that the project
/// sends no LLM prompts: a line carrying [`NO_LLM_PROMPTS_MARKER`]
/// followed by a non-empty reason. A marker without a reason does not
/// count.
pub fn declares_no_llm_prompts(root: &Path) -> bool {
    std::fs::read_to_string(root.join("AGENTS.md"))
        .map(|content| has_no_llm_prompts_marker(&content))
        .unwrap_or(false)
}

/// `true` if any line of `content` carries the marker with a reason.
pub fn has_no_llm_prompts_marker(content: &str) -> bool {
    content.lines().any(|line| {
        line.find(NO_LLM_PROMPTS_MARKER).is_some_and(|idx| {
            line[idx + NO_LLM_PROMPTS_MARKER.len()..]
                .chars()
                .any(|c| !c.is_whitespace())
        })
    })
}

/// The prompts shipped in the tree: the names of the subdirectories of
/// `prompts/`, sorted. Empty when `prompts/` is absent or holds only
/// loose files such as a README.
pub fn shipped_prompts(root: &Path) -> Result<Vec<String>> {
    let prompts_root = root.join("prompts");
    if !prompts_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = std::fs::read_dir(&prompts_root)
        .with_context(|| format!("read {}", prompts_root.display()))?
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .collect();
    names.sort();
    Ok(names)
}

/// §13.5 checks: `prompts/` is required unless `AGENTS.md` declares the
/// project prompt-free, a declared project ships no prompt, and every
/// shipped prompt has a versioned file.
pub(super) fn check(path: &Path, report: &mut Report) -> Result<()> {
    let prompt_free = declares_no_llm_prompts(path);
    if !path.join("prompts").is_dir() {
        if !prompt_free {
            report.violations.push(Violation {
                spec_section: SECTION,
                message: format!(
                    "missing directory prompts (a project that sends no LLM prompts \
                     declares `{NO_LLM_PROMPTS_MARKER} <reason>` in AGENTS.md instead)"
                ),
            });
        }
        return Ok(());
    }

    for name in shipped_prompts(path)? {
        if prompt_free {
            report.violations.push(Violation {
                spec_section: SECTION,
                message: format!(
                    "AGENTS.md declares that the project sends no LLM prompts, but \
                     prompts/{name}/ ships one; remove the prompt or the marker"
                ),
            });
        }
        if !has_versioned_file(&path.join("prompts").join(&name)) {
            report.violations.push(Violation {
                spec_section: SECTION,
                message: format!(
                    "prompts/{name}/ has no versioned <major>_<minor>_<patch>.md file"
                ),
            });
        }
    }
    Ok(())
}

/// `true` if `dir` holds a `<major>_<minor>_<patch>.md` file.
fn has_versioned_file(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|it| {
            it.flatten().any(|e| {
                let f = e.path();
                f.extension().and_then(|s| s.to_str()) == Some("md")
                    && f.file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(crate::prompts::parse_version)
                        .is_some()
            })
        })
        .unwrap_or(false)
}
