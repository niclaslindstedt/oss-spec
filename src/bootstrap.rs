//! Bootstrap engine — walks the embedded templates tree and writes a fully
//! materialized OSS_SPEC.md-compliant repository to disk.

use anyhow::{Context, Result, bail};
use include_dir::{Dir, DirEntry, File};
use std::path::{Path, PathBuf};

use crate::embedded::TEMPLATES;
use crate::manifest::{Kind, Language, License, ProjectManifest};
use crate::render::render_str;

const TMPL_SUFFIX: &str = ".tmpl";

/// Materialize `manifest` into `target_dir`. Creates the directory if missing.
pub fn write(manifest: &ProjectManifest, target_dir: &Path) -> Result<()> {
    log::debug!(
        "bootstrapping {} into {}",
        manifest.name,
        target_dir.display()
    );
    crate::output::header(&format!(
        "Bootstrapping {} into {}",
        manifest.name,
        target_dir.display()
    ));
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("create target dir {}", target_dir.display()))?;

    // 1. Common tree (everything in templates/_common).
    crate::output::info("Writing common files...");
    let common = TEMPLATES
        .get_dir("_common")
        .context("templates/_common missing — build broken")?;
    write_dir(common, "_common", manifest, target_dir)?;

    // 2. License: pick one file from _licenses/ and rename to LICENSE.
    crate::output::info(&format!("Writing {} license...", manifest.license.spdx()));
    write_license(manifest, target_dir)?;

    // 3. Language overlay (templates/<lang>/).
    if let Some(lang_dir) = TEMPLATES.get_dir(manifest.language.as_str()) {
        crate::output::info(&format!(
            "Applying {} language overlay...",
            manifest.language
        ));
        write_dir(lang_dir, manifest.language.as_str(), manifest, target_dir)?;
    }

    // 4. CLI overlay (man pages, etc.) when applicable.
    if manifest.ships_cli() {
        if let Some(cli_dir) = TEMPLATES.get_dir("cli") {
            crate::output::info("Applying CLI overlay (man pages, etc.)...");
            write_dir(cli_dir, "cli", manifest, target_dir)?;
        }
    }

    // 5. AGENTS.md symlinks (chicken-and-egg: AGENTS.md must exist by now).
    crate::output::info("Creating AGENTS.md symlinks...");
    create_agents_symlinks(target_dir)?;

    // 6. Agent-skills symlink: `.claude/skills` -> `../.agent/skills` (§21.2).
    crate::output::info("Creating agent-skills symlink...");
    create_skills_symlink(target_dir)?;

    Ok(())
}

fn create_skills_symlink(target: &Path) -> Result<()> {
    let skills_root = target.join(".agent/skills");
    if !skills_root.is_dir() {
        // Nothing to link to — the template set did not ship any skills.
        return Ok(());
    }
    std::fs::create_dir_all(target.join(".claude"))?;
    let link = target.join(".claude/skills");
    if link.is_symlink() || link.exists() {
        // On Windows a *directory* symlink cannot be removed with
        // `remove_file` (fails with ACCESS_DENIED) — it must go through
        // `remove_dir`. Try both; swallow the final error since the next
        // `symlink_dir` call will surface any real problem.
        if std::fs::remove_file(&link).is_err() {
            std::fs::remove_dir(&link).ok();
        }
    }
    symlink_dir(Path::new("../.agent/skills"), &link)
        .with_context(|| format!("symlink {} -> ../.agent/skills", link.display()))?;
    Ok(())
}

fn write_dir(dir: &Dir<'_>, prefix: &str, manifest: &ProjectManifest, target: &Path) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub) => write_dir(sub, prefix, manifest, target)?,
            DirEntry::File(file) => write_file(file, prefix, manifest, target)?,
        }
    }
    Ok(())
}

fn write_file(
    file: &File<'_>,
    prefix: &str,
    manifest: &ProjectManifest,
    target: &Path,
) -> Result<()> {
    let rel = file.path();
    // Strip the leading prefix segment (e.g. "_common/", "rust/", "cli/").
    let stripped = rel
        .strip_prefix(prefix)
        .with_context(|| format!("path {} not under prefix {prefix}", rel.display()))?;

    // Skip licenses dir — handled by write_license.
    if stripped.starts_with("_licenses") {
        return Ok(());
    }

    let (out_rel, is_template) = strip_tmpl_suffix(stripped);
    let out_path = target.join(&out_rel);

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }

    let bytes = file.contents();
    if is_template {
        let source = std::str::from_utf8(bytes)
            .with_context(|| format!("template {} not utf-8", rel.display()))?;
        let rendered = render_str(&out_rel.to_string_lossy(), source, manifest)?;
        std::fs::write(&out_path, rendered)
            .with_context(|| format!("write {}", out_path.display()))?;
    } else {
        std::fs::write(&out_path, bytes)
            .with_context(|| format!("write {}", out_path.display()))?;
    }

    // Preserve executable bit for shell scripts.
    if out_path.extension().and_then(|s| s.to_str()) == Some("sh") {
        set_executable(&out_path)?;
    }

    Ok(())
}

fn strip_tmpl_suffix(rel: &Path) -> (PathBuf, bool) {
    let s = rel.to_string_lossy();
    if let Some(stem) = s.strip_suffix(TMPL_SUFFIX) {
        (PathBuf::from(stem), true)
    } else {
        (rel.to_path_buf(), false)
    }
}

fn write_license(manifest: &ProjectManifest, target: &Path) -> Result<()> {
    let licenses_dir = TEMPLATES
        .get_dir("_common/_licenses")
        .context("templates/_common/_licenses missing")?;
    let filename = manifest.license.template_filename();
    let file = licenses_dir
        .get_file(format!("_common/_licenses/{filename}"))
        .with_context(|| format!("license template {filename} missing"))?;
    let source = std::str::from_utf8(file.contents()).context("license template not utf-8")?;
    let rendered = render_str("LICENSE", source, manifest)?;
    std::fs::write(target.join("LICENSE"), rendered)
        .with_context(|| "write LICENSE".to_string())?;
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(path)?.permissions();
    perm.set_mode(perm.mode() | 0o755);
    std::fs::set_permissions(path, perm)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn create_agents_symlinks(target: &Path) -> Result<()> {
    let agents = target.join("AGENTS.md");
    if !agents.exists() {
        bail!("AGENTS.md was not produced by templates — refusing to create symlinks");
    }

    let links: &[(&str, &str)] = &[
        ("CLAUDE.md", "AGENTS.md"),
        (".cursorrules", "AGENTS.md"),
        (".windsurfrules", "AGENTS.md"),
        ("GEMINI.md", "AGENTS.md"),
        (".aider.conf.md", "AGENTS.md"),
        (".github/copilot-instructions.md", "../AGENTS.md"),
    ];
    for (link, dest) in links {
        let link_path = target.join(link);
        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if link_path.is_symlink() || link_path.exists() {
            std::fs::remove_file(&link_path).ok();
        }
        symlink_file(Path::new(dest), &link_path)
            .with_context(|| format!("symlink {} -> {}", link_path.display(), dest))?;
    }
    Ok(())
}

#[cfg(unix)]
pub fn symlink_file(src: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, link)
}

#[cfg(windows)]
pub fn symlink_file(src: &Path, link: &Path) -> std::io::Result<()> {
    // Windows distinguishes file and directory symlinks. All our targets
    // (AGENTS.md / ../AGENTS.md) resolve to files, so symlink_file is correct.
    // Requires Developer Mode or admin; GitHub-hosted windows-latest runners
    // satisfy this.
    std::os::windows::fs::symlink_file(src, link)
}

#[cfg(unix)]
pub fn symlink_dir(src: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, link)
}

#[cfg(windows)]
pub fn symlink_dir(src: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, link)
}

/// Helper for tests/inspection: list all output paths a manifest would create.
pub fn planned_paths(manifest: &ProjectManifest) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Dir<'_>, prefix: &str, out: &mut Vec<PathBuf>) {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(d) => walk(d, prefix, out),
                DirEntry::File(f) => {
                    if let Ok(stripped) = f.path().strip_prefix(prefix)
                        && !stripped.starts_with("_licenses")
                    {
                        let (rel, _) = strip_tmpl_suffix(stripped);
                        out.push(rel);
                    }
                }
            }
        }
    }
    if let Some(common) = TEMPLATES.get_dir("_common") {
        walk(common, "_common", &mut out);
    }
    out.push(PathBuf::from("LICENSE"));
    if let Some(lang) = TEMPLATES.get_dir(manifest.language.as_str()) {
        walk(lang, manifest.language.as_str(), &mut out);
    }
    if manifest.ships_cli()
        && let Some(cli) = TEMPLATES.get_dir("cli")
    {
        walk(cli, "cli", &mut out);
    }
    out.sort();
    out.dedup();
    let _ = (Language::Rust, Kind::Cli, License::Mit); // suppress unused-variant warnings
    out
}
