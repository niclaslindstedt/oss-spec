//! Embedded templates. The entire `templates/` directory is compiled into the
//! binary at build time so `oss-spec` can be `cargo install`-ed and used
//! standalone with no runtime data files.

use include_dir::{Dir, include_dir};

pub static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// The full `OSS_SPEC.md` text, compiled into the binary so prompts can
/// reference it without reading from disk.
pub static OSS_SPEC: &str = include_str!("../OSS_SPEC.md");

/// Parse the `version:` field from the YAML front matter of `OSS_SPEC.md`.
/// Accepts both LF and CRLF line endings so git checkouts on Windows
/// (where `core.autocrlf` may rewrite LF → CRLF) still parse.
/// Returns `"unknown"` if the front matter is malformed — which would be
/// a packaging bug, caught by the tests in `tests/validate_test.rs`.
pub fn oss_spec_version() -> &'static str {
    let rest = match OSS_SPEC
        .strip_prefix("---\n")
        .or_else(|| OSS_SPEC.strip_prefix("---\r\n"))
    {
        Some(r) => r,
        None => return "unknown",
    };
    let Some(end) = rest.find("\n---") else {
        return "unknown";
    };
    for line in rest[..end].lines() {
        if let Some(v) = line.strip_prefix("version:") {
            return v.trim().trim_matches('"').trim_matches('\'');
        }
    }
    "unknown"
}
