//! Thin wrappers around `git` and `gh` so the bootstrap flow can land a first
//! commit and (optionally) create the GitHub remote. All commands run in the
//! target directory and propagate failures verbatim.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use dialoguer::Confirm;

pub fn init_and_commit(target: &Path) -> Result<()> {
    if target.join(".git").exists() {
        log::debug!("git repo already exists at {}", target.display());
        return Ok(()); // already a repo, leave alone
    }
    log::debug!("initialising git repo at {}", target.display());
    run(target, "git", &["init", "-b", "main"])?;
    run(target, "git", &["add", "."])?;
    run(
        target,
        "git",
        &["commit", "-m", "chore: bootstrap project from oss-spec"],
    )?;
    Ok(())
}

/// Create a GitHub remote via the `gh` CLI. Asks for confirmation unless
/// `assume_yes` is set, since this is an external/hard-to-reverse action.
pub fn gh_create(
    target: &Path,
    owner: &str,
    name: &str,
    visibility: &str,
    assume_yes: bool,
) -> Result<()> {
    if which("gh").is_none() {
        crate::output::warn("gh not installed; skipping `gh repo create`");
        return Ok(());
    }
    let slug = format!("{owner}/{name}");
    log::debug!("creating GitHub repo {slug} ({visibility})");
    if !assume_yes {
        let proceed = Confirm::new()
            .with_prompt(format!("Create GitHub repo {slug} ({visibility})?"))
            .default(true)
            .interact()
            .unwrap_or(false);
        if !proceed {
            return Ok(());
        }
    }
    let visibility_flag = format!("--{visibility}");
    let args = vec![
        "repo",
        "create",
        &slug,
        &visibility_flag,
        "--source",
        ".",
        "--push",
    ];
    run(target, "gh", &args)?;
    Ok(())
}

fn run(cwd: &Path, prog: &str, args: &[&str]) -> Result<()> {
    log::debug!("exec: {prog} {}", args.join(" "));
    let status = Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("spawn {prog} {args:?}"))?;
    if !status.success() {
        bail!("`{prog} {}` failed with {status}", args.join(" "));
    }
    Ok(())
}

/// Clone the public oss-spec repository to a local directory so an agent can
/// browse the spec, templates, and reference implementation. Returns the
/// resolved destination path. If `into` is None, a unique subdirectory under
/// the system temp dir is used.
pub fn fetch_oss_spec(url: &str, into: Option<&Path>, shallow: bool) -> Result<PathBuf> {
    let dest: PathBuf = match into {
        Some(p) => p.to_path_buf(),
        None => {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            std::env::temp_dir().join(format!("oss-spec-{nanos}"))
        }
    };

    if dest.exists() {
        bail!(
            "destination {} already exists; pass --into to a fresh path",
            dest.display()
        );
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut args: Vec<String> = vec!["clone".into()];
    if shallow {
        args.extend(["--depth".into(), "1".into()]);
    }
    args.push(url.into());
    args.push(dest.to_string_lossy().into_owned());

    let status = Command::new("git")
        .args(&args)
        .status()
        .with_context(|| format!("spawn git {args:?}"))?;
    if !status.success() {
        bail!("`git clone` failed with {status}");
    }
    Ok(dest)
}

fn which(prog: &str) -> Option<String> {
    let out = Command::new("sh")
        .args(["-c", &format!("command -v {prog}")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
