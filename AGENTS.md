# Agent guidance for oss-spec

This file is the canonical source of truth for AI coding agents working in this repo. `CLAUDE.md`, `.cursorrules`, `.windsurfrules`, `GEMINI.md`, `.aider.conf.md`, and `.github/copilot-instructions.md` are symlinks to this file.

## What this repo is

`oss-spec` is a Rust CLI that bootstraps new open source repositories conforming to [`OSS_SPEC.md`](OSS_SPEC.md). The `init` subcommand takes an optional freeform prompt, sends it to the [`zag`](https://crates.io/crates/zag) library for LLM interpretation, then writes a complete repo to disk.

The repo is its own first customer: `oss-spec validate .` against this directory passes.

## OSS Spec conformance

This repository adheres to [`OSS_SPEC.md`](OSS_SPEC.md), which lives at the repository root and is also the canonical spec this CLI generates projects against. The spec is versioned in its YAML front matter (semver); **bump the `version` field every time you modify `OSS_SPEC.md`**. Use major for breaking changes to existing mandates, minor for new mandates or sections, patch for clarifications and typo fixes. Generated projects receive the spec via the symlink `templates/_common/OSS_SPEC.md -> ../../OSS_SPEC.md`, so edits to the root file automatically flow into every future bootstrap.

## Build and test commands

```sh
make build       # cargo build
make test        # cargo test
make lint        # cargo clippy --all-targets -- -D warnings
make fmt         # cargo fmt --all
make fmt-check   # cargo fmt --all -- --check
make release     # cargo build --release
make install     # cargo install --path .
```

## Commit and PR conventions

- All commits follow [Conventional Commits](https://www.conventionalcommits.org/).
- PRs are squash-merged; the **PR title** becomes the single commit on `main`, so it must follow conventional-commit format.
- Breaking changes use `<type>!:` or a `BREAKING CHANGE:` footer.

## Architecture summary

```
src/
├── main.rs        # tokio entry → lib::run
├── lib.rs         # public re-exports
├── cli.rs         # clap derive + dispatch
├── interview.rs   # interactive Q&A → ProjectManifest
├── ai.rs          # thin zag wrappers (interpret_prompt, draft_readme_why)
├── manifest.rs    # ProjectManifest, Language, Kind, License enums
├── render.rs      # minijinja env + render_str
├── embedded.rs    # include_dir!("templates")
├── bootstrap.rs   # walks embedded tree → writes target dir
├── git.rs         # git init / gh repo create wrappers
├── validate/      # §19 conformance validator (structural, content, toolchain, agent_skills)
├── fix.rs         # zag-driven auto-fix agent
├── agent_help.rs  # §12 CLI discoverability contract
└── output.rs      # central logging + styled output (§19 logging)
templates/         # all the files the bootstrap engine emits, with {{ jinja }} placeholders
docs/              # oss-spec's own user docs
man/               # oss-spec's own manpage(s)
```

Dependency direction is top-down: `main` → `lib` → `cli` → (`interview`, `bootstrap`, `validate`, `agent_help`); each leaf module owns its own concern. `ai.rs` is the **only** module allowed to `use zag::*` — keep zag isolated behind it so AI failures stay non-fatal and `--no-ai` continues to work.

## Where new code goes

| Change type | Goes in |
|---|---|
| New CLI flag / subcommand | `src/cli.rs` (clap) + `src/agent_help.rs` (commands table, COMMAND_SPECS, EXAMPLES) + `man/oss-spec.md` |
| New template file | `templates/_common/`, `templates/<lang>/`, or `templates/cli/` |
| New §19 conformance rule | `src/validate/` (structural checks in `structural.rs`, content checks in `content.rs`, toolchain in `toolchain.rs`, agent skills in `agent_skills.rs`) |
| New auto-fix behavior | `src/fix.rs` (zag agent orchestration) |
| New AI-driven step | `src/ai.rs` (thin wrapper) + caller in `interview.rs` |
| New language overlay | `templates/<lang>/`, plus `Language` enum variant in `manifest.rs` |
| Tests | `tests/` |
| New agent skill | `.agent/skills/<name>/SKILL.md` (+ `.last-updated`); also `templates/_common/.agent/skills/<name>/` for generated projects |

## Test conventions

- **All tests live in `tests/`** as separate files — never as inline `#[cfg(test)]` blocks in `src/`. This keeps source files free of test scaffolding and lets agents, hooks, and linters treat source and test code differently.
- Test files are named `<module>_test.rs` (e.g. `validate_test.rs`, `prompts_test.rs`). The stem must end with `_test` or `_tests` per §20 of `OSS_SPEC.md`.
- Functions that tests need to call must be `pub` on the library crate.
- Use `tempfile::tempdir()` for any test that writes files.
- Snapshot tests use `insta`. The self-conformance test runs `validate::run(".")` against this repo and must always pass.

## Source file size (§20.5)

- Non-test source files must stay under **1000 physical lines**. When a file crosses the limit, the fix is almost always to split by concern — see `src/validate/` for the canonical example (the validator was split into `mod`, `structural`, `content`, `agent_skills`, and `toolchain` for exactly this reason).
- A file may opt out with an `oss-spec:allow-large-file: <reason>` marker in any comment within its first 20 lines. The reason must be non-empty and genuinely justify the size — reviewers will push back on markers that just paper over a skippable refactor. Valid motivations: generated code, cohesive state machines, third-party snapshots, inherent rule-catalogue density.
- When `oss-spec fix` runs into a §20.5 violation, it only attempts the easy refactor (extracting an inline `#[cfg(test)]` block, which often resolves both §20 and §20.5 at once). Genuinely large source files are left for a human to split or annotate manually.

## Documentation sync points

When you change… | Update…
--- | ---
A CLI flag or subcommand | `man/oss-spec.md`, `docs/agent/help-agent.txt`, `agent_help::COMMANDS_TABLE`, `agent_help::COMMAND_SPECS`, `README.md` Usage table
A template file | `templates/_common/` (or overlay) — and re-run `oss-spec validate` against a generated demo
A §19 rule | `src/validate/` (appropriate submodule), `OSS_SPEC.md`, this `## Documentation sync points` table
A toolchain version bump (Rust / Python / Node / Go) | the repo-root pin file (`rust-toolchain.toml`, `.python-version`, `.nvmrc`, or `go.mod`'s `toolchain` directive), its `templates/<lang>/` counterpart, `templates/_common/.github/workflows/ci.yml.tmpl`, and `MIN_TOOLCHAIN_VERSIONS` in `src/validate/toolchain.rs` (§10.5 local/CI parity, §10.3 minimums)
An LLM prompt's source of truth (spec text, validator rule, manifest enum, rendering-context key) | A new file under `prompts/<name>/<major>_<minor>_<patch>.md` (never edit an existing versioned file — bump semver and create a new one per §13.5). Touch `src/prompts.rs` afterwards (e.g. `touch src/prompts.rs`) so the `include_dir!` proc-macro picks up the new embedded file on the next build. Run the `update-prompts` skill or let the `maintenance` sweep pick it up.
The list of supported languages | `manifest::Language`, `templates/<lang>/`, `Makefile.tmpl`, `ci.yml.tmpl`, `dependabot.yml.tmpl`
`OSS_SPEC.md` | Bump the `version` field in its YAML front matter (semver — `feat!`/breaking bumps major, `feat` or new mandate bumps minor, pure clarifications bump patch). Also update `README.md`, `docs/`, `templates/_common/AGENTS.md.tmpl`, and this file as needed. The spec is mirrored into generated projects via the symlink `templates/_common/OSS_SPEC.md -> ../../OSS_SPEC.md`, so there is only one source of truth.

## Reference implementation

This repo is the canonical reference implementation of `OSS_SPEC.md`. Other projects bootstrapped by oss-spec — and developers reading the spec — look at this repo to understand what a conformant project looks like in practice. This means:

- **Layout and structure must exemplify the spec.** Every required file, directory, symlink, and workflow that `OSS_SPEC.md` mandates should be present and well-maintained here — not just passing the automated check, but serving as a good example of *how* to do it.
- **When in doubt, look at the spec.** If you're unsure whether a change is appropriate, read the relevant section of [`OSS_SPEC.md`](OSS_SPEC.md). The spec is the source of truth; this repo is its embodiment.
- **Quality over minimum compliance.** The README should have a genuine "Why?" section, the docs should be useful, the CI workflows should be production-grade, the AGENTS.md should be thorough. Automated checks verify presence; you should verify quality.
- **Changes here ripple outward.** Patterns established in this repo get replicated by the bootstrap engine into every generated project. Sloppy conventions here become sloppy conventions everywhere.

## Parity / cross-cutting rules

- **Embedded templates discipline.** `templates/`, `docs/`, and `man/` are compiled into the binary via `include_dir!`. After adding a new file under any of those directories, run `cargo clean && cargo build` to make sure it gets picked up.
- **AGENTS.md symlinks.** When generating projects, all of `CLAUDE.md`, `.cursorrules`, `.windsurfrules`, `GEMINI.md`, `.aider.conf.md`, and `.github/copilot-instructions.md` must be symlinks to `AGENTS.md`. The same rule applies to this repo.
- **`zag` is isolated.** Only `src/ai.rs` may import from `zag`. Every AI call must have a deterministic fallback so `--no-ai` keeps working.
- **`oss-spec validate .` must always pass on this repo.** A self-conformance test (`tests/self_conformance.rs`) enforces it; if you add a new §19 rule that breaks the dogfood, fix the dogfood at the same time.
- **No raw prints.** All user-facing output goes through `src/output.rs` (`output::status`, `output::warn`, `output::info`, `output::header`, `output::error`). The only exception is machine-readable §12 contract output in `agent_help.rs`. Use `log::debug!` for verbose diagnostics.

## Maintenance skills

Per §21 of `OSS_SPEC.md`, this repo ships agent skills for keeping drift-prone artifacts in sync with their sources of truth. Skills live under `.agent/skills/<name>/`; `.claude/skills` is a symlink into that tree so Claude Code picks them up at their canonical location.

| Skill | When to run | Artifacts it fixes |
|---|---|---|
| `maintenance`     | When several artifacts have likely drifted at once (after a big merge, before a release) — umbrella skill that dispatches to every `update-*` skill in order | all of the below, in one sweep |
| `update-spec`     | Whenever `OSS_SPEC.md` is edited — propagates the new mandate through `validate.rs`, tests, templates, and docs | everything downstream of the spec |
| `update-manpages` | Whenever `src/cli.rs` clap definitions change | `man/oss-spec.md` |
| `update-docs`     | Whenever user-visible behavior described in `docs/` changes | `docs/*.md` |
| `update-readme`   | Whenever a CLI flag, subcommand, §19 rule, supported language, or the spec version changes | `README.md` |
| `update-prompts`  | Whenever `OSS_SPEC.md`, `src/validate.rs`, `src/ai.rs`, `src/fix.rs`, or `src/manifest.rs` changes in a way that could leave a prompt template stale | `prompts/**/*.md` |
| `update-website`  | Whenever a source-derived section of the website (hero, version, CLI table) drifts from README / docs / spec | `website/` |
| `sync-oss-spec`   | Whenever `oss-spec validate .` reports violations, or after a spec bump — brings repo contents back into conformance with `OSS_SPEC.md` | repo-wide §19 conformance |
| `commit`          | After any feature or fix, to run quality gates, commit, push, and open/update the PR | — |

The `maintenance` skill reads a **Registry** table (its single source of truth) that lists every `update-*` skill and the order they must run in. When you add a new `update-*` skill, add a matching row to `maintenance/SKILL.md` — `oss-spec validate .` treats a missing row as a drift bug.

Each skill has a `SKILL.md` (the playbook) and a `.last-updated` file (baseline commit hash). A run ends by rewriting `.last-updated` with the current `HEAD` so the next run sees a smaller diff. Skills are expected to improve their own mapping tables when they discover new drift paths — commit those edits alongside the artifact edits.
