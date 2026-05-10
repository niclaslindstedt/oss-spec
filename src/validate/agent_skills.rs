//! §21 Agent skills — maintenance playbooks for drift-prone artifacts.
//!
//! Checks that the canonical `.agent/skills/` tree exists, that tool-
//! specific locations (e.g. `.claude/skills`) are symlinks into it, and
//! that every published drift-prone artifact has a matching `update-*`
//! skill alongside the always-required `maintenance` umbrella skill.
//!
//! Mirrored in `scripts/validate.sh` — keep both in lockstep when adding
//! or changing a rule (see [`super`] for the full parity policy).

use super::{Report, Violation};
use std::path::Path;

/// §21 Agent skills. Every repo must ship the canonical `.agent/skills/`
/// tree, the `.claude/skills` symlink, and at least one maintenance skill
/// per drift-prone artifact it publishes.
pub(super) fn check(path: &Path, report: &mut Report) {
    let skills_root = path.join(".agent/skills");

    // 21.2: canonical tree must exist.
    if !skills_root.is_dir() {
        report.violations.push(Violation {
            spec_section: "§21.2",
            message: "missing directory .agent/skills (see §21 Agent skills)".into(),
        });
        return;
    }

    // 21.2: `.claude/skills` must be a symlink whose target (after
    // normalizing path separators) ends with `.agent/skills`. We deliberately
    // avoid `canonicalize` here: on Windows it returns verbatim `\\?\` UNC
    // paths that may not compare equal even for the same location, and the
    // directory-vs-file symlink distinction can make following the link
    // brittle. Checking the raw link target is sufficient to verify intent.
    let claude_skills = path.join(".claude/skills");
    let link_ok = match std::fs::symlink_metadata(&claude_skills) {
        Ok(meta) if meta.file_type().is_symlink() => std::fs::read_link(&claude_skills)
            .ok()
            .and_then(|t| t.to_str().map(|s| s.replace('\\', "/")))
            .map(|s| s.trim_end_matches('/').ends_with(".agent/skills"))
            .unwrap_or(false),
        _ => false,
    };
    if !link_ok {
        report.violations.push(Violation {
            spec_section: "§21.2",
            message: ".claude/skills must be a symlink to ../.agent/skills".into(),
        });
    }

    // 21.3/21.4: every subdirectory under `.agent/skills/` must be a valid
    // skill (SKILL.md with YAML front matter + `.last-updated` file).
    let entries = match std::fs::read_dir(&skills_root) {
        Ok(it) => it,
        Err(_) => return,
    };
    let mut present_skills: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = match p.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        present_skills.push(name.clone());
        validate_skill_dir(&p, &name, report);
    }

    // 21.5 / 21.6: required skills per drift-prone artifact, plus the
    // always-required `maintenance` umbrella skill.
    let mut required: Vec<(&'static str, &'static str)> = vec![("maintenance", "always")];
    if path.join("README.md").exists() {
        required.push(("update-readme", "README.md"));
    }
    if path.join("docs").is_dir() {
        required.push(("update-docs", "docs/"));
    }
    if path.join("man").is_dir() {
        required.push(("update-manpages", "man/"));
    }
    if path.join("website").is_dir() {
        required.push(("update-website", "website/"));
    }
    for (skill, artifact) in required {
        if !present_skills.iter().any(|s| s == skill) {
            let sec = if skill == "maintenance" {
                "§21.6"
            } else {
                "§21.5"
            };
            let reason = if artifact == "always" {
                "always required".to_string()
            } else {
                format!("required because {artifact} is present")
            };
            report.violations.push(Violation {
                spec_section: sec,
                message: format!(
                    "missing maintenance skill .agent/skills/{skill}/SKILL.md ({reason})"
                ),
            });
        }
    }
}

/// Validate a single `.agent/skills/<name>/` directory. Pushes a violation
/// for each problem found — missing SKILL.md, missing front matter, missing
/// tracking file, etc.
fn validate_skill_dir(dir: &Path, name: &str, report: &mut Report) {
    // Kebab-case naming (§21.5).
    if !is_kebab_case(name) {
        report.violations.push(Violation {
            spec_section: "§21.5",
            message: format!(
                ".agent/skills/{name}: skill name must be kebab-case \
                 (lowercase letters, digits, hyphens)"
            ),
        });
    }

    let skill_md = dir.join("SKILL.md");
    let last_updated = dir.join(".last-updated");

    if !skill_md.is_file() {
        report.violations.push(Violation {
            spec_section: "§21.3",
            message: format!(".agent/skills/{name}: missing SKILL.md"),
        });
        return;
    }
    if !last_updated.is_file() {
        report.violations.push(Violation {
            spec_section: "§21.4",
            message: format!(
                ".agent/skills/{name}: missing .last-updated tracking file \
                 (see §21.4)"
            ),
        });
    }

    let content = match std::fs::read_to_string(&skill_md) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(front) = extract_front_matter(&content) else {
        report.violations.push(Violation {
            spec_section: "§21.3",
            message: format!(
                ".agent/skills/{name}/SKILL.md: missing YAML front matter \
                 with `name` and `description`"
            ),
        });
        return;
    };
    if !has_yaml_key(front, "name") {
        report.violations.push(Violation {
            spec_section: "§21.3",
            message: format!(".agent/skills/{name}/SKILL.md: front matter missing `name` field"),
        });
    }
    if !has_yaml_key(front, "description") {
        report.violations.push(Violation {
            spec_section: "§21.3",
            message: format!(
                ".agent/skills/{name}/SKILL.md: front matter missing `description` field"
            ),
        });
    }
}

/// Extract the YAML front matter block from a markdown file. Returns the
/// raw body between the opening `---` line and the closing `---` line, or
/// `None` if the file does not start with front matter. Accepts both LF
/// and CRLF line endings so files checked out through git on Windows
/// (where `core.autocrlf` may rewrite LF → CRLF) still validate.
pub fn extract_front_matter(content: &str) -> Option<&str> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))?;
    // Find the closing `---` line. It may be preceded by `\n` or `\r\n`.
    let end = rest.find("\n---")?;
    // Trim a trailing `\r` off the captured body when running on CRLF files
    // so `has_yaml_key` sees clean line tails.
    let body = &rest[..end];
    Some(body.trim_end_matches('\r'))
}

/// Return `true` if the YAML fragment contains a top-level `<key>:` line.
/// This is a deliberately shallow parser — we only need to confirm that
/// the key exists with some value; detailed schema validation is out of
/// scope for `oss-spec validate`.
pub fn has_yaml_key(yaml: &str, key: &str) -> bool {
    for line in yaml.lines() {
        // Ignore indented continuation lines and comments.
        if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start();
            if rest.starts_with(':') {
                return true;
            }
        }
    }
    false
}

/// Return `true` if `name` is a valid kebab-case identifier: one or more
/// segments of `[a-z0-9]+` separated by single hyphens.
pub fn is_kebab_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut prev_hyphen = true; // disallow leading hyphen
    for c in name.chars() {
        if c == '-' {
            if prev_hyphen {
                return false;
            }
            prev_hyphen = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }
    !prev_hyphen // disallow trailing hyphen
}
