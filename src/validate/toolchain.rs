//! §10.3 / §10.5 toolchain checks — CI workflow pins and local pin-file parity.
//!
//! Mirrored in `scripts/validate.sh` — keep both in lockstep. In particular,
//! [`MIN_TOOLCHAIN_VERSIONS`] below has a matching `MIN_RUST` / `MIN_PYTHON`
//! / `MIN_NODE` / `MIN_GO` block in the bash script that must move together
//! whenever a minimum is bumped (see [`super`] for the full parity policy).

use super::Violation;
use std::path::Path;

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

/// Scan a workflow file for language toolchain setup blocks and return a
/// violation for every one that uses a floating specifier or pins below
/// the spec minimum. Absent toolchains do **not** produce a violation —
/// a Rust-only project has no `actions/setup-node` block, and that is
/// fine.
pub fn check_toolchain_versions(file: &str, content: &str) -> Vec<Violation> {
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
            if let Some(v) = evaluate("Rust", &spec, file) {
                out.push(v);
            }
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
                    if let Some(v) = evaluate(lang, &spec, file) {
                        out.push(v);
                    }
                }
                break;
            }
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

/// §10.5 Local/CI environment parity.
///
/// For every language whose manifest is present at the repo root, require
/// the corresponding pin file and cross-check its version against the
/// `ci.yml` workflow. A missing pin file, a floating channel, or a
/// mismatch between the pin file and `ci.yml` is a violation.
///
/// `ci_yml` is the contents of `.github/workflows/ci.yml` if the file
/// exists, used only for the cross-check step. Missing `ci.yml` is
/// already reported elsewhere as a §10 violation, so we just skip the
/// cross-check when it is absent.
pub fn check_local_toolchain_pin(path: &Path, ci_yml: Option<&str>) -> Vec<Violation> {
    let mut out = Vec::new();

    if path.join("Cargo.toml").is_file() {
        check_rust_pin(path, ci_yml, &mut out);
    }
    if path.join("pyproject.toml").is_file() || path.join("setup.py").is_file() {
        check_python_pin(path, ci_yml, &mut out);
    }
    if path.join("package.json").is_file() {
        check_node_pin(path, ci_yml, &mut out);
    }
    if path.join("go.mod").is_file() {
        check_go_pin(path, ci_yml, &mut out);
    }

    out
}

fn check_rust_pin(path: &Path, ci_yml: Option<&str>, out: &mut Vec<Violation>) {
    let pin = path.join("rust-toolchain.toml");
    let content = match std::fs::read_to_string(&pin) {
        Ok(c) => c,
        Err(_) => {
            out.push(Violation {
                spec_section: "§10.5",
                message: "missing rust-toolchain.toml; pin the Rust channel at repo root \
                          (see §10.5)"
                    .into(),
            });
            return;
        }
    };
    let channel = match parse_rust_channel(&content) {
        Some(c) => c,
        None => {
            out.push(Violation {
                spec_section: "§10.5",
                message: "rust-toolchain.toml must declare [toolchain] channel = \"<x.y.z>\" \
                          (see §10.5)"
                    .into(),
            });
            return;
        }
    };
    if is_floating_specifier(&channel) {
        out.push(Violation {
            spec_section: "§10.5",
            message: format!(
                "rust-toolchain.toml channel '{channel}' is a floating specifier; \
                 pin to an exact version (see §10.5)"
            ),
        });
        return;
    }
    if let Some(yml) = ci_yml {
        if let Some(ci_ver) = find_rust_ci_version(yml) {
            if ci_ver != channel {
                out.push(Violation {
                    spec_section: "§10.5",
                    message: format!(
                        "rust-toolchain.toml channel '{channel}' does not match ci.yml \
                         dtolnay/rust-toolchain@{ci_ver} (see §10.5)"
                    ),
                });
            }
        }
    }
}

fn check_python_pin(path: &Path, ci_yml: Option<&str>, out: &mut Vec<Violation>) {
    let pin = path.join(".python-version");
    let content = match std::fs::read_to_string(&pin) {
        Ok(c) => c,
        Err(_) => {
            out.push(Violation {
                spec_section: "§10.5",
                message: "missing .python-version; pin the Python version at repo root \
                          (see §10.5)"
                    .into(),
            });
            return;
        }
    };
    let pinned = content.trim().to_string();
    if pinned.is_empty() {
        out.push(Violation {
            spec_section: "§10.5",
            message: ".python-version is empty; pin a concrete version like '3.12' \
                      (see §10.5)"
                .into(),
        });
        return;
    }
    if is_floating_specifier(&pinned) {
        out.push(Violation {
            spec_section: "§10.5",
            message: format!(
                ".python-version '{pinned}' is a floating specifier; pin to an \
                 exact version (see §10.5)"
            ),
        });
        return;
    }
    if let Some(yml) = ci_yml {
        if let Some(ci_ver) = find_setup_version(yml, "actions/setup-python", "python-version")
            && !versions_same_major_minor(&pinned, &ci_ver)
        {
            out.push(Violation {
                spec_section: "§10.5",
                message: format!(
                    ".python-version '{pinned}' does not match ci.yml python-version \
                     '{ci_ver}' (see §10.5)"
                ),
            });
        }
    }
}

fn check_node_pin(path: &Path, ci_yml: Option<&str>, out: &mut Vec<Violation>) {
    let pin = path.join(".nvmrc");
    let content = match std::fs::read_to_string(&pin) {
        Ok(c) => c,
        Err(_) => {
            out.push(Violation {
                spec_section: "§10.5",
                message: "missing .nvmrc; pin the Node version at repo root (see §10.5)".into(),
            });
            return;
        }
    };
    let pinned = content.trim().trim_start_matches('v').to_string();
    if pinned.is_empty() {
        out.push(Violation {
            spec_section: "§10.5",
            message: ".nvmrc is empty; pin a concrete major version like '24' (see §10.5)".into(),
        });
        return;
    }
    if is_floating_specifier(&pinned) {
        out.push(Violation {
            spec_section: "§10.5",
            message: format!(
                ".nvmrc '{pinned}' is a floating specifier; pin to a concrete \
                 version (see §10.5)"
            ),
        });
        return;
    }
    if let Some(yml) = ci_yml {
        if let Some(ci_ver) = find_setup_version(yml, "actions/setup-node", "node-version") {
            let ci_major = ci_ver.split('.').next().unwrap_or("");
            let pin_major = pinned.split('.').next().unwrap_or("");
            if ci_major != pin_major {
                out.push(Violation {
                    spec_section: "§10.5",
                    message: format!(
                        ".nvmrc '{pinned}' does not match ci.yml node-version '{ci_ver}' \
                         (see §10.5)"
                    ),
                });
            }
        }
    }
}

fn check_go_pin(path: &Path, ci_yml: Option<&str>, out: &mut Vec<Violation>) {
    let go_mod = path.join("go.mod");
    let content = match std::fs::read_to_string(&go_mod) {
        Ok(c) => c,
        Err(_) => return, // absence is handled by the outer check.
    };
    let toolchain = parse_go_toolchain(&content);
    let Some(toolchain) = toolchain else {
        out.push(Violation {
            spec_section: "§10.5",
            message: "go.mod must contain a 'toolchain goX.Y.Z' directive pinning the \
                      Go toolchain (see §10.5)"
                .into(),
        });
        return;
    };
    if is_floating_specifier(&toolchain) {
        out.push(Violation {
            spec_section: "§10.5",
            message: format!(
                "go.mod toolchain '{toolchain}' is a floating specifier; pin to an \
                 exact version (see §10.5)"
            ),
        });
        return;
    }
    if let Some(yml) = ci_yml {
        if let Some(ci_ver) = find_setup_version(yml, "actions/setup-go", "go-version")
            && !versions_same_major_minor(&toolchain, &ci_ver)
        {
            out.push(Violation {
                spec_section: "§10.5",
                message: format!(
                    "go.mod toolchain '{toolchain}' does not match ci.yml go-version \
                     '{ci_ver}' (see §10.5)"
                ),
            });
        }
    }
}

/// Extract `channel = "<value>"` from a `[toolchain]` section in a
/// `rust-toolchain.toml` file. Tolerant of surrounding whitespace and
/// comments but does not pull in a full TOML parser.
pub fn parse_rust_channel(content: &str) -> Option<String> {
    let mut in_toolchain = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            in_toolchain = trimmed == "[toolchain]";
            continue;
        }
        if !in_toolchain {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("channel") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let raw = rest.trim();
                let value = raw
                    .split('#')
                    .next()
                    .unwrap_or(raw)
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Extract the `toolchain goX.Y[.Z]` directive from `go.mod`. Returns
/// just the version (`1.22.6`), without the `go` prefix.
pub fn parse_go_toolchain(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("toolchain") {
            let rest = rest.trim_start();
            let tok = rest.split_whitespace().next()?;
            let ver = tok.strip_prefix("go").unwrap_or(tok);
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    None
}

/// `true` if two version strings share the same major and minor
/// segments. Used for Node and Python, where a patch-level mismatch
/// between pin file and CI is fine but a minor mismatch is not.
pub fn versions_same_major_minor(a: &str, b: &str) -> bool {
    let parts = |s: &str| {
        let mut it = s.split('.');
        let major = it.next().unwrap_or("").to_string();
        let minor = it.next().unwrap_or("").to_string();
        (major, minor)
    };
    let (am, an) = parts(a);
    let (bm, bn) = parts(b);
    am == bm && an == bn
}

/// Locate the version used by a `dtolnay/rust-toolchain@<spec>` step
/// in a CI workflow. Returns only the raw spec (for example `1.88.0`
/// or `stable`).
pub fn find_rust_ci_version(ci_yml: &str) -> Option<String> {
    for line in ci_yml.lines() {
        if let Some(idx) = line.find("dtolnay/rust-toolchain@") {
            let rest = &line[idx + "dtolnay/rust-toolchain@".len()..];
            let spec: String = rest
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '#')
                .collect();
            if !spec.is_empty() {
                return Some(spec);
            }
        }
    }
    None
}

/// Locate a `<lang>-version: "<spec>"` value that follows a
/// `uses: actions/setup-<lang>` step. Mirrors the lookup in
/// [`check_toolchain_versions`].
pub fn find_setup_version(ci_yml: &str, setup_action: &str, key: &str) -> Option<String> {
    let lines: Vec<&str> = ci_yml.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().contains(setup_action) {
            if let Some(spec) = find_value_after(&lines, i, key, 6) {
                return Some(spec);
            }
        }
    }
    None
}
