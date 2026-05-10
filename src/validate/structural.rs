//! Structural checks — file/directory/symlink presence and shape.
//!
//! Every check here is deterministic and cheap: it reads the filesystem,
//! pushes a [`Violation`] per problem found, and returns. AI-assisted
//! quality review lives elsewhere.
//!
//! Mirrored in `scripts/validate.sh` — keep both in lockstep when adding
//! or changing a rule (see [`super`] for the full parity policy).

use super::{Report, Violation, toolchain};
use anyhow::{Context, Result};
use std::path::Path;

use super::content::is_valid_test_stem;

pub(super) fn check(path: &Path, report: &mut Report) -> Result<()> {
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
    // one versioned <major>_<minor>_<patch>.md file. An empty prompts/ is
    // allowed (project sends no LLM prompts), but a half-built one is not.
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
                                .and_then(crate::prompts::parse_version)
                                .is_some()
                    })
                })
                .unwrap_or(false);
            if !has_versioned {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                report.violations.push(Violation {
                    spec_section: "§13.5",
                    message: format!(
                        "prompts/{name}/ has no versioned <major>_<minor>_<patch>.md file"
                    ),
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

    // §10.3 Pinned toolchain minimum versions. Every CI and release job
    // that sets up a language toolchain must declare an explicit minimum
    // version, not a floating specifier (`stable`, `latest`, `lts/*`).
    let ci_yml_content = std::fs::read_to_string(path.join(".github/workflows/ci.yml")).ok();
    for w in &["ci.yml", "release.yml"] {
        let p = path.join(".github/workflows").join(w);
        if let Ok(content) = std::fs::read_to_string(&p) {
            for v in toolchain::check_toolchain_versions(w, &content) {
                report.violations.push(v);
            }
        }
    }

    // §10.5 Local/CI environment parity. For every detected language,
    // require a repo-root pin file and cross-check it against ci.yml.
    for v in toolchain::check_local_toolchain_pin(path, ci_yml_content.as_deref()) {
        report.violations.push(v);
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

    // §20.2 Test file naming: every file in tests/ must have a stem ending
    // with _test, _tests, Test, or Tests.
    let tests_dir = path.join("tests");
    if tests_dir.is_dir() {
        for entry in std::fs::read_dir(&tests_dir)
            .with_context(|| format!("read {}", tests_dir.display()))?
            .flatten()
        {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                if !is_valid_test_stem(stem) {
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                    report.violations.push(Violation {
                        spec_section: "§20.2",
                        message: format!(
                            "tests/{name}: file stem '{stem}' does not end with \
                             _test, _tests, Test, or Tests"
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}
