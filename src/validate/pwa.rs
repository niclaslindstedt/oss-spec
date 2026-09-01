//! §11.4 — Progressive Web App completeness checks.
//!
//! Detects PWA opt-in from repository signals (Web App Manifest file,
//! `<link rel="manifest">` in HTML, vite-plugin-pwa / next-pwa /
//! workbox config, a `navigator.serviceWorker.register(` call) and,
//! once any signal appears, asserts that the rest of the §11.4 shape is
//! present: required manifest fields, a maskable 512×512 icon, iOS
//! install meta tags, an offline fallback, a user-visible update
//! affordance, an icon-generation source, and a Lighthouse PWA
//! assertion.
//!
//! The check is intentionally permissive about *where* files live —
//! webapps put icons under `public/`, `static/`, `assets/`, or
//! framework-specific paths. We follow the same heuristic as
//! [`content::check_website_seo`](super::content::check_website_seo):
//! scan everything text-ish under the project root, ignore the usual
//! build / vendor directories, and let a signal seen *anywhere* satisfy
//! the requirement.
//!
//! Mirrored in `scripts/validate.sh::check_pwa` — keep both in lockstep
//! when adding or modifying a rule (see [`super`] for the parity
//! policy).

use super::{Report, Violation};
use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn check(path: &Path, report: &mut Report) -> Result<()> {
    let mut signals = PwaSignals::default();
    walk_pwa(path, &mut signals)?;

    if !signals.opted_in() {
        return Ok(());
    }

    let mut missing: Vec<&str> = Vec::new();
    if !signals.manifest_link {
        missing.push("<link rel=\"manifest\"> in HTML head");
    }
    if !signals.manifest_name {
        missing.push("manifest `name` field");
    }
    if !signals.manifest_short_name {
        missing.push("manifest `short_name` field");
    }
    if !signals.manifest_start_url {
        missing.push("manifest `start_url` field");
    }
    if !signals.manifest_display {
        missing.push("manifest `display` field (standalone / minimal-ui / fullscreen)");
    }
    if !signals.manifest_theme_color {
        missing.push("manifest `theme_color` field");
    }
    if !signals.manifest_background_color {
        missing.push("manifest `background_color` field");
    }
    if !signals.manifest_icons {
        missing.push("manifest `icons` array");
    }
    if !signals.maskable_icon {
        missing.push("maskable 512×512 icon (icons entry with `purpose: maskable`)");
    }
    if !signals.service_worker_registered {
        missing.push("service worker registration (registerSW / register( / VitePWA / next-pwa)");
    }
    if !signals.offline_fallback {
        missing.push("offline navigateFallback (workbox navigateFallback or hand-rolled handler)");
    }
    if !signals.apple_touch_icon {
        missing.push("apple-touch-icon link in HTML head");
    }
    if !signals.apple_mobile_capable {
        missing.push("apple-mobile-web-app-capable meta tag");
    }
    if !signals.apple_title {
        missing.push("apple-mobile-web-app-title meta tag");
    }
    if !signals.theme_color_meta {
        missing.push("theme-color meta tag in HTML head");
    }
    if !signals.update_prompt {
        missing.push("user-visible update prompt component (UpdateToast / UpdatePrompt / ReloadBanner / useRegisterSW onNeedRefresh)");
    }
    if !signals.icon_source_pipeline {
        missing
            .push("icon-generation source (pwa-assets.config.* / `make icons` / `npm run icons`)");
    }
    if !signals.lighthouse_pwa {
        missing.push("Lighthouse `pwa` category in lighthouserc (minScore ≥ 0.9)");
    }

    if missing.is_empty() {
        return Ok(());
    }

    report.violations.push(Violation {
        spec_section: "§11.4",
        message: format!(
            "project opted into PWA but the PWA shape is incomplete; missing: {}",
            missing.join(", ")
        ),
    });
    Ok(())
}

#[derive(Default)]
struct PwaSignals {
    /// vite-plugin-pwa, next-pwa, @angular/pwa, workbox-* config or
    /// a checked-in `*.webmanifest` — any of these counts as opt-in.
    plugin_or_manifest_file: bool,
    /// `<link rel="manifest">` appears anywhere in HTML / templates.
    manifest_link: bool,
    /// Manifest fields, scanned across the repo (the manifest itself
    /// or the build-plugin literal that emits it).
    manifest_name: bool,
    manifest_short_name: bool,
    manifest_start_url: bool,
    manifest_display: bool,
    manifest_theme_color: bool,
    manifest_background_color: bool,
    manifest_icons: bool,
    maskable_icon: bool,
    /// A service-worker registration call or framework hook.
    service_worker_registered: bool,
    /// `navigateFallback` configured (workbox) or hand-rolled fetch
    /// handler returning a shell.
    offline_fallback: bool,
    /// Apple legacy install meta tags.
    apple_touch_icon: bool,
    apple_mobile_capable: bool,
    apple_title: bool,
    /// `<meta name="theme-color">` in HTML.
    theme_color_meta: bool,
    /// User-visible update affordance — a component name or the
    /// `onNeedRefresh` / `needRefresh` hook from `useRegisterSW`.
    update_prompt: bool,
    /// Icon pipeline — pwa-assets generator config, or a script
    /// target named `icons` in Makefile / package.json.
    icon_source_pipeline: bool,
    /// Lighthouse rc config asserts the `pwa` category.
    lighthouse_pwa: bool,
}

impl PwaSignals {
    /// A repo is "opted in" to PWA mandates as soon as any of the
    /// strongest fingerprints appears: a checked-in `.webmanifest`,
    /// a `<link rel="manifest">`, a vite-plugin-pwa / next-pwa /
    /// workbox config, or a `serviceWorker.register(` call.
    fn opted_in(&self) -> bool {
        self.plugin_or_manifest_file || self.manifest_link || self.service_worker_registered
    }
}

const PWA_EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".git",
    ".agents",
    ".claude",
    "__pycache__",
    ".venv",
    "venv",
];

/// Files worth scanning for PWA signal substrings. Stays text-only —
/// PNG and JPEG icons can't contain manifest fields, and reading them
/// as UTF-8 just wastes I/O.
fn is_pwa_scannable(file_name: &str) -> bool {
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
                | "webmanifest"
                | "yml"
                | "yaml"
        );
    }
    false
}

/// Strong opt-in fingerprints — a single match flips the project into
/// "must satisfy §11.4" mode.
fn is_plugin_or_manifest_file(file_name: &str) -> bool {
    // Explicit Web App Manifest filenames.
    if matches!(
        file_name,
        "manifest.webmanifest" | "manifest.json" | "site.webmanifest"
    ) {
        return true;
    }
    // vite-plugin-pwa companion config.
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name);
    matches!(stem, "pwa-assets.config")
}

fn walk_pwa(dir: &Path, signals: &mut PwaSignals) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
    {
        let p = entry.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if p.is_dir() {
            if PWA_EXCLUDED_DIRS.contains(&name) {
                continue;
            }
            walk_pwa(&p, signals)?;
            continue;
        }
        if !p.is_file() {
            continue;
        }

        if is_plugin_or_manifest_file(name) {
            signals.plugin_or_manifest_file = true;
        }

        if !is_pwa_scannable(name) {
            continue;
        }
        let content = match std::fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };
        scan_content(&content, name, signals);
    }
    Ok(())
}

fn scan_content(content: &str, file_name: &str, signals: &mut PwaSignals) {
    // Strong opt-in markers detectable from contents.
    if content.contains("vite-plugin-pwa")
        || content.contains("VitePWA(")
        || content.contains("next-pwa")
        || content.contains("withPWA(")
        || content.contains("@angular/pwa")
        || content.contains("workbox-build")
        || content.contains("workbox-webpack-plugin")
    {
        signals.plugin_or_manifest_file = true;
    }
    if content.contains("rel=\"manifest\"")
        || content.contains("rel='manifest'")
        || content.contains("manifest.webmanifest")
        || content.contains("manifest.json\"")
    {
        signals.manifest_link = true;
    }

    // Service-worker registration — explicit API call, framework
    // hooks, or build-plugin auto-injection.
    if content.contains("serviceWorker.register(")
        || content.contains("navigator.serviceWorker")
        || content.contains("useRegisterSW")
        || content.contains("registerSW(")
        || content.contains("VitePWA(")
        || content.contains("withPWA(")
        || content.contains("@angular/service-worker")
    {
        signals.service_worker_registered = true;
    }

    // Manifest fields — the actual `.webmanifest` JSON or the plugin
    // literal that emits one. We look for the field token plus the
    // colon that follows it in JSON-like contexts.
    let is_manifest_like = file_name.ends_with(".webmanifest")
        || file_name == "manifest.json"
        || file_name == "site.webmanifest"
        || content.contains("VitePWA(")
        || content.contains("withPWA(")
        || content.contains("workbox-build")
        || file_name.ends_with(".ts")
        || file_name.ends_with(".tsx")
        || file_name.ends_with(".js")
        || file_name.ends_with(".mjs")
        || file_name.ends_with(".cjs")
        || file_name.ends_with(".json");

    if is_manifest_like {
        // Tolerate `"name": …`, `name: …`, and quoted-key variants.
        let has = |needle_quoted: &str, needle_unquoted: &str| {
            content.contains(needle_quoted) || content.contains(needle_unquoted)
        };
        if has("\"name\":", "name:") {
            signals.manifest_name = true;
        }
        if has("\"short_name\":", "short_name:") {
            signals.manifest_short_name = true;
        }
        if has("\"start_url\":", "start_url:") {
            signals.manifest_start_url = true;
        }
        if has("\"display\":", "display:") {
            signals.manifest_display = true;
        }
        if has("\"theme_color\":", "theme_color:") {
            signals.manifest_theme_color = true;
        }
        if has("\"background_color\":", "background_color:") {
            signals.manifest_background_color = true;
        }
        if has("\"icons\":", "icons:") {
            signals.manifest_icons = true;
        }
        if content.contains("\"maskable\"") || content.contains("'maskable'") {
            signals.maskable_icon = true;
        }
    }

    // Offline fallback — workbox config or a service-worker fetch
    // handler returning a shell HTML.
    if content.contains("navigateFallback")
        || content.contains("navigate_fallback")
        || content.contains("precacheAndRoute")
    {
        signals.offline_fallback = true;
    }

    // iOS install meta tags + theme-color.
    if content.contains("apple-touch-icon") {
        signals.apple_touch_icon = true;
    }
    if content.contains("apple-mobile-web-app-capable")
        || content.contains("mobile-web-app-capable")
    {
        signals.apple_mobile_capable = true;
    }
    if content.contains("apple-mobile-web-app-title") {
        signals.apple_title = true;
    }
    if content.contains("name=\"theme-color\"") || content.contains("name='theme-color'") {
        signals.theme_color_meta = true;
    }

    // Update-prompt affordance.
    if content.contains("UpdateToast")
        || content.contains("UpdatePrompt")
        || content.contains("ReloadBanner")
        || content.contains("ReloadPrompt")
        || content.contains("onNeedRefresh")
        || content.contains("needRefresh")
        || content.contains("PWAUpdate")
    {
        signals.update_prompt = true;
    }

    // Icon-generation pipeline — pwa-assets config (already detected
    // by filename above) or an `icons` target in Makefile /
    // package.json scripts.
    if (file_name == "Makefile" || file_name == "Makefile.tmpl")
        && (content.contains("\nicons:") || content.starts_with("icons:"))
    {
        signals.icon_source_pipeline = true;
    }
    if (file_name == "package.json" || file_name == "package.json.tmpl")
        && (content.contains("\"icons\":") || content.contains("\"generate-pwa-assets\":"))
    {
        signals.icon_source_pipeline = true;
    }
    if content.contains("@vite-pwa/assets-generator") || content.contains("pwa-asset-generator") {
        signals.icon_source_pipeline = true;
    }

    // Lighthouse PWA assertion. Lighthouse CI's `assertions` map
    // names categories with the `categories:<name>` key syntax (e.g.
    // `"categories:pwa": ["error", { "minScore": 0.9 }]`). We also
    // accept a bare `"pwa"` token elsewhere in the config so that
    // looser shapes — e.g. a custom Lighthouse runner that lists
    // `categories: ["pwa"]` — still satisfy the rule.
    if file_name.starts_with("lighthouserc")
        && (content.contains("categories:pwa")
            || content.contains("\"pwa\"")
            || content.contains("'pwa'"))
    {
        signals.lighthouse_pwa = true;
    }
}
