//! Content checks — rules that look inside files.
//!
//! Includes §19.4 (central output module), §20 (no inline tests), and
//! §20.5 (source files ≤ 1000 lines with an opt-in exception marker).
//!
//! Mirrored in `scripts/validate.sh` — keep both in lockstep when adding
//! or changing a rule (see [`super`] for the full parity policy).

use super::{Report, Violation};
use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn check(path: &Path, report: &mut Report) -> Result<()> {
    // §19.4 Central output module: all user-facing output must route
    // through a single module, not raw prints. Only applicable when the
    // repo has a recognisable source tree.
    check_output_module(path, report);

    // §20 Test organization: tests must live in separate files, not inline.
    // Check that no source file contains inline test blocks.
    let src_dir = path.join("src");
    if src_dir.is_dir() {
        check_no_inline_tests(&src_dir, path, report)?;
    }

    // §20.5 Source file size limit: non-test source files must not exceed
    // 1000 physical lines unless they carry an `oss-spec:allow-large-file:`
    // marker with a motivating reason in the first 20 lines.
    check_source_file_size(path, report)?;

    // §11.3 SEO and discoverability: if the project has a website, it must
    // ship the SEO scaffolding (Open Graph, Twitter Card, JSON-LD,
    // sitemap.xml, robots.txt). Skipped if there's no website/.
    check_website_seo(path, report)?;

    Ok(())
}

/// Returns `true` if the stem ends with `_test`, `_tests`, `Test`, or `Tests`.
pub(super) fn is_valid_test_stem(stem: &str) -> bool {
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

/// §19.4 Central output module. Every project with source code must
/// route user-facing output through a single module (e.g. `src/output.rs`
/// in Rust, `lib/output.ts` in Node). The check is skipped for repos
/// without a recognisable source tree — a pure docs or template-only
/// repository has nothing to route.
pub(super) fn check_output_module(path: &Path, report: &mut Report) {
    let has_src = path.join("src").is_dir() || path.join("lib").is_dir();
    if !has_src {
        return;
    }
    // Single-file modules under common source roots.
    let file_candidates: &[&str] = &[
        "src/output.rs",
        "src/output.ts",
        "src/output.js",
        "src/output.mjs",
        "src/output.py",
        "src/output.go",
        "src/output.rb",
        "src/output.java",
        "src/output.kt",
        "src/output.swift",
        "src/output.cs",
        "lib/output.ts",
        "lib/output.js",
        "lib/output.mjs",
        "lib/output.py",
    ];
    if file_candidates.iter().any(|c| path.join(c).is_file()) {
        return;
    }
    // Directory-style modules (e.g. `src/output/mod.rs`,
    // `src/output/__init__.py`, `internal/output/output.go`).
    let dir_candidates: &[&str] = &["src/output", "lib/output", "internal/output"];
    if dir_candidates.iter().any(|d| path.join(d).is_dir()) {
        return;
    }
    report.violations.push(Violation {
        spec_section: "§19.4",
        message: "missing central output module (expected e.g. src/output.rs \
                  with semantic helpers status/warn/info/header/error; see §19.4)"
            .into(),
    });
}

/// §20.5 Source file size limits. Non-test source files must not exceed
/// 1000 physical lines. A file may opt out by carrying an
/// `oss-spec:allow-large-file: <reason>` marker with a non-empty reason
/// in any comment within its first 20 lines.
const MAX_SOURCE_LINES: usize = 1000;
const MARKER_SCAN_LINES: usize = 20;

/// Root-relative directories to scan for source files. Paths are joined
/// to the project root at walk time; missing directories are silently
/// skipped.
const SOURCE_ROOTS: &[&str] = &["src", "lib"];

/// File extensions that count as source code for §20.5. Anything not in
/// this set (Markdown, YAML, TOML, generated artifacts, etc.) is ignored.
fn is_source_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "kt" | "cs" | "swift"
    )
}

/// Directory names that should never be descended into while scanning
/// for source files. Keeps test trees, vendored dependencies, and build
/// artifacts out of the size check.
fn is_excluded_dir(name: &str) -> bool {
    matches!(
        name,
        "tests"
            | "target"
            | "node_modules"
            | ".git"
            | ".agent"
            | ".claude"
            | "dist"
            | "build"
            | "__pycache__"
            | ".venv"
            | "venv"
    )
}

fn check_source_file_size(root: &Path, report: &mut Report) -> Result<()> {
    for src_name in SOURCE_ROOTS {
        let dir = root.join(src_name);
        if dir.is_dir() {
            walk_source_tree(&dir, root, report)?;
        }
    }
    Ok(())
}

fn walk_source_tree(dir: &Path, root: &Path, report: &mut Report) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
    {
        let p = entry.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            if is_excluded_dir(name) {
                continue;
            }
            walk_source_tree(&p, root, report)?;
            continue;
        }
        if !p.is_file() {
            continue;
        }
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !is_source_extension(ext) {
            continue;
        }
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            if is_valid_test_stem(stem) {
                continue;
            }
        }
        let content = match std::fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let line_count = content.lines().count();
        if line_count <= MAX_SOURCE_LINES {
            continue;
        }
        if has_allow_large_file_marker(&content) {
            continue;
        }
        let rel = p.strip_prefix(root).unwrap_or(&p);
        let rel_str = rel.display().to_string().replace('\\', "/");
        report.violations.push(Violation {
            spec_section: "§20.5",
            message: format!(
                "{rel_str}: {line_count} lines exceeds {MAX_SOURCE_LINES}-line limit; \
                 split the file or add an `oss-spec:allow-large-file: <reason>` marker \
                 in the first {MARKER_SCAN_LINES} lines"
            ),
        });
    }
    Ok(())
}

/// §11.3 SEO and discoverability. If the project ships a website, it must
/// ship the SEO scaffolding the spec mandates: Open Graph + Twitter Card
/// meta on the routes a crawler sees, JSON-LD structured data, a
/// sitemap.xml + robots.txt + llms.txt emitted by the build, a
/// structural `check-seo` script wired into a workflow (§11.3.10), and a
/// `lighthouse` workflow with a checked-in lighthouserc config.
///
/// The check is intentionally vague about *where* the website lives —
/// it might be `website/`, `pages/`, `site/`, `web/`, `docs-site/`, or
/// somewhere else entirely. We detect "this project ships a website" via
/// signals that survive across naming conventions: a GitHub Pages deploy
/// workflow, or a checked-in `index.html` source. Once a website is
/// detected, we scan every reasonable text file in the repo for the
/// SEO signals; a signal seen *anywhere* counts, since projects that
/// emit their meta tags from a build script's string literals satisfy
/// the spec just as well as projects that hard-code them in
/// hand-authored HTML.
pub(super) fn check_website_seo(path: &Path, report: &mut Report) -> Result<()> {
    let mut has_website = false;
    let mut signals = SeoSignals::default();
    walk_seo(path, &mut has_website, &mut signals)?;

    if !has_website {
        return Ok(());
    }

    let mut missing: Vec<&str> = Vec::new();
    if !signals.og_image {
        missing.push("Open Graph (og:image)");
    }
    if !signals.twitter_card {
        missing.push("Twitter Card (twitter:card)");
    }
    if !signals.json_ld {
        missing.push("JSON-LD (application/ld+json)");
    }
    if !signals.sitemap {
        missing.push("sitemap.xml");
    }
    if !signals.robots {
        missing.push("robots.txt");
    }
    if !signals.llms_txt {
        missing.push("llms.txt");
    }
    if !signals.seo_check {
        missing.push("check-seo script/workflow");
    }
    if !signals.lighthouse {
        missing.push("lighthouse workflow / lighthouserc");
    }
    if missing.is_empty() {
        return Ok(());
    }

    report.violations.push(Violation {
        spec_section: "§11.3",
        message: format!(
            "project has a website but its SEO scaffolding is incomplete; \
             missing: {}",
            missing.join(", ")
        ),
    });
    Ok(())
}

#[derive(Default)]
struct SeoSignals {
    og_image: bool,
    twitter_card: bool,
    json_ld: bool,
    sitemap: bool,
    robots: bool,
    /// §11.3.6 — `/llms.txt` referenced or emitted somewhere in source.
    llms_txt: bool,
    /// §11.3.10 — a structural SEO check is wired in. Satisfied by either
    /// a checked-in `check-seo.*` / `check_seo.*` script, or a workflow
    /// step that invokes it (`check:seo`, `check-seo`, etc.).
    seo_check: bool,
    /// §11.3.10 — a Lighthouse CI run is wired in. Satisfied by either a
    /// `lighthouserc.{json,js,cjs,yml}` config in the repo, or a workflow
    /// step that runs `lhci`.
    lighthouse: bool,
}

/// Directories the SEO walk never enters. Same shape as the source-size
/// excluded list — build artifacts, vendor caches, and VCS metadata.
const SEO_EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".git",
    ".agent",
    ".claude",
    "__pycache__",
    ".venv",
    "venv",
];

/// `true` if `file_name` is a fingerprint that this project ships a
/// website. The two signals we trust:
///
/// - `index.html` (or its template form) — almost universally present in
///   the source of any handwritten or framework-built site.
/// - A GitHub Pages deploy workflow — the canonical "we publish a
///   website" tell, detected by the `actions/deploy-pages` action being
///   referenced from a `.github/workflows/*.yml` file (handled in the
///   walk callback by inspecting workflow contents).
///
/// We deliberately do *not* enumerate every static-site / SPA framework
/// config (`vite.config.*`, `astro.config.*`, etc.) — those names drift
/// faster than this validator could keep up, and the two signals above
/// already cover the cases that matter.
fn is_website_indicator(file_name: &str) -> bool {
    matches!(file_name, "index.html" | "index.htm" | "index.html.tmpl")
}

/// `true` if a file with this name is worth scanning for SEO signal
/// substrings. We deliberately stay text-only — binaries (images, fonts,
/// archives) can never contain meta tags or JSON-LD, and reading them as
/// UTF-8 just wastes I/O.
fn is_seo_scannable(file_name: &str) -> bool {
    if let Some(ext) = file_name.rsplit('.').next() {
        return matches!(
            ext,
            "html"
                | "htm"
                | "js"
                | "ts"
                | "mjs"
                | "cjs"
                | "jsx"
                | "tsx"
                | "vue"
                | "svelte"
                | "tmpl"
                | "json"
        );
    }
    false
}

/// `true` if `file_name` is the structural-SEO script the spec mandates
/// at `website/scripts/check-seo.{ts,mjs}` (§11.3.10). We accept hyphen
/// or underscore and any common script extension so projects laid out
/// differently still pass.
fn is_check_seo_script(file_name: &str) -> bool {
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name);
    matches!(stem, "check-seo" | "check_seo")
}

/// `true` if `file_name` is a Lighthouse CI config file. Lighthouse CI
/// reads `lighthouserc.{json,js,cjs,yml,yaml}` at the project root (or
/// wherever the workflow points it).
fn is_lighthouserc(file_name: &str) -> bool {
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name);
    stem == "lighthouserc"
}

/// `true` if a file path is a GitHub Actions workflow. Workflows are the
/// secondary website-detection signal — a `.github/workflows/*.{yml,yaml}`
/// that references `actions/deploy-pages` says "this project ships a
/// website" even when the source layout uses an unusual folder name.
fn is_github_workflow(p: &Path) -> bool {
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ext != "yml" && ext != "yaml" {
        return false;
    }
    let mut comps = p.components().rev();
    let _file = comps.next();
    let parent = comps.next().and_then(|c| c.as_os_str().to_str());
    let grandparent = comps.next().and_then(|c| c.as_os_str().to_str());
    parent == Some("workflows") && grandparent == Some(".github")
}

fn walk_seo(dir: &Path, has_website: &mut bool, signals: &mut SeoSignals) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
    {
        let p = entry.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if p.is_dir() {
            if SEO_EXCLUDED_DIRS.contains(&name) {
                continue;
            }
            walk_seo(&p, has_website, signals)?;
            continue;
        }
        if !p.is_file() {
            continue;
        }
        if is_website_indicator(name) {
            *has_website = true;
        }
        // §11.3.10 — checked-in `check-seo.{ts,mjs,js,…}` script counts
        // as the structural-SEO check even without inspecting workflow
        // contents. Many projects co-locate it under website/scripts/.
        if is_check_seo_script(name) {
            signals.seo_check = true;
        }
        // §11.3.10 — a checked-in `lighthouserc.*` config means Lighthouse
        // CI is wired in (the workflow that consumes it is a separate but
        // less load-bearing concern; the config is the source of truth).
        if is_lighthouserc(name) {
            signals.lighthouse = true;
        }
        // Workflow scan: a `.github/workflows/*.{yml,yaml}` can supply the
        // website-publishing signal *and* the SEO-CI signals on its own.
        // `actions/deploy-pages` says "this project publishes a website";
        // `check-seo` / `check:seo` says the structural SEO check runs in
        // CI; `lhci` says Lighthouse CI runs in CI.
        if is_github_workflow(&p) {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if content.contains("actions/deploy-pages") {
                    *has_website = true;
                }
                if content.contains("check-seo") || content.contains("check:seo") {
                    signals.seo_check = true;
                }
                if content.contains("lhci") || content.contains("lighthouserc") {
                    signals.lighthouse = true;
                }
            }
        }
        if !is_seo_scannable(name) {
            continue;
        }
        let content = match std::fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if content.contains("og:image") {
            signals.og_image = true;
        }
        if content.contains("twitter:card") {
            signals.twitter_card = true;
        }
        if content.contains("application/ld+json") {
            signals.json_ld = true;
        }
        if content.contains("sitemap.xml") {
            signals.sitemap = true;
        }
        if content.contains("robots.txt") {
            signals.robots = true;
        }
        if content.contains("llms.txt") {
            signals.llms_txt = true;
        }
        if content.contains("check-seo") || content.contains("check:seo") {
            signals.seo_check = true;
        }
        if content.contains("lhci") || content.contains("lighthouserc") {
            signals.lighthouse = true;
        }
    }
    Ok(())
}

/// Detect `oss-spec:allow-large-file: <reason>` within the first
/// [`MARKER_SCAN_LINES`] of `content`. The reason after the colon must be
/// non-empty (i.e. contain at least one non-whitespace character) — an
/// unmotivated exception is not an exception.
pub(super) fn has_allow_large_file_marker(content: &str) -> bool {
    const MARKER: &str = "oss-spec:allow-large-file:";
    for line in content.lines().take(MARKER_SCAN_LINES) {
        if let Some(idx) = line.find(MARKER) {
            let after = &line[idx + MARKER.len()..];
            if after.trim().chars().any(|c| !c.is_whitespace()) {
                return true;
            }
        }
    }
    false
}
