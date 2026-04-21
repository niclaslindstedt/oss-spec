//! `oss-spec init` — interactive post-bootstrap tailoring (§23).
//!
//! After `bootstrap::write()` materializes a project from templates, this
//! module optionally launches a `zag` agent in the target directory so
//! the user can walk it through tailoring the **scaffolding layer**
//! (README, AGENTS.md, docs, skills, workflows) to match the project
//! description. The agent is strictly interactive: each file edit is
//! surfaced to the user for approval.
//!
//! Skipped under `--no-ai`, `--no-tailor`, or when the project has no
//! meaningful description to tailor against. All zag interaction is
//! owned by `crate::ai::tailor_init`; this module is just the
//! user-facing orchestration.

use anyhow::Result;
use std::path::Path;

use crate::manifest::ProjectManifest;

/// Run the interactive tailoring pass. Returns `Ok(())` on success,
/// on user-declined launch, and when zag surfaces no edits. Agent
/// failures are demoted to a warning so the rest of `init` (git init,
/// gh repo create) still runs.
pub async fn run(manifest: &ProjectManifest, target: &Path, assume_yes: bool) -> Result<()> {
    if manifest.description.trim().is_empty() || manifest.description.starts_with("TODO:") {
        log::debug!("tailor: skipping — description is empty or TODO placeholder");
        return Ok(());
    }

    crate::output::header("Tailoring project scaffolding");
    crate::output::info(
        "About to launch an interactive zag agent that proposes edits to README.md,",
    );
    crate::output::info(
        "AGENTS.md, docs/, .agent/skills/, and .github/workflows/ so they read as if",
    );
    crate::output::info(
        "they were written for this specific project. Application source (src/, tests/)",
    );
    crate::output::info(
        "is off-limits. You'll be asked to approve each file change before it lands.",
    );
    crate::output::info("(Skip with --no-tailor, or --no-ai to skip all AI.)");

    if !assume_yes {
        let proceed = dialoguer::Confirm::new()
            .with_prompt("Launch tailoring agent?")
            .default(true)
            .interact()
            .unwrap_or(false);
        if !proceed {
            crate::output::info("Skipping tailoring.");
            return Ok(());
        }
    }

    if let Err(e) = crate::ai::tailor_init(manifest, target).await {
        crate::output::warn(&format!("tailoring skipped: {e}"));
        return Ok(());
    }

    crate::output::status("Tailoring complete");
    Ok(())
}
