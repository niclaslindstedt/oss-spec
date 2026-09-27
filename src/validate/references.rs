//! §24 — the scientific references registry.
//!
//! A project whose purpose rests on scientific claims keeps every source
//! its numbers come from in `docs/references.json` and cites an entry with
//! a `[ref:<id>]` tag in the comment beside the number. Opt-in is detected
//! the way §11.4 detects a PWA: the registry file, or any tag in the §20.5
//! source tree, and from then on the whole shape is required:
//!
//! 1. the registry parses, every entry carries the §24.2 fields, and the
//!    optional fields that have a shape (`language`, `summary`, `topics`)
//!    have it when present;
//! 2. every tag names an entry, every entry is tagged somewhere, and each
//!    entry's `usedBy` lists exactly the files that tag it (§24.3);
//! 3. some non-test source or build file reads `references.json`, which is
//!    how the references reach the user (§24.4).
//!
//! Whether a project that has *not* opted in should have — a health app
//! citing its thresholds in prose — needs judgment, and is left to the
//! agent review checklist.
//!
//! Mirrored in `scripts/validate.sh::check_references` — keep both in
//! lockstep when adding or modifying a rule (see [`super`] for the parity
//! policy).

use super::agent_skills::is_kebab_case;
use super::content::{SOURCE_ROOTS, is_excluded_dir, is_source_extension, is_valid_test_stem};
use super::{Report, Violation};
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Where the registry lives, relative to the project root.
pub const REGISTRY_PATH: &str = "docs/references.json";

/// The §24.2 `evidence` vocabulary.
pub const EVIDENCE_KINDS: &[&str] = &[
    "guideline",
    "consensus",
    "systematic-review",
    "meta-analysis",
    "randomized-trial",
    "cohort",
    "clinical-study",
    "review",
    "method",
    "dataset",
    "health-service",
];

const SECTION: &str = "§24";

pub(super) fn check(root: &Path, report: &mut Report) -> Result<()> {
    let citations = collect_citations(root)?;
    let registry_path = root.join(REGISTRY_PATH);
    let has_registry = registry_path.is_file();
    if !has_registry && citations.is_empty() {
        return Ok(());
    }

    let mut push = |message: String| {
        report.violations.push(Violation {
            spec_section: SECTION,
            message,
        });
    };

    if !has_registry {
        let ids: Vec<&str> = citations.keys().map(String::as_str).collect();
        push(format!(
            "source files cite {} reference id(s) ({}) but {REGISTRY_PATH} does not exist; \
             add an entry for every cited source",
            ids.len(),
            ids.join(", ")
        ));
        return Ok(());
    }

    let raw = std::fs::read_to_string(&registry_path)
        .with_context(|| format!("read {}", registry_path.display()))?;
    let registry: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            push(format!("{REGISTRY_PATH} is not valid JSON: {e}"));
            return Ok(());
        }
    };
    let Some(entries) = registry.get("references").and_then(Value::as_object) else {
        push(format!(
            "{REGISTRY_PATH} must be an object with a `references` object keyed by id"
        ));
        return Ok(());
    };

    for (id, entry) in entries {
        for problem in entry_problems(id, entry) {
            push(format!("{REGISTRY_PATH}: `{id}` {problem}"));
        }
    }

    for (id, files) in &citations {
        if !entries.contains_key(id) {
            push(format!(
                "[ref:{id}] is cited in {} but has no entry in {REGISTRY_PATH}",
                join(files)
            ));
        }
    }

    for (id, entry) in entries {
        let Some(files) = citations.get(id) else {
            push(format!(
                "{REGISTRY_PATH}: `{id}` is not cited by any [ref:{id}] tag in the source tree; \
                 cite it beside the number it supports, or remove the entry"
            ));
            continue;
        };
        let Some(used_by) = entry.get("usedBy").and_then(Value::as_array) else {
            continue; // reported by `entry_problems`
        };
        let listed: BTreeSet<String> = used_by
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        if &listed != files {
            push(format!(
                "{REGISTRY_PATH}: `{id}` usedBy [{}] does not match the files that cite it [{}]",
                join(&listed),
                join(files)
            ));
        }
    }

    if !registry_is_read(root)? {
        push(format!(
            "no non-test source or build file reads references.json; §24.4 requires the \
             references to be shown to users from {REGISTRY_PATH} (an in-app view that \
             imports it, or a page generated from it)"
        ));
    }

    Ok(())
}

/// What is wrong with one registry entry, as sentence tails; empty when
/// the entry carries every §24.2 field in a valid form.
pub fn entry_problems(id: &str, entry: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if !is_kebab_case(id) {
        out.push("id is not kebab-case".to_string());
    }
    let Some(e) = entry.as_object() else {
        out.push("is not an object".to_string());
        return out;
    };

    if !non_empty_str(e, "title") {
        out.push("is missing `title`".to_string());
    }
    match e.get("year") {
        Some(y) if y.is_i64() || y.is_u64() => {}
        _ => out.push("is missing an integer `year`".to_string()),
    }

    let has_authors = e.get("authors").and_then(Value::as_array).is_some_and(|a| {
        !a.is_empty()
            && a.iter()
                .all(|x| x.as_str().is_some_and(|s| !s.trim().is_empty()))
    });
    if !has_authors && !non_empty_str(e, "organization") {
        out.push("needs a non-empty `authors` list or an `organization`".to_string());
    }

    let doi = e.get("doi").and_then(Value::as_str);
    let url = e.get("url").and_then(Value::as_str);
    let isbn = e.get("isbn").and_then(Value::as_str);
    if [doi, url, isbn]
        .iter()
        .all(|v| v.is_none_or(|s| s.trim().is_empty()))
    {
        out.push("needs a `doi`, `url`, or `isbn`".to_string());
    }
    if let Some(d) = doi
        && !is_bare_doi(d)
    {
        out.push(format!(
            "`doi` \"{d}\" is not a bare DOI (10.<registrant>/<suffix>)"
        ));
    }
    if let Some(u) = url
        && !u.starts_with("https://")
    {
        out.push(format!("`url` \"{u}\" is not an https:// URL"));
    }
    if let Some(a) = e.get("accessed")
        && !a.as_str().is_some_and(is_iso_date)
    {
        out.push("`accessed` is not a YYYY-MM-DD date".to_string());
    }

    match e.get("evidence").and_then(Value::as_str) {
        Some(kind) if EVIDENCE_KINDS.contains(&kind) => {}
        Some(kind) => out.push(format!(
            "`evidence` \"{kind}\" is not one of: {}",
            EVIDENCE_KINDS.join(", ")
        )),
        None => out.push("is missing `evidence`".to_string()),
    }

    let quotes_ok = e.get("quotes").and_then(Value::as_array).is_some_and(|q| {
        !q.is_empty()
            && q.iter().all(|x| {
                x.get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|t| !t.trim().is_empty())
            })
    });
    if !quotes_ok {
        out.push(
            "needs a non-empty `quotes` list, each with the verbatim `text` the numbers come from"
                .to_string(),
        );
    }

    if !non_empty_str(e, "supports") {
        out.push("is missing `supports` (what the project uses the source for)".to_string());
    }
    let used_by_ok = e
        .get("usedBy")
        .and_then(Value::as_array)
        .is_some_and(|a| a.iter().all(Value::is_string));
    if !used_by_ok {
        out.push("is missing a `usedBy` list of the files that cite it".to_string());
    }

    // The optional fields §24.2 gives a shape to: checked only when present.
    if let Some(lang) = e.get("language")
        && !lang.as_str().is_some_and(is_language_tag)
    {
        out.push("`language` is not a BCP 47 language tag".to_string());
    }
    if let Some(summary) = e.get("summary") {
        let ok = summary.as_object().is_some_and(|m| {
            !m.is_empty()
                && m.iter().all(|(lang, line)| {
                    is_language_tag(lang) && line.as_str().is_some_and(|s| !s.trim().is_empty())
                })
        });
        if !ok {
            out.push(
                "`summary` must be an object of non-empty strings keyed by BCP 47 language tag"
                    .to_string(),
            );
        }
    }
    if let Some(topics) = e.get("topics") {
        let ok = topics.as_array().is_some_and(|a| {
            !a.is_empty() && a.iter().all(|t| t.as_str().is_some_and(is_kebab_case))
        });
        if !ok {
            out.push("`topics` must be a non-empty list of kebab-case topic names".to_string());
        }
    }
    out
}

/// Every `[ref:<id>]` tag in `text`, in order. An id must be kebab-case, so
/// placeholders like `[ref:<id>]` in prose are not tags.
pub fn citation_tags(text: &str) -> Vec<&str> {
    const OPEN: &str = "[ref:";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        let after = &rest[start + OPEN.len()..];
        let len = after
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(after.len());
        let id = &after[..len];
        if after[len..].starts_with(']') && is_kebab_case(id) {
            out.push(id);
        }
        rest = after;
    }
    out
}

/// Which files in the §20.5 source tree cite which ids, as root-relative
/// paths with forward slashes.
fn collect_citations(root: &Path) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut by_id = BTreeMap::new();
    for name in SOURCE_ROOTS {
        let dir = root.join(name);
        if dir.is_dir() {
            walk(&dir, &mut |p| {
                if !is_scannable(p, false) {
                    return Ok(false);
                }
                let Ok(text) = std::fs::read_to_string(p) else {
                    return Ok(false);
                };
                let rel = relative(root, p);
                for id in citation_tags(&text) {
                    by_id
                        .entry(id.to_string())
                        .or_insert_with(BTreeSet::new)
                        .insert(rel.clone());
                }
                Ok(false)
            })?;
        }
    }
    Ok(by_id)
}

/// Whether any non-test source or build file anywhere in the project
/// mentions `references.json` — the importer or generator §24.4 needs.
fn registry_is_read(root: &Path) -> Result<bool> {
    walk(root, &mut |p| {
        if !is_scannable(p, true) {
            return Ok(false);
        }
        Ok(std::fs::read_to_string(p).is_ok_and(|t| t.contains("references.json")))
    })
}

/// A non-test file with a source extension; with `build`, script
/// extensions a site or docs generator is written in count too.
fn is_scannable(p: &Path, build: bool) -> bool {
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    let wanted = is_source_extension(ext)
        || (build && matches!(ext, "mjs" | "cjs" | "mts" | "cts" | "vue" | "svelte"));
    if !wanted {
        return false;
    }
    // `foo_test.rs` (§20.2), and the `foo.test.ts` / `foo.spec.ts` of the
    // JavaScript runners.
    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let stem = name.split('.').next().unwrap_or(name);
    !is_valid_test_stem(stem) && !name.contains(".test.") && !name.contains(".spec.")
}

/// Depth-first walk that skips hidden entries and the §20.5 excluded
/// directories; stops as soon as `visit` returns `true`.
fn walk(dir: &Path, visit: &mut dyn FnMut(&Path) -> Result<bool>) -> Result<bool> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            if !is_excluded_dir(name) && walk(&p, visit)? {
                return Ok(true);
            }
        } else if p.is_file() && visit(&p)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn relative(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn non_empty_str(e: &Map<String, Value>, key: &str) -> bool {
    e.get(key)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
}

fn join(files: &BTreeSet<String>) -> String {
    files.iter().cloned().collect::<Vec<_>>().join(", ")
}

/// `10.<four or more digits>/<suffix without whitespace>`.
pub fn is_bare_doi(doi: &str) -> bool {
    let Some(rest) = doi.strip_prefix("10.") else {
        return false;
    };
    let Some((registrant, suffix)) = rest.split_once('/') else {
        return false;
    };
    registrant.len() >= 4
        && registrant.chars().all(|c| c.is_ascii_digit() || c == '.')
        && registrant.starts_with(|c: char| c.is_ascii_digit())
        && !suffix.is_empty()
        && !suffix.chars().any(char::is_whitespace)
}

/// A BCP 47 language tag, loosely: a two- or three-letter primary
/// language, then `-`-separated subtags of two to eight letters or digits
/// (`en`, `sv`, `pt-BR`, `zh-Hant-TW`).
pub fn is_language_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let primary = parts.next().unwrap_or("");
    (2..=3).contains(&primary.len())
        && primary.chars().all(|c| c.is_ascii_alphabetic())
        && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}
