//! Versioned prompt loader.
//!
//! All LLM prompts live under `prompts/<name>/<major>_<minor>.md` (see the
//! `Prompts` section of `OSS_SPEC.md`). Each file has two markdown sections:
//!
//! ```text
//! ## System
//! ...system instructions...
//!
//! ## User
//! ...user message, may use {{ jinja }} placeholders...
//! ```
//!
//! `load(name)` returns the highest version available, with placeholders
//! rendered against the supplied context. Prompts are embedded into the
//! binary via `include_dir!` so the CLI ships self-contained.

use anyhow::{Context, Result, anyhow, bail};
use include_dir::{Dir, include_dir};
use minijinja::Environment;
use serde::Serialize;

pub static EMBEDDED_PROMPTS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/prompts");

/// A loaded prompt, ready to feed to an LLM.
#[derive(Debug, Clone)]
pub struct Prompt {
    pub system: String,
    pub user: String,
}

/// Load `prompts/<name>/<latest>.md`, parse its `## System` / `## User`
/// sections, and render the user section against `ctx`.
pub fn load<S: Serialize>(name: &str, ctx: S) -> Result<Prompt> {
    let dir = EMBEDDED_PROMPTS
        .get_dir(name)
        .ok_or_else(|| anyhow!("no prompt directory `prompts/{name}`"))?;

    let mut versions: Vec<((u32, u32), &include_dir::File<'_>)> = Vec::new();
    for entry in dir.entries() {
        if let include_dir::DirEntry::File(f) = entry
            && let Some(stem) = f.path().file_stem().and_then(|s| s.to_str())
            && f.path().extension().and_then(|s| s.to_str()) == Some("md")
            && let Some(v) = parse_version(stem)
        {
            versions.push((v, f));
        }
    }
    if versions.is_empty() {
        bail!("prompts/{name}/ contains no versioned <major>_<minor>.md file");
    }
    versions.sort_by_key(|(v, _)| *v);
    let (_, file) = versions.last().unwrap();
    let body = std::str::from_utf8(file.contents())
        .with_context(|| format!("prompts/{name} is not valid UTF-8"))?;

    let (system, user) = split_sections(body)
        .with_context(|| format!("prompts/{name}: missing ## System or ## User section"))?;

    let env = Environment::new();
    let user_rendered = env
        .render_str(&user, &ctx)
        .with_context(|| format!("prompts/{name}: failed to render user section"))?;

    Ok(Prompt {
        system: system.trim().to_string(),
        user: user_rendered.trim().to_string(),
    })
}

/// Parse `1_0`, `2_13`, etc. into `(major, minor)`. Returns `None` for
/// anything else (like `README.md` sitting next to the versioned files).
fn parse_version(stem: &str) -> Option<(u32, u32)> {
    let (maj, min) = stem.split_once('_')?;
    Some((maj.parse().ok()?, min.parse().ok()?))
}

/// Split a prompt body into its `## System` and `## User` sections.
/// The leading `# Title` line and any other top-level prose is ignored.
fn split_sections(body: &str) -> Result<(String, String)> {
    enum Section {
        None,
        System,
        User,
    }
    let mut system = String::new();
    let mut user = String::new();
    let mut current = Section::None;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## system") {
            current = Section::System;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("## user") {
            current = Section::User;
            continue;
        }
        let buf = match current {
            Section::None => continue,
            Section::System => &mut system,
            Section::User => &mut user,
        };
        buf.push_str(line);
        buf.push('\n');
    }
    if system.trim().is_empty() || user.trim().is_empty() {
        bail!("missing required sections");
    }
    Ok((system, user))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_interpret_prompt() {
        let p = load(
            "interpret-prompt",
            minijinja::context! { prompt => "make a python cli" },
        )
        .unwrap();
        assert!(p.system.contains("oss-spec"));
        assert!(p.user.contains("make a python cli"));
    }

    #[test]
    fn picks_highest_version() {
        // 1_0 must exist for every shipped prompt; if we ever add 1_1 it
        // should win automatically. This test just guards the parser.
        assert_eq!(parse_version("1_0"), Some((1, 0)));
        assert_eq!(parse_version("2_13"), Some((2, 13)));
        assert_eq!(parse_version("README"), None);
    }
}
