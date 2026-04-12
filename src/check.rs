//! `oss-spec check` — validate an existing repo against the §19 checklist.
//!
//! This is intentionally a structural / file-presence check. Deeper semantic
//! linting (e.g. "README has a `## Why` section") can layer on later.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Violation {
    pub spec_section: &'static str,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct Report {
    pub violations: Vec<Violation>,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn print(&self) {
        if self.violations.is_empty() {
            crate::output::status("repo conforms to OSS_SPEC.md");
            return;
        }
        crate::output::error(&format!("{} violations:", self.violations.len()));
        for (i, v) in self.violations.iter().enumerate() {
            crate::output::info(&format!(
                "  {:>2}. [{}] {}",
                i + 1,
                v.spec_section,
                v.message
            ));
        }
    }
}

pub fn run(path: &Path) -> Result<Report> {
    log::debug!("checking conformance at {}", path.display());
    let path = path
        .canonicalize()
        .with_context(|| format!("cannot canonicalize {}", path.display()))?;

    let mut report = Report::default();

    // Required root files (§19).
    let required_files: &[(&str, &str)] = &[
        ("LICENSE", "§2"),
        ("README.md", "§3"),
        ("CONTRIBUTING.md", "§4"),
        ("CODE_OF_CONDUCT.md", "§5"),
        ("SECURITY.md", "§6"),
        ("AGENTS.md", "§7"),
        ("CHANGELOG.md", "§8.4"),
        (".gitignore", "§19"),
        (".editorconfig", "§19"),
        ("Makefile", "§9"),
    ];
    for (f, sec) in required_files {
        if !path.join(f).exists() {
            report.violations.push(Violation {
                spec_section: sec,
                message: format!("missing {f}"),
            });
        }
    }

    // AGENTS.md symlinks (§7.1).
    let symlinks: &[(&str, &str)] = &[
        ("CLAUDE.md", "§7.1"),
        (".cursorrules", "§7.1"),
        (".windsurfrules", "§7.1"),
        ("GEMINI.md", "§7.1"),
        (".github/copilot-instructions.md", "§7.1"),
    ];
    for (link, sec) in symlinks {
        let p = path.join(link);
        if !p.is_symlink() {
            report.violations.push(Violation {
                spec_section: sec,
                message: format!("{link} must be a symlink to AGENTS.md"),
            });
        }
    }

    // Required directories (§10/§11/§13.5/§15).
    let required_dirs: &[(&str, &str)] = &[
        (".github/workflows", "§10.1"),
        (".github/ISSUE_TEMPLATE", "§15"),
        ("docs", "§11.1"),
        ("prompts", "§13.5"),
        ("scripts", "§10.3"),
    ];
    for (d, sec) in required_dirs {
        if !path.join(d).is_dir() {
            report.violations.push(Violation {
                spec_section: sec,
                message: format!("missing directory {d}"),
            });
        }
    }

    // §13.5 prompts/ structure: every subdirectory must contain at least
    // one versioned <major>_<minor>.md file. An empty prompts/ is allowed
    // (project sends no LLM prompts), but a half-built one is not.
    let prompts_root = path.join("prompts");
    if prompts_root.is_dir() {
        for entry in std::fs::read_dir(&prompts_root)
            .with_context(|| format!("read {}", prompts_root.display()))?
            .flatten()
        {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let has_versioned = std::fs::read_dir(&p)
                .map(|it| {
                    it.flatten().any(|e| {
                        let f = e.path();
                        f.extension().and_then(|s| s.to_str()) == Some("md")
                            && f.file_stem()
                                .and_then(|s| s.to_str())
                                .and_then(parse_version)
                                .is_some()
                    })
                })
                .unwrap_or(false);
            if !has_versioned {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                report.violations.push(Violation {
                    spec_section: "§13.5",
                    message: format!("prompts/{name}/ has no versioned <major>_<minor>.md file"),
                });
            }
        }
    }

    // Required CI workflows (§10.1, §10.3, §10.4).
    let required_workflows: &[&str] = &["ci.yml", "version-bump.yml", "release.yml", "pages.yml"];
    for w in required_workflows {
        let p = path.join(".github/workflows").join(w);
        if !p.exists() {
            report.violations.push(Violation {
                spec_section: "§10",
                message: format!("missing .github/workflows/{w}"),
            });
        }
    }

    // PR + issue templates (§15).
    for f in [
        ".github/PULL_REQUEST_TEMPLATE.md",
        ".github/ISSUE_TEMPLATE/bug_report.md",
        ".github/ISSUE_TEMPLATE/feature_request.md",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/dependabot.yml",
    ] {
        if !path.join(f).exists() {
            report.violations.push(Violation {
                spec_section: "§15",
                message: format!("missing {f}"),
            });
        }
    }

    Ok(report)
}

#[allow(dead_code)]
fn list_dir(p: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(p)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

/// Parse a `prompts/<name>/<stem>.md` stem like `1_0` or `2_13` into
/// `(major, minor)`. Returns `None` for anything that isn't a version.
fn parse_version(stem: &str) -> Option<(u32, u32)> {
    let (maj, min) = stem.split_once('_')?;
    Some((maj.parse().ok()?, min.parse().ok()?))
}
