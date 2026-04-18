//! Versioned prompt loader.
//!
//! All LLM prompts live under `prompts/<name>/<major>_<minor>_<patch>.md`
//! (see §13.5 of `OSS_SPEC.md`). Each file starts with YAML front matter
//! describing the prompt, followed by two markdown sections:
//!
//! ```text
//! ---
//! name: fix-conformance
//! description: "…"
//! version: 1.1.0
//! ---
//!
//! # fix-conformance
//!
//! ## System
//! ...system instructions...
//!
//! ## User
//! ...user message, may use {{ jinja }} placeholders...
//! ```
//!
//! `load(name)` returns the highest version available, with the front
//! matter stripped before the System/User sections are extracted and
//! placeholders rendered against the supplied context. Prompts are
//! embedded into the binary via `include_dir!` so the CLI ships
//! self-contained.

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

/// Load `prompts/<name>/<latest>.md`, strip its YAML front matter, parse
/// its `## System` / `## User` sections, and render the user section
/// against `ctx`.
pub fn load<S: Serialize>(name: &str, ctx: S) -> Result<Prompt> {
    let dir = EMBEDDED_PROMPTS
        .get_dir(name)
        .ok_or_else(|| anyhow!("no prompt directory `prompts/{name}`"))?;

    let mut versions: Vec<((u32, u32, u32), &include_dir::File<'_>)> = Vec::new();
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
        bail!("prompts/{name}/ contains no versioned <major>_<minor>_<patch>.md file");
    }
    versions.sort_by_key(|(v, _)| *v);
    let (_, file) = versions.last().unwrap();
    let body = std::str::from_utf8(file.contents())
        .with_context(|| format!("prompts/{name} is not valid UTF-8"))?;

    let body = strip_front_matter(body);

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

/// Parse `1_0_0`, `2_13_7`, etc. into `(major, minor, patch)`. Returns
/// `None` for stems that do not match the `X_Y_Z` semver pattern.
pub fn parse_version(stem: &str) -> Option<(u32, u32, u32)> {
    let mut parts = stem.split('_');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Strip the leading YAML front matter block (delimited by `---` lines)
/// from a prompt body. If the body does not begin with front matter,
/// return it unchanged. Accepts both LF and CRLF line endings.
pub fn strip_front_matter(body: &str) -> &str {
    let rest = match body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))
    {
        Some(r) => r,
        None => return body,
    };
    // Find the closing `---` line.
    let end = match rest.find("\n---\n").or_else(|| rest.find("\n---\r\n")) {
        Some(e) => e,
        None => return body, // malformed — pass through, splitter will fail
    };
    let after = &rest[end..];
    // Skip the closing delimiter line itself.
    after
        .strip_prefix("\n---\n")
        .or_else(|| after.strip_prefix("\n---\r\n"))
        .unwrap_or(after)
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
