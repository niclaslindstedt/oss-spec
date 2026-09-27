//! §11.3.12 — unlisted websites.
//!
//! A project whose website is only a testing surface declares it with an
//! `oss-spec:unlisted-website: <reason>` marker line in `AGENTS.md`. The
//! marker switches off the §11.3 SEO scaffolding check, the `seo.yml` /
//! `lighthouse.yml` workflow requirement, and the §11.4.7 Lighthouse PWA
//! assertion, and switches on the checks in this module: some page must
//! carry a robots `noindex` meta tag, and no `robots.txt` may disallow
//! the whole site (a crawler that cannot fetch a page never sees its
//! `noindex`).
//!
//! Mirrored in `scripts/validate.sh` (`declares_unlisted_website`,
//! `check_unlisted_website`) — keep both in lockstep when changing a rule
//! (see [`super`] for the parity policy).

use super::{Report, Violation};
use anyhow::{Context, Result};
use std::path::Path;

/// The declaration token. Same shape as the §20.5.1
/// `oss-spec:allow-large-file:` marker: the token, then a non-empty reason.
pub const UNLISTED_WEBSITE_MARKER: &str = "oss-spec:unlisted-website:";

/// Spec section every violation from this module is filed under.
const SECTION: &str = "§11.3.12";

/// `true` if `AGENTS.md` at the repository root declares the website
/// unlisted: a line carrying [`UNLISTED_WEBSITE_MARKER`] followed by a
/// non-empty reason. A marker without a reason does not count.
pub fn declares_unlisted_website(root: &Path) -> bool {
    std::fs::read_to_string(root.join("AGENTS.md"))
        .map(|content| has_unlisted_marker(&content))
        .unwrap_or(false)
}

/// `true` if any line of `content` carries the marker with a reason.
pub fn has_unlisted_marker(content: &str) -> bool {
    content.lines().any(|line| {
        line.find(UNLISTED_WEBSITE_MARKER).is_some_and(|idx| {
            line[idx + UNLISTED_WEBSITE_MARKER.len()..]
                .chars()
                .any(|c| !c.is_whitespace())
        })
    })
}

/// `true` if `content` contains a `<meta>` tag naming `robots` with a
/// `noindex` directive, e.g. `<meta name="robots" content="noindex">`.
/// Case-insensitive, and a tag may span several lines.
pub fn has_robots_noindex_meta(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(start) = rest.find("<meta") {
        let tag_and_after = &rest[start..];
        let end = tag_and_after.find('>').unwrap_or(tag_and_after.len());
        let tag = &tag_and_after[..end];
        if tag.contains("robots") && tag.contains("noindex") {
            return true;
        }
        rest = &tag_and_after[end..];
    }
    false
}

/// `true` if a `robots.txt` body forbids crawling the whole site — a
/// `Disallow: /` line (the key is case-insensitive; the path is exactly `/`).
pub fn robots_txt_disallows_all(content: &str) -> bool {
    content.lines().any(|line| {
        let line = line.split('#').next().unwrap_or("").trim();
        match line.split_once(':') {
            Some((key, value)) => {
                key.trim().eq_ignore_ascii_case("disallow") && value.trim() == "/"
            }
            None => false,
        }
    })
}

/// §11.3.12 checks for a project that declared its website unlisted and
/// ships one. Called from the §11.3 check in place of the SEO scaffolding
/// check.
pub(super) fn check(path: &Path, report: &mut Report) -> Result<()> {
    let mut found = Found::default();
    walk(path, path, &mut found)?;
    found.disallowing_robots.sort();

    if !found.noindex {
        report.violations.push(Violation {
            spec_section: SECTION,
            message: "AGENTS.md declares the website unlisted but no page carries \
                      <meta name=\"robots\" content=\"noindex\">; add it to every page"
                .into(),
        });
    }
    for rel in found.disallowing_robots {
        report.violations.push(Violation {
            spec_section: SECTION,
            message: format!(
                "{rel}: `Disallow: /` stops crawlers from reading the noindex; \
                 an unlisted website's robots.txt must allow crawling"
            ),
        });
    }
    Ok(())
}

#[derive(Default)]
struct Found {
    noindex: bool,
    /// Root-relative paths of `robots.txt` files that disallow everything.
    disallowing_robots: Vec<String>,
}

fn walk(dir: &Path, root: &Path, found: &mut Found) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
    {
        let p = entry.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if p.is_dir() {
            if super::content::SEO_EXCLUDED_DIRS.contains(&name) {
                continue;
            }
            walk(&p, root, found)?;
            continue;
        }
        if !p.is_file() {
            continue;
        }
        if name == "robots.txt" {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if robots_txt_disallows_all(&content) {
                    let rel = p.strip_prefix(root).unwrap_or(&p);
                    found
                        .disallowing_robots
                        .push(rel.display().to_string().replace('\\', "/"));
                }
            }
            continue;
        }
        if found.noindex || !super::content::is_seo_scannable(name) {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&p) {
            if has_robots_noindex_meta(&content) {
                found.noindex = true;
            }
        }
    }
    Ok(())
}
