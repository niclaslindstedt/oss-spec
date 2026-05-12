#!/usr/bin/env node
// §11.3.1 — post-build prerender. Builds an SSR bundle of
// `entry-server.tsx` with Vite, imports it, renders the React tree to a
// static HTML string, and splices the result into `dist/index.html`
// (replacing the empty `<div id="root"></div>` shell). The client entry
// then `hydrateRoot`s over the prerendered tree, so the page ships a
// non-empty `<body>` to crawlers AND to the React app without a
// double-render.
//
// Also emits `dist/404.html` from the same SSR pass but with the
// robots meta forced to `noindex,follow` per §11.3.2 (so GitHub Pages'
// SPA fallback does not leak soft-404 signals on guessed URLs).
//
// A separate `vite build --ssr` (rather than the programmatic
// `ssrLoadModule` runner) is the path that survives react-router-dom's
// CJS export shape — the runner's ESM interop chokes on it but the
// build pipeline normalizes everything via Rollup.

import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { build } from "vite";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const DIST = resolve(ROOT, "dist");
const SSR_DIR = resolve(ROOT, ".ssr-build");

// Build the server entry. `ssr` mode produces a Node-compatible bundle
// with no client-side concerns; CSS/asset imports are stripped.
await build({
  root: ROOT,
  configFile: resolve(ROOT, "vite.config.ts"),
  logLevel: "warn",
  build: {
    ssr: resolve(ROOT, "src/entry-server.tsx"),
    outDir: SSR_DIR,
    emptyOutDir: true,
    rollupOptions: {
      output: {
        format: "esm",
        entryFileNames: "entry-server.mjs",
      },
    },
  },
});

const ssrEntry = pathToFileURL(resolve(SSR_DIR, "entry-server.mjs")).href;
const { render } = await import(ssrEntry);

const indexPath = resolve(DIST, "index.html");
const template = readFileSync(indexPath, "utf8");
// Match the basename declared on the client `<BrowserRouter>` so the
// router renders the root route during SSR.
const appHtml = render("/oss-spec/");
const indexHtml = template.replace(
  '<div id="root"></div>',
  `<div id="root">${appHtml}</div>`,
);
writeFileSync(indexPath, indexHtml, "utf8");
console.log("Wrote dist/index.html (prerendered)");

// 404: same body, but `noindex,follow`. The check-seo script asserts
// this distinction so soft-404 signals can't leak.
const notFoundHtml = indexHtml.replace(
  /<meta name="robots" content="[^"]+"/,
  '<meta name="robots" content="noindex,follow"',
);
writeFileSync(resolve(DIST, "404.html"), notFoundHtml, "utf8");
console.log("Wrote dist/404.html (noindex)");

// Throw away the SSR bundle; it has served its purpose.
rmSync(SSR_DIR, { recursive: true, force: true });
