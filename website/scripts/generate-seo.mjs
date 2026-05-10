#!/usr/bin/env node
// Post-build SEO generator. Runs after `vite build`; emits the SEO files
// the spec mandates (§11.3) into dist/:
//
//   - sitemap.xml   — every public route the project wants indexed
//   - robots.txt    — `Allow: /` plus an absolute Sitemap: line
//
// Per-route <head> metadata (Open Graph, Twitter Card, JSON-LD) is baked
// into website/index.html itself, since the current site is a single
// page. When the site grows additional public routes, splice their
// per-route <head> blocks in here and add them to SITEMAP_URLS below.

import { writeFileSync, mkdirSync, statSync } from "fs";
import { resolve, dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const DIST = resolve(__dirname, "../dist");
const SITE_URL = "https://niclaslindstedt.github.io/oss-spec";

// Public routes the sitemap should list. `lastmod` comes from the
// real mtime of a representative source file so search engines see a
// truthful last-modified date instead of `now()`.
const SITEMAP_URLS = [
  {
    loc: `${SITE_URL}/`,
    lastmod: lastmodOf(resolve(__dirname, "../../OSS_SPEC.md")),
    changefreq: "weekly",
    priority: "1.0",
  },
];

function lastmodOf(absPath) {
  try {
    return new Date(statSync(absPath).mtime).toISOString();
  } catch {
    return new Date().toISOString();
  }
}

function escapeXml(s) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function renderSitemap() {
  const body = SITEMAP_URLS.map(
    (u) =>
      `  <url>\n    <loc>${escapeXml(u.loc)}</loc>\n    <lastmod>${escapeXml(u.lastmod)}</lastmod>\n    <changefreq>${u.changefreq}</changefreq>\n    <priority>${u.priority}</priority>\n  </url>`,
  ).join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${body}\n</urlset>\n`;
}

function renderRobots() {
  return `User-agent: *\nAllow: /\n\nSitemap: ${SITE_URL}/sitemap.xml\n`;
}

function writeOut(rel, body) {
  const full = join(DIST, rel);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, body, "utf-8");
  console.log(`Wrote ${rel}`);
}

writeOut("sitemap.xml", renderSitemap());
writeOut("robots.txt", renderRobots());
