#!/usr/bin/env node
// Extract structured data from Rust source files and generate sourceData.ts.
//
// Usage: node scripts/extract-source-data.mjs
// Run from the website/ directory.
//
// This replaces hardcoded website data with values parsed from the actual
// Rust source, so the website stays in sync with the codebase.

import { readFileSync, writeFileSync, mkdirSync } from "fs";
import { resolve, dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "../..");

function read(relPath) {
  return readFileSync(join(REPO_ROOT, relPath), "utf-8");
}

// ---------------------------------------------------------------------------
// 1. Version (from Cargo.toml)
// ---------------------------------------------------------------------------

function extractVersion() {
  const cargo = read("Cargo.toml");
  const m = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("Could not extract version from Cargo.toml");
  return m[1];
}

// ---------------------------------------------------------------------------
// 2. Commands (from src/cli.rs — Command enum)
// ---------------------------------------------------------------------------

function extractCommands() {
  const src = read("src/cli.rs");

  const block = src.match(/pub enum Command\s*\{([\s\S]*?)^}/m);
  if (!block) throw new Error("Could not find Command enum in cli.rs");

  const commands = [];
  const re = /\/\/\/\s*(.+(?:\n\s*\/\/\/\s*.+)*)\n\s+(\w+)\s*[\{,]/g;
  let m;
  while ((m = re.exec(block[1])) !== null) {
    const desc = m[1].replace(/\n\s*\/\/\/\s*/g, " ").trim();
    const name = camelToKebab(m[2]);
    commands.push({ name, description: desc });
  }
  return commands;
}

function camelToKebab(s) {
  return s.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

// ---------------------------------------------------------------------------
// 3. Languages (from src/manifest.rs — Language enum)
// ---------------------------------------------------------------------------

function extractEnumVariants(src, enumName) {
  const block = src.match(new RegExp(`pub enum ${enumName}\\s*\\{([\\s\\S]*?)^}`, "m"));
  if (!block) return [];
  return [...block[1].matchAll(/^\s+(\w+),?$/gm)]
    .map((m) => m[1])
    .filter((v) => !v.startsWith("#"));
}

function extractLanguages() {
  const src = read("src/manifest.rs");
  return extractEnumVariants(src, "Language").map((v) => v.toLowerCase());
}

// ---------------------------------------------------------------------------
// 4. Kinds (from src/manifest.rs — Kind enum)
// ---------------------------------------------------------------------------

function extractKinds() {
  const src = read("src/manifest.rs");
  return extractEnumVariants(src, "Kind").map((v) => v.toLowerCase());
}

// ---------------------------------------------------------------------------
// 5. Licenses (from src/manifest.rs — License enum spdx method)
// ---------------------------------------------------------------------------

function extractLicenses() {
  const src = read("src/manifest.rs");
  const spdxBlock = src.match(/pub fn spdx\(self\)[^{]*\{([\s\S]*?)\n\s{4}\}/);
  if (!spdxBlock) return [];
  return [...spdxBlock[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
}

// ---------------------------------------------------------------------------
// 6. Spec checks (from src/check.rs)
// ---------------------------------------------------------------------------

function extractSpecChecks() {
  const src = read("src/check.rs");
  const checks = [];

  // Required files
  const filesBlock = src.match(/let required_files[^=]*=\s*&\[([\s\S]*?)\];/);
  if (filesBlock) {
    for (const m of filesBlock[1].matchAll(/\("([^"]+)",\s*"([^"]+)"\)/g)) {
      checks.push({ item: m[1], section: m[2], type: "file" });
    }
  }

  // Symlinks
  const symBlock = src.match(/let symlinks[^=]*=\s*&\[([\s\S]*?)\];/);
  if (symBlock) {
    for (const m of symBlock[1].matchAll(/\("([^"]+)",\s*"([^"]+)"\)/g)) {
      checks.push({ item: `${m[1]} -> AGENTS.md`, section: m[2], type: "symlink" });
    }
  }

  // Required directories
  const dirBlock = src.match(/let required_dirs[^=]*=\s*&\[([\s\S]*?)\];/);
  if (dirBlock) {
    for (const m of dirBlock[1].matchAll(/\("([^"]+)",\s*"([^"]+)"\)/g)) {
      checks.push({ item: `${m[1]}/`, section: m[2], type: "directory" });
    }
  }

  // Required workflows
  const wfBlock = src.match(/let required_workflows[^=]*=\s*&\[([\s\S]*?)\];/);
  if (wfBlock) {
    for (const m of wfBlock[1].matchAll(/"([^"]+)"/g)) {
      checks.push({ item: `.github/workflows/${m[1]}`, section: "§10", type: "workflow" });
    }
  }

  return checks;
}

// ---------------------------------------------------------------------------
// Generate output
// ---------------------------------------------------------------------------

function generate() {
  const version = extractVersion();
  const commands = extractCommands();
  const languages = extractLanguages();
  const kinds = extractKinds();
  const licenses = extractLicenses();
  const specChecks = extractSpecChecks();

  const output = `// AUTO-GENERATED from Rust source — do not edit manually.
// To regenerate: npm run extract (from website/)
// Source files:
//   - Cargo.toml (version)
//   - src/cli.rs (commands)
//   - src/manifest.rs (languages, kinds, licenses)
//   - src/check.rs (spec checks)

// --- Types ---

export interface CommandData {
  name: string;
  description: string;
}

export interface SpecCheck {
  item: string;
  section: string;
  type: "file" | "symlink" | "directory" | "workflow";
}

// --- Data ---

export const version = ${JSON.stringify(version)};

export const commands: CommandData[] = ${JSON.stringify(commands, null, 2)};

export const languages: string[] = ${JSON.stringify(languages)};

export const kinds: string[] = ${JSON.stringify(kinds)};

export const licenses: string[] = ${JSON.stringify(licenses)};

export const specChecks: SpecCheck[] = ${JSON.stringify(specChecks, null, 2)};
`;

  const outDir = join(__dirname, "../src/data");
  mkdirSync(outDir, { recursive: true });
  const outPath = join(outDir, "sourceData.ts");
  writeFileSync(outPath, output, "utf-8");
  console.log(`Generated ${outPath}`);
  console.log(`  Version: ${version}`);
  console.log(`  Commands: ${commands.length}`);
  console.log(`  Languages: ${languages.length}`);
  console.log(`  Kinds: ${kinds.length}`);
  console.log(`  Licenses: ${licenses.length}`);
  console.log(`  Spec checks: ${specChecks.length}`);
}

generate();
