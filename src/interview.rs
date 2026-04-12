//! Interactive interview that turns a freeform prompt or CLI flags into a
//! ProjectManifest. Uses dialoguer for terminal Q&A and (optionally) zag for
//! AI-driven inference.

use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, Select};

use crate::ai;
use crate::cli::Cli;
use crate::manifest::{Kind, Language, License, ProjectManifest};

/// Build a manifest by asking the user (and optionally calling zag) for missing
/// fields. `init_mode` is true when called from `oss-spec init`.
pub async fn run(
    cli: &Cli,
    name: Option<String>,
    description: Option<String>,
    init_mode: bool,
) -> Result<ProjectManifest> {
    let name = name
        .or_else(|| cli.name.clone())
        .or_else(|| {
            if init_mode {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
            } else {
                None
            }
        })
        .map(Ok)
        .unwrap_or_else(|| ask_text("Project name", None))?;

    let description = match description.or(cli.prompt.clone()) {
        Some(d) => d,
        None if cli.yes => format!("TODO: describe {name}"),
        None => ask_text("One-sentence description", None)?,
    };

    let mut m = ProjectManifest::skeleton(&name, &description);

    // Apply explicit flag overrides first.
    apply_flag_overrides(cli, &mut m)?;
    fill_author_defaults(&mut m);

    if cli.yes {
        return Ok(m);
    }

    // Interactive refinements (skipped under --yes).
    m.language = ask_language(m.language)?;
    m.kind = ask_kind(m.kind)?;
    m.license = ask_license(m.license)?;

    if !cli.no_ai
        && Confirm::new()
            .with_prompt("Generate README 'Why?' bullets with AI?")
            .default(true)
            .interact()
            .unwrap_or(false)
    {
        match ai::draft_readme_why(&m.description, &m.name).await {
            Ok(bullets) => m.why_bullets = bullets,
            Err(e) => crate::output::warn(&format!("ai disabled: {e}")),
        }
    }

    Ok(m)
}

/// Default flow: a freeform prompt is interpreted by zag into a manifest, then
/// the user is asked to confirm.
pub async fn from_prompt(cli: &Cli, prompt: &str) -> Result<ProjectManifest> {
    log::debug!("from_prompt: interpreting freeform prompt");
    let mut m = if cli.no_ai {
        ProjectManifest::skeleton("new-project", prompt)
    } else {
        match ai::interpret_prompt(prompt).await {
            Ok(m) => m,
            Err(e) => {
                crate::output::warn(&format!(
                    "ai interpretation failed ({e}); falling back to defaults"
                ));
                ProjectManifest::skeleton("new-project", prompt)
            }
        }
    };

    apply_flag_overrides(cli, &mut m)?;
    fill_author_defaults(&mut m);

    if cli.yes {
        return Ok(m);
    }

    crate::output::info("");
    crate::output::header("Project plan:");
    crate::output::info(&format!("  name        {}", m.name));
    crate::output::info(&format!("  description {}", m.description));
    crate::output::info(&format!("  language    {}", m.language));
    crate::output::info(&format!("  kind        {}", m.kind));
    crate::output::info(&format!("  license     {}", m.license));
    crate::output::info(&format!("  github      {}", m.github_owner));
    crate::output::info("");

    let ok = Confirm::new()
        .with_prompt("Looks good?")
        .default(true)
        .interact()
        .unwrap_or(true);
    if !ok {
        // Fall back to the long-form interview, seeded with what we have.
        m.name = ask_text("Project name", Some(&m.name))?;
        m.language = ask_language(m.language)?;
        m.kind = ask_kind(m.kind)?;
        m.license = ask_license(m.license)?;
    }

    Ok(m)
}

fn apply_flag_overrides(cli: &Cli, m: &mut ProjectManifest) -> Result<()> {
    if let Some(n) = &cli.name {
        m.name = n.clone();
    }
    if let Some(l) = &cli.lang {
        m.language = Language::parse(l).with_context(|| format!("unknown --lang {l}"))?;
    }
    if let Some(k) = &cli.kind {
        m.kind = Kind::parse(k).with_context(|| format!("unknown --kind {k}"))?;
    }
    if let Some(lic) = &cli.license {
        m.license = License::parse(lic).with_context(|| format!("unknown --license {lic}"))?;
    }
    Ok(())
}

fn fill_author_defaults(m: &mut ProjectManifest) {
    if m.author_name == "Your Name"
        && let Some(name) = git_config("user.name")
    {
        m.author_name = name;
    }
    if m.author_email == "you@example.com"
        && let Some(email) = git_config("user.email")
    {
        m.author_email = email;
    }
    if m.github_owner == "your-github"
        && let Some(owner) = gh_user()
    {
        m.github_owner = owner;
    }
}

fn git_config(key: &str) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn gh_user() -> Option<String> {
    let out = std::process::Command::new("gh")
        .args(["api", "user", "-q", ".login"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn ask_text(label: &str, default: Option<&str>) -> Result<String> {
    let mut input = Input::<String>::new().with_prompt(label.to_string());
    if let Some(d) = default {
        input = input.default(d.to_string());
    }
    input.interact_text().context("interactive prompt failed")
}

fn ask_language(default: Language) -> Result<Language> {
    let opts = ["rust", "python", "node", "go", "generic"];
    let default_idx = opts
        .iter()
        .position(|s| *s == default.as_str())
        .unwrap_or(0);
    let idx = Select::new()
        .with_prompt("Language")
        .items(&opts)
        .default(default_idx)
        .interact()?;
    Ok(Language::parse(opts[idx]).unwrap())
}

fn ask_kind(default: Kind) -> Result<Kind> {
    let opts = ["lib", "cli", "service"];
    let default_idx = opts
        .iter()
        .position(|s| *s == default.as_str())
        .unwrap_or(1);
    let idx = Select::new()
        .with_prompt("Project kind")
        .items(&opts)
        .default(default_idx)
        .interact()?;
    Ok(Kind::parse(opts[idx]).unwrap())
}

fn ask_license(default: License) -> Result<License> {
    let opts = ["MIT", "Apache-2.0", "MPL-2.0"];
    let default_idx = opts.iter().position(|s| *s == default.spdx()).unwrap_or(0);
    let idx = Select::new()
        .with_prompt("License")
        .items(&opts)
        .default(default_idx)
        .interact()?;
    Ok(License::parse(opts[idx]).unwrap())
}
