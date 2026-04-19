//! Content checks — rules that look inside files.
//!
//! Includes §19.4 (central output module), §20 (no inline tests), and
//! §20.5 (source files ≤ 1000 lines with an opt-in exception marker).

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
