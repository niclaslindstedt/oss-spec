//! `oss-spec fix` — bring an existing repo into OSS_SPEC.md conformance.
//!
//! Runs the §19 checker, then dispatches a zag-driven agent to either fix
//! the violations in place (default) or file one GitHub issue per violation
//! cluster (`--create-issues`). All file IO and `gh` calls happen *inside*
//! the agent loop via zag's built-in tools — this module is just the
//! orchestrator.

use anyhow::Result;
use std::path::Path;

pub async fn run(path: &Path, create_issues: bool, max_turns: u32, assume_yes: bool) -> Result<()> {
    log::debug!(
        "fix: checking {} (create_issues={create_issues})",
        path.display()
    );
    let report = crate::check::run(path)?;
    if report.is_clean() {
        crate::output::status("repo already conforms — nothing to do");
        return Ok(());
    }
    report.print();

    if !assume_yes {
        let prompt = if create_issues {
            format!("File {} GitHub issue(s) via gh?", report.violations.len())
        } else {
            format!(
                "Launch zag agent to fix {} violation(s) in {}?",
                report.violations.len(),
                path.display()
            )
        };
        let proceed = dialoguer::Confirm::new()
            .with_prompt(prompt)
            .default(true)
            .interact()
            .unwrap_or(false);
        if !proceed {
            return Ok(());
        }
    }

    if create_issues {
        crate::output::header("Filing GitHub issues via agent");
        crate::ai::file_conformance_issues(path, &report, max_turns).await?;
    } else {
        crate::output::header("Launching fix agent");
        crate::ai::fix_conformance(path, &report, max_turns).await?;
    }

    crate::output::info("\nRe-running check...");
    let after = crate::check::run(path)?;
    after.print();
    if !after.is_clean() && !create_issues {
        std::process::exit(1);
    }
    Ok(())
}
