//! §11.3.12 — unlisted websites (a testing surface, not a public site).
//!
//! A project declares its website unlisted with an
//! `oss-spec:unlisted-website: <reason>` line in AGENTS.md. The validator
//! then skips the §11.3 scaffolding, the §11.3.10 workflows, and the
//! §11.4.7 Lighthouse PWA assertion, and checks for `noindex` instead.
//! Projects without the marker must be validated exactly as before.

use oss_spec::validate::{
    self, declares_unlisted_website, has_robots_noindex_meta, has_unlisted_marker,
    robots_txt_disallows_all,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

const MARKER_LINE: &str = "oss-spec:unlisted-website: the web build is a testing surface; \
                           users install the app from its store listing\n";

fn in_section<'a>(report: &'a validate::Report, section: &str) -> Vec<&'a validate::Violation> {
    report
        .violations
        .iter()
        .filter(|v| v.spec_section == section)
        .collect()
}

fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
}

/// A website with no SEO scaffolding at all: an index.html and a Pages
/// deploy. `head` is spliced into the page's `<head>`.
fn website(root: &Path, head: &str) {
    write(
        root,
        "index.html",
        &format!("<!doctype html><html><head><title>App</title>{head}</head><body></body></html>"),
    );
    write(
        root,
        ".github/workflows/pages.yml",
        "jobs:\n  deploy:\n    steps:\n      - uses: actions/deploy-pages@v4\n",
    );
}

/// Every §11.4 signal except the Lighthouse PWA assertion (§11.4.7).
fn pwa_without_lighthouse(root: &Path) {
    write(
        root,
        "public/manifest.webmanifest",
        r##"{ "name": "Calc", "short_name": "Calc", "start_url": "/", "scope": "/", "id": "/",
             "display": "standalone", "theme_color": "#111111", "background_color": "#111111",
             "icons": [
               { "src": "/pwa-192x192.png", "sizes": "192x192", "type": "image/png" },
               { "src": "/pwa-512x512.png", "sizes": "512x512", "type": "image/png" },
               { "src": "/maskable-512x512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable" }
             ] }"##,
    );
    write(
        root,
        "src/pwa.html",
        r##"<link rel="manifest" href="/manifest.webmanifest" />
            <link rel="apple-touch-icon" href="/apple-touch-icon-180x180.png" />
            <meta name="apple-mobile-web-app-capable" content="yes" />
            <meta name="apple-mobile-web-app-title" content="Calc" />
            <meta name="theme-color" content="#111111" />"##,
    );
    write(
        root,
        "src/sw-register.ts",
        r##"import { registerSW } from 'virtual:pwa-register';
            const updateSW = registerSW({ onNeedRefresh() { /* <UpdateToast/> */ } });
            // workbox: { navigateFallback: '/index.html' }
        "##,
    );
    write(
        root,
        "pwa-assets.config.ts",
        "import { defineConfig } from '@vite-pwa/assets-generator/config';\n",
    );
}

// The marker --------------------------------------------------------------

#[test]
fn marker_with_reason_declares_unlisted() {
    assert!(has_unlisted_marker(MARKER_LINE));
    assert!(has_unlisted_marker(&format!(
        "# Agent guidance\n\n<!-- {} -->\n",
        MARKER_LINE.trim()
    )));
    assert!(has_unlisted_marker(
        "The site is `oss-spec:unlisted-website: testing only`.\n"
    ));
}

#[test]
fn marker_without_reason_does_not_declare() {
    assert!(!has_unlisted_marker("oss-spec:unlisted-website:\n"));
    assert!(!has_unlisted_marker("oss-spec:unlisted-website:    \n"));
    assert!(!has_unlisted_marker(
        "# Agent guidance\n\nNo marker here.\n"
    ));
}

#[test]
fn marker_is_read_from_agents_md_only() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    assert!(!declares_unlisted_website(root), "no AGENTS.md at all");
    write(root, "README.md", MARKER_LINE);
    assert!(
        !declares_unlisted_website(root),
        "the marker counts only in AGENTS.md"
    );
    write(root, "AGENTS.md", MARKER_LINE);
    assert!(declares_unlisted_website(root));
}

// The noindex and robots.txt helpers --------------------------------------

#[test]
fn robots_noindex_meta_is_recognised() {
    assert!(has_robots_noindex_meta(
        r#"<meta name="robots" content="noindex" />"#
    ));
    assert!(has_robots_noindex_meta(
        r#"<META content="noindex,nofollow" name="Robots">"#
    ));
    assert!(has_robots_noindex_meta(
        "<meta\n  name=\"robots\"\n  content=\"noindex\"\n/>"
    ));
    assert!(has_robots_noindex_meta(
        r#"head.push('<meta name="robots" content="noindex">')"#
    ));
}

#[test]
fn indexable_or_unrelated_meta_is_not_noindex() {
    assert!(!has_robots_noindex_meta(
        r#"<meta name="robots" content="index,follow" />"#
    ));
    assert!(!has_robots_noindex_meta(
        r#"<meta name="description" content="noindex is a word"><meta name="robots" content="all">"#
    ));
    assert!(!has_robots_noindex_meta("robots: noindex"));
}

#[test]
fn robots_txt_disallow_all_is_detected() {
    assert!(robots_txt_disallows_all("User-agent: *\nDisallow: /\n"));
    assert!(robots_txt_disallows_all(
        "User-agent: *\ndisallow:/   # all\n"
    ));
    assert!(!robots_txt_disallows_all("User-agent: *\nAllow: /\n"));
    assert!(!robots_txt_disallows_all("User-agent: *\nDisallow:\n"));
    assert!(!robots_txt_disallows_all(
        "User-agent: *\nDisallow: /private/\n"
    ));
    assert!(!robots_txt_disallows_all("# Disallow: /\nUser-agent: *\n"));
}

// An exempt project -------------------------------------------------------

#[test]
fn unlisted_website_with_noindex_passes() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    website(root, r#"<meta name="robots" content="noindex" />"#);
    write(
        root,
        "public/robots.txt",
        "# Crawling stays allowed so the noindex is seen.\nUser-agent: *\nAllow: /\n",
    );

    let report = validate::run(root).unwrap();
    for section in ["§11.3", "§11.3.10", "§11.3.12"] {
        assert!(
            in_section(&report, section).is_empty(),
            "an unlisted website with noindex must not be flagged for {section}: {:?}",
            report.violations
        );
    }
}

#[test]
fn unlisted_website_without_noindex_is_violation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    website(root, r#"<meta name="robots" content="index,follow" />"#);

    let report = validate::run(root).unwrap();
    let v = in_section(&report, "§11.3.12");
    assert_eq!(v.len(), 1, "expected one §11.3.12 violation: {v:?}");
    assert!(v[0].message.contains("noindex"), "{}", v[0].message);
    assert!(
        in_section(&report, "§11.3").is_empty(),
        "the SEO scaffolding check must be skipped: {:?}",
        report.violations
    );
}

#[test]
fn unlisted_website_robots_txt_must_allow_crawling() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    website(root, r#"<meta name="robots" content="noindex" />"#);
    write(root, "public/robots.txt", "User-agent: *\nDisallow: /\n");

    let report = validate::run(root).unwrap();
    let v = in_section(&report, "§11.3.12");
    assert_eq!(v.len(), 1, "expected one §11.3.12 violation: {v:?}");
    assert!(
        v[0].message.contains("public/robots.txt"),
        "{}",
        v[0].message
    );
}

#[test]
fn unlisted_marker_drops_seo_and_lighthouse_workflows() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    fs::create_dir_all(root.join(".github/workflows")).unwrap();

    let report = validate::run(root).unwrap();
    assert!(
        in_section(&report, "§11.3.10").is_empty(),
        "seo.yml / lighthouse.yml must not be required: {:?}",
        report.violations
    );
    // The §10 workflows are still required.
    assert!(
        report
            .violations
            .iter()
            .any(|v| v.message == "missing .github/workflows/pages.yml"),
        "pages.yml must still be required: {:?}",
        report.violations
    );
}

#[test]
fn unlisted_pwa_needs_no_lighthouse_assertion() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    website(root, r#"<meta name="robots" content="noindex" />"#);
    pwa_without_lighthouse(root);

    let report = validate::run(root).unwrap();
    assert!(
        in_section(&report, "§11.4").is_empty(),
        "an unlisted PWA must not need a Lighthouse PWA assertion: {:?}",
        report.violations
    );
}

#[test]
fn unlisted_pwa_still_needs_the_rest_of_the_pwa_shape() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", MARKER_LINE);
    website(root, r#"<meta name="robots" content="noindex" />"#);
    write(
        root,
        "public/manifest.webmanifest",
        r#"{ "name": "Calc", "start_url": "/" }"#,
    );

    let report = validate::run(root).unwrap();
    let v = in_section(&report, "§11.4");
    assert_eq!(v.len(), 1, "expected one §11.4 violation: {v:?}");
    assert!(v[0].message.contains("maskable"), "{}", v[0].message);
    assert!(
        !v[0].message.contains("Lighthouse"),
        "§11.4.7 must not be listed: {}",
        v[0].message
    );
}

// A non-exempt project: validated exactly as before ------------------------

#[test]
fn listed_website_with_noindex_still_needs_seo_scaffolding() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "# Agent guidance\n");
    website(root, r#"<meta name="robots" content="noindex" />"#);

    let report = validate::run(root).unwrap();
    let v = in_section(&report, "§11.3");
    assert_eq!(v.len(), 1, "expected one §11.3 violation: {v:?}");
    for piece in [
        "JSON-LD",
        "sitemap.xml",
        "llms.txt",
        "check-seo",
        "lighthouse",
    ] {
        assert!(v[0].message.contains(piece), "{piece}: {}", v[0].message);
    }
    assert!(
        in_section(&report, "§11.3.12").is_empty(),
        "§11.3.12 applies only to unlisted websites: {:?}",
        report.violations
    );
}

#[test]
fn listed_project_still_requires_seo_and_lighthouse_workflows() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "# Agent guidance\n");
    fs::create_dir_all(root.join(".github/workflows")).unwrap();

    let report = validate::run(root).unwrap();
    let v: Vec<_> = in_section(&report, "§11.3.10")
        .into_iter()
        .map(|v| v.message.as_str())
        .collect();
    assert_eq!(
        v,
        [
            "missing .github/workflows/seo.yml",
            "missing .github/workflows/lighthouse.yml"
        ]
    );
}

#[test]
fn marker_without_reason_is_validated_as_listed() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "oss-spec:unlisted-website:\n");
    website(root, r#"<meta name="robots" content="noindex" />"#);
    pwa_without_lighthouse(root);

    let report = validate::run(root).unwrap();
    assert_eq!(in_section(&report, "§11.3").len(), 1);
    assert_eq!(in_section(&report, "§11.3.10").len(), 2);
    let pwa = in_section(&report, "§11.4");
    assert_eq!(pwa.len(), 1, "expected one §11.4 violation: {pwa:?}");
    assert!(pwa[0].message.contains("Lighthouse"), "{}", pwa[0].message);
}

#[test]
fn listed_website_robots_txt_is_not_checked_by_unlisted_rule() {
    // The `Disallow: /` rule is §11.3.12's; a listed website's robots.txt
    // is covered by check-seo at build time (§11.3.10), not here.
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(root, "AGENTS.md", "# Agent guidance\n");
    website(root, "");
    write(root, "public/robots.txt", "User-agent: *\nDisallow: /\n");

    let report = validate::run(root).unwrap();
    assert!(in_section(&report, "§11.3.12").is_empty());
}
