//! `oss-spec validate` — validate an existing repo against the §19 checklist.
//!
//! Two layers of checking:
//!
//! 1. **Structural** — file-presence, symlink, and directory checks (deterministic).
//! 2. **AI quality** — one-shot LLM review of file *contents* against OSS_SPEC.md
//!    (enabled by default, skipped with `--no-ai`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Violation {
    pub spec_section: &'static str,
    pub message: String,
}

/// A quality/content finding produced by AI review. Unlike [`Violation`],
/// these represent issues that require human or AI judgment to detect
/// (e.g. placeholder text, missing required sections in README).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiFinding {
    pub file: String,
    pub spec_section: String,
    pub severity: String,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Default)]
pub struct Report {
    pub violations: Vec<Violation>,
    pub ai_findings: Vec<AiFinding>,
}

impl Report {
    /// Returns `true` if there are no structural violations.
    /// AI findings do **not** affect this — they are advisory only.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn print(&self) {
        if self.violations.is_empty() && self.ai_findings.is_empty() {
            crate::output::status("repo conforms to OSS_SPEC.md");
            return;
        }
        if !self.violations.is_empty() {
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
        if !self.ai_findings.is_empty() {
            crate::output::header("AI quality findings:");
            let errors: Vec<_> = self
                .ai_findings
                .iter()
                .filter(|f| f.severity == "error")
                .collect();
            let warnings: Vec<_> = self
                .ai_findings
                .iter()
                .filter(|f| f.severity != "error")
                .collect();
            let mut idx = 0;
            for f in errors.iter().chain(warnings.iter()) {
                idx += 1;
                let sev = if f.severity == "error" { "ERR" } else { "WARN" };
                crate::output::info(&format!(
                    "  {:>2}. [{sev}] [{}] {}: {}",
                    idx, f.spec_section, f.file, f.message
                ));
                crate::output::info(&format!("      Suggestion: {}", f.suggestion));
            }
        }
    }
}

pub fn run(path: &Path) -> Result<Report> {
    log::debug!("checking conformance at {}", path.display());
    crate::output::info(
        "note: bringing a repo fully into spec usually takes a few `oss-spec validate` runs — \
         fixing one violation (e.g. creating a missing file) often uncovers the next layer \
         (e.g. that file not yet being complete). Re-run until the report is clean.",
    );
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

    // §10.3 Pinned toolchain minimum versions. Every CI and release job
    // that sets up a language toolchain must declare an explicit minimum
    // version, not a floating specifier (`stable`, `latest`, `lts/*`).
    for w in &["ci.yml", "release.yml"] {
        let p = path.join(".github/workflows").join(w);
        if let Ok(content) = std::fs::read_to_string(&p) {
            for v in check_toolchain_versions(w, &content) {
                report.violations.push(v);
            }
        }
    }

    // §10.5 Local/CI environment parity. For each language the CI
    // workflows pin, a matching root-level pin file must exist and its
    // version must agree with CI.
    for v in check_local_ci_parity(&path) {
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

    // §20 Test organization: tests must live in separate files, not inline.
    // Check that no source file contains inline test blocks.
    let src_dir = path.join("src");
    if src_dir.is_dir() {
        check_no_inline_tests(&src_dir, &path, &mut report)?;
    }

    // §21 Agent skills: every project must ship a canonical skills tree
    // at `.agent/skills/`, with tool-specific locations (e.g. `.claude/skills`)
    // symlinked to it, and at least one required maintenance skill per
    // drift-prone artifact that the project publishes.
    check_agent_skills(&path, &mut report);

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

    Ok(report)
}

/// Returns `true` if the stem ends with `_test`, `_tests`, `Test`, or `Tests`.
fn is_valid_test_stem(stem: &str) -> bool {
    stem.ends_with("_test")
        || stem.ends_with("_tests")
        || stem.ends_with("Test")
        || stem.ends_with("Tests")
}

/// Recursively scan a directory for source files containing inline test blocks
/// (e.g. `#[cfg(test)]` in Rust). Each match is a §20 violation.
///
/// Only lines where the marker appears as actual code are flagged — occurrences
/// inside string literals, comments, or doc comments are ignored.
fn check_no_inline_tests(dir: &Path, root: &Path, report: &mut Report) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
    {
        let p = entry.path();
        if p.is_dir() {
            check_no_inline_tests(&p, root, report)?;
            continue;
        }
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "rs" {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&p) {
            if has_inline_test_attribute(&content) {
                let rel = p.strip_prefix(root).unwrap_or(&p);
                let rel_str = rel.display().to_string().replace('\\', "/");
                report.violations.push(Violation {
                    spec_section: "§20",
                    message: format!(
                        "{rel_str}: contains inline test block; \
                         move tests to a separate file in tests/"
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Returns `true` if the Rust source contains an **inline** test module —
/// i.e. `#[cfg(test)]` followed by `mod <name> { ... }` (with braces).
///
/// `#[cfg(test)]` that merely imports a separate file (`mod tests;` with a
/// semicolon) or gates a `use` statement is allowed and not flagged.
fn has_inline_test_attribute(source: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        // Skip comments and doc comments.
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            i += 1;
            continue;
        }
        if trimmed.starts_with("#[cfg(test)]") {
            // Look at the rest of this line and subsequent non-blank,
            // non-attribute lines for `mod <name> {`.
            let after_attr = trimmed.strip_prefix("#[cfg(test)]").unwrap().trim();
            if is_inline_mod(after_attr) {
                return true;
            }
            // Check following lines (the mod declaration may be on the next line).
            for next_line in &lines[i + 1..] {
                let next = next_line.trim();
                if next.is_empty() || next.starts_with("#[") {
                    continue;
                }
                return is_inline_mod(next);
            }
        }
        i += 1;
    }
    false
}

/// Returns `true` if the line declares a module with a body (`mod foo {`),
/// as opposed to an external file reference (`mod foo;`).
fn is_inline_mod(line: &str) -> bool {
    if let Some(rest) = line.strip_prefix("mod ") {
        // `mod tests {` or `mod tests{` → inline; `mod tests;` → external file
        let rest = rest.trim();
        return rest.contains('{');
    }
    false
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

/// Spec-defined minimum toolchain versions (§10.3). Mirrors the table in
/// `OSS_SPEC.md`. Each entry is `(language, minimum)`; the minimum is the
/// same string that `oss-spec validate` will compare declared versions
/// against.
const MIN_TOOLCHAIN_VERSIONS: &[(&str, &str)] = &[
    ("Rust", "1.88.0"),
    ("Python", "3.12"),
    ("Node", "24"),
    ("Go", "1.22"),
];

fn min_version(lang: &str) -> &'static str {
    MIN_TOOLCHAIN_VERSIONS
        .iter()
        .find(|(l, _)| *l == lang)
        .map(|(_, v)| *v)
        .expect("unknown language")
}

/// Compare two dotted version strings segment-by-segment. Shorter versions
/// are zero-padded, so `"1.82"` == `"1.82.0"`. Returns `None` if either
/// side contains a non-numeric segment.
pub fn version_ge(lhs: &str, rhs: &str) -> Option<bool> {
    let parse = |s: &str| -> Option<Vec<u32>> { s.split('.').map(|p| p.parse().ok()).collect() };
    let mut a = parse(lhs)?;
    let mut b = parse(rhs)?;
    while a.len() < b.len() {
        a.push(0);
    }
    while b.len() < a.len() {
        b.push(0);
    }
    Some(a >= b)
}

fn is_floating_specifier(spec: &str) -> bool {
    let s = spec.trim().trim_matches('"').trim_matches('\'');
    matches!(s, "stable" | "latest" | "lts" | "lts/*" | "*")
}

/// Look for `key: "<value>"` (or single-quoted / unquoted) on one of the
/// next `window` lines after `anchor_idx`. Returns the raw value.
fn find_value_after(lines: &[&str], anchor_idx: usize, key: &str, window: usize) -> Option<String> {
    let end = (anchor_idx + 1 + window).min(lines.len());
    for line in &lines[anchor_idx + 1..end] {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix(':') {
                let raw = rest.trim();
                let value = raw.trim_matches('"').trim_matches('\'').to_string();
                return Some(value);
            }
        }
    }
    None
}

/// Scan a workflow YAML for language toolchain setup blocks and return
/// `(language, pinned-version)` for every one found. No judgment about
/// floors or floating specifiers — that lives in [`check_toolchain_versions`].
/// Used by both §10.3 (version minimums) and §10.5 (local/CI parity).
pub fn detect_ci_toolchains(content: &str) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        // Rust: `dtolnay/rust-toolchain@<spec>` on a `uses:` line.
        if let Some(idx) = line.find("dtolnay/rust-toolchain@") {
            let rest = &line[idx + "dtolnay/rust-toolchain@".len()..];
            let spec: String = rest
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '#')
                .collect();
            out.push(("Rust", spec));
            continue;
        }

        // Python / Node / Go: `uses: actions/setup-<lang>` followed within
        // a few lines by `<lang>-version: "<spec>"`.
        let trimmed = line.trim_start();
        let setup = [
            ("actions/setup-python", "Python", "python-version"),
            ("actions/setup-node", "Node", "node-version"),
            ("actions/setup-go", "Go", "go-version"),
        ];
        for (needle, lang, key) in setup {
            if trimmed.contains(needle) {
                if let Some(spec) = find_value_after(&lines, i, key, 6) {
                    out.push((lang, spec));
                }
                break;
            }
        }
    }

    out
}

/// Scan a workflow file for language toolchain setup blocks and return a
/// violation for every one that uses a floating specifier or pins below
/// the spec minimum. Absent toolchains do **not** produce a violation —
/// a Rust-only project has no `actions/setup-node` block, and that is
/// fine.
pub fn check_toolchain_versions(file: &str, content: &str) -> Vec<Violation> {
    let mut out = Vec::new();
    for (lang, spec) in detect_ci_toolchains(content) {
        if let Some(v) = evaluate(lang, &spec, file) {
            out.push(v);
        }
    }
    out
}

fn evaluate(lang: &str, spec: &str, file: &str) -> Option<Violation> {
    let min = min_version(lang);
    if is_floating_specifier(spec) {
        return Some(Violation {
            spec_section: "§10.3",
            message: format!(
                ".github/workflows/{file}: {lang} toolchain uses floating specifier '{spec}'; \
                 pin to >= {min}"
            ),
        });
    }
    match version_ge(spec, min) {
        Some(true) => None,
        Some(false) => Some(Violation {
            spec_section: "§10.3",
            message: format!(
                ".github/workflows/{file}: {lang} toolchain pinned to {spec}; minimum is {min}"
            ),
        }),
        None => Some(Violation {
            spec_section: "§10.3",
            message: format!(
                ".github/workflows/{file}: {lang} toolchain has unparseable version '{spec}'; \
                 pin to >= {min}"
            ),
        }),
    }
}

/// §10.5 Local/CI environment parity. For each language the CI workflows
/// pin, the repo must commit a matching root-level pin file whose version
/// agrees with CI. Returns one violation per drift.
///
/// CI is the source of truth; local files follow. A repo whose CI pins
/// no language toolchain (pure-docs / `Generic`) is a no-op.
pub fn check_local_ci_parity(path: &Path) -> Vec<Violation> {
    let mut out = Vec::new();

    // Collect CI-pinned (lang, version) from both workflows, deduped.
    let mut ci: Vec<(&'static str, String)> = Vec::new();
    for w in &["ci.yml", "release.yml"] {
        let p = path.join(".github/workflows").join(w);
        if let Ok(content) = std::fs::read_to_string(&p) {
            for (lang, spec) in detect_ci_toolchains(&content) {
                if !ci.iter().any(|(l, s)| *l == lang && s == &spec) {
                    ci.push((lang, spec));
                }
            }
        }
    }
    if ci.is_empty() {
        return out;
    }

    // Keep the first CI pin per language; if CI workflows disagree, that
    // is a §10.3 concern (handled by `check_toolchain_versions`), not §10.5.
    let mut seen_langs: Vec<&'static str> = Vec::new();
    for (lang, spec) in ci {
        if seen_langs.contains(&lang) {
            continue;
        }
        seen_langs.push(lang);
        check_parity_for_lang(path, lang, &spec, &mut out);
    }

    out
}

fn check_parity_for_lang(path: &Path, lang: &str, ci_version: &str, out: &mut Vec<Violation>) {
    let pins = read_local_pins(path, lang);

    if pins.is_empty() {
        out.push(Violation {
            spec_section: "§10.5",
            message: format!(
                "missing local toolchain pin for {lang} (CI pins {ci_version}); \
                 commit {} at the repo root — see §10.5",
                expected_pin_filename(lang)
            ),
        });
        return;
    }

    // If multiple pin files exist for the same language, they must agree
    // with each other before we compare against CI.
    if pins.len() > 1 {
        let first = &pins[0].1;
        for (file, ver) in pins.iter().skip(1) {
            if !toolchain_eq(lang, first, ver) {
                out.push(Violation {
                    spec_section: "§10.5",
                    message: format!(
                        "local toolchain pins disagree: {}={} vs {}={}",
                        pins[0].0, first, file, ver
                    ),
                });
            }
        }
    }

    for (file, ver) in &pins {
        if !toolchain_eq(lang, ci_version, ver) {
            out.push(Violation {
                spec_section: "§10.5",
                message: format!(
                    "{file} pins {lang} {ver} but CI pins {ci_version}; \
                     local and CI must match (§10.5)"
                ),
            });
        }
    }
}

/// Human-readable list of accepted pin filenames for a language, used in
/// violation messages.
fn expected_pin_filename(lang: &str) -> &'static str {
    match lang {
        "Rust" => "rust-toolchain.toml (or rust-toolchain)",
        "Python" => ".python-version",
        "Node" => ".nvmrc (or .node-version)",
        "Go" => ".go-version (or a `go` directive in go.mod)",
        _ => "a root-level toolchain pin file",
    }
}

/// Read every local pin file that applies to `lang`. Returns
/// `(filename, parsed-version)` pairs; empty if none are present.
fn read_local_pins(path: &Path, lang: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    match lang {
        "Rust" => {
            if let Some(v) = read_rust_toolchain(&path.join("rust-toolchain.toml")) {
                out.push(("rust-toolchain.toml".to_string(), v));
            }
            if let Some(v) = read_rust_toolchain(&path.join("rust-toolchain")) {
                out.push(("rust-toolchain".to_string(), v));
            }
        }
        "Python" => {
            if let Some(v) = read_first_value(&path.join(".python-version")) {
                out.push((".python-version".to_string(), v));
            }
        }
        "Node" => {
            if let Some(v) = read_first_value(&path.join(".nvmrc")) {
                out.push((".nvmrc".to_string(), v));
            }
            if let Some(v) = read_first_value(&path.join(".node-version")) {
                out.push((".node-version".to_string(), v));
            }
        }
        "Go" => {
            if let Some(v) = read_first_value(&path.join(".go-version")) {
                out.push((".go-version".to_string(), v));
            } else if let Some(v) = read_go_mod_directive(&path.join("go.mod")) {
                out.push(("go.mod".to_string(), v));
            }
        }
        _ => {}
    }
    out
}

/// Parse a `rust-toolchain.toml` (or legacy `rust-toolchain`) and return the
/// channel. Accepts:
///   - `[toolchain]` table with `channel = "..."`
///   - bare `channel = "..."` at the top of the file
///   - legacy single-line `rust-toolchain` file containing just the channel
fn read_rust_toolchain(p: &Path) -> Option<String> {
    let content = std::fs::read_to_string(p).ok()?;
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with('[') {
            continue;
        }
        if let Some(rest) = t.strip_prefix("channel") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                return Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
        // Legacy bare file: first non-comment, non-empty, non-TOML line is
        // the channel itself.
        return Some(t.trim_matches('"').trim_matches('\'').to_string());
    }
    None
}

/// First non-blank, non-comment line, with surrounding whitespace and
/// quotes trimmed. Suitable for `.python-version`, `.nvmrc`,
/// `.node-version`, `.go-version`.
fn read_first_value(p: &Path) -> Option<String> {
    let content = std::fs::read_to_string(p).ok()?;
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        return Some(t.trim_matches('"').trim_matches('\'').to_string());
    }
    None
}

/// Extract the `go <version>` directive from a `go.mod` file.
fn read_go_mod_directive(p: &Path) -> Option<String> {
    let content = std::fs::read_to_string(p).ok()?;
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("go ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Compare a CI toolchain spec against a local pin spec. The rule is
/// permissive about harmless shapes (trailing `.0`, local Python patch
/// level, local Rust channel written as a quoted full version) and
/// strict about everything else. Non-numeric channels must match
/// byte-for-byte.
pub fn toolchain_eq(lang: &str, ci: &str, local: &str) -> bool {
    let ci = ci.trim_matches('"').trim_matches('\'').trim();
    let local = local.trim_matches('"').trim_matches('\'').trim();

    let ci_numeric = is_numeric_version(ci);
    let local_numeric = is_numeric_version(local);
    if !ci_numeric || !local_numeric {
        return ci == local;
    }

    // Python: local may be more specific than CI (3.12.4 vs 3.12) but the
    // major.minor prefix must match.
    if lang == "Python" {
        let ci_mm = major_minor(ci);
        let local_mm = major_minor(local);
        return ci_mm == local_mm;
    }

    // Everything else: strip a trailing `.0` on either side so `1.22` and
    // `1.22.0` compare equal, then require string equality.
    strip_trailing_zero(ci) == strip_trailing_zero(local)
}

fn is_numeric_version(s: &str) -> bool {
    !s.is_empty()
        && s.split('.')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn major_minor(s: &str) -> String {
    let parts: Vec<&str> = s.split('.').collect();
    match parts.as_slice() {
        [a, b, ..] => format!("{a}.{b}"),
        [a] => (*a).to_string(),
        _ => String::new(),
    }
}

fn strip_trailing_zero(s: &str) -> String {
    let mut parts: Vec<&str> = s.split('.').collect();
    while parts.len() > 1 && *parts.last().unwrap() == "0" {
        parts.pop();
    }
    parts.join(".")
}

/// §21 Agent skills. Every repo must ship the canonical `.agent/skills/`
/// tree, the `.claude/skills` symlink, and at least one maintenance skill
/// per drift-prone artifact it publishes.
pub fn check_agent_skills(path: &Path, report: &mut Report) {
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

/// Maximum number of lines to include per file in the AI verification prompt.
const MAX_LINES_PER_FILE: usize = 200;

/// All spec-relevant files whose content should be sent to the AI for quality
/// review. Paths are relative to the repo root.
const VERIFIABLE_FILES: &[&str] = &[
    "LICENSE",
    "README.md",
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md",
    "SECURITY.md",
    "AGENTS.md",
    "CHANGELOG.md",
    ".editorconfig",
    "Makefile",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    ".github/workflows/version-bump.yml",
    ".github/workflows/pages.yml",
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/ISSUE_TEMPLATE/feature_request.md",
    ".github/dependabot.yml",
];

/// Read the content of every spec-relevant file that exists on disk.
/// Each file is truncated to [`MAX_LINES_PER_FILE`] lines to keep the
/// prompt size manageable.
pub fn gather_file_contents(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for &relpath in VERIFIABLE_FILES {
        let full = root.join(relpath);
        if let Ok(raw) = std::fs::read_to_string(&full) {
            let truncated: String = raw
                .lines()
                .take(MAX_LINES_PER_FILE)
                .collect::<Vec<_>>()
                .join("\n");
            out.push((relpath.to_string(), truncated));
        }
    }
    out
}
