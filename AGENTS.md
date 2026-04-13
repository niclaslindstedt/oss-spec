# Agent guidance for oss-spec

This file is the canonical source of truth for AI coding agents working in this repo. `CLAUDE.md`, `.cursorrules`, `.windsurfrules`, `GEMINI.md`, `.aider.conf.md`, and `.github/copilot-instructions.md` are symlinks to this file.

## What this repo is

`oss-spec` is a Rust CLI that bootstraps new open source repositories conforming to [`OSS_SPEC.md`](OSS_SPEC.md). The default invocation takes a freeform prompt, sends it to the [`zag`](https://crates.io/crates/zag) library for LLM interpretation, then writes a complete repo to disk.

The repo is its own first customer: `oss-spec check .` against this directory passes.

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
├── check.rs       # §19 conformance validator
├── agent_help.rs  # §12 CLI discoverability contract
└── output.rs      # central logging + styled output (§19 logging)
templates/         # all the files the bootstrap engine emits, with {{ jinja }} placeholders
docs/              # oss-spec's own user docs
man/               # oss-spec's own manpage(s)
```

Dependency direction is top-down: `main` → `lib` → `cli` → (`interview`, `bootstrap`, `check`, `agent_help`); each leaf module owns its own concern. `ai.rs` is the **only** module allowed to `use zag::*` — keep zag isolated behind it so AI failures stay non-fatal and `--no-ai` continues to work.

## Where new code goes

| Change type | Goes in |
|---|---|
| New CLI flag / subcommand | `src/cli.rs` (clap) + `src/agent_help.rs` (commands table, COMMAND_SPECS, EXAMPLES) + `man/oss-spec.md` |
| New template file | `templates/_common/`, `templates/<lang>/`, or `templates/cli/` |
| New §19 conformance rule | `src/check.rs` |
| New AI-driven step | `src/ai.rs` (thin wrapper) + caller in `interview.rs` |
| New language overlay | `templates/<lang>/`, plus `Language` enum variant in `manifest.rs` |
| Tests | `tests/` |

## Test conventions

- Unit tests live next to the code they test in `#[cfg(test)] mod tests` blocks.
- Integration tests live in `tests/`. Use `tempfile::tempdir()` for any test that writes files.
- Snapshot tests use `insta`. The self-conformance test runs `check::run(".")` against this repo and must always pass.

## Documentation sync points

When you change… | Update…
--- | ---
A CLI flag or subcommand | `man/oss-spec.md`, `docs/agent/help-agent.txt`, `agent_help::COMMANDS_TABLE`, `agent_help::COMMAND_SPECS`, `README.md` Usage table
A template file | `templates/_common/` (or overlay) — and re-run `oss-spec check` against a generated demo
A §19 rule | `src/check.rs`, `OSS_SPEC.md`, this `## Documentation sync points` table
The list of supported languages | `manifest::Language`, `templates/<lang>/`, `Makefile.tmpl`, `ci.yml.tmpl`, `dependabot.yml.tmpl`
`OSS_SPEC.md` | `README.md`, `docs/`, `templates/_common/AGENTS.md.tmpl`, this file

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
- **`oss-spec check .` must always pass on this repo.** A self-conformance test (`tests/self_conformance.rs`) enforces it; if you add a new §19 rule that breaks the dogfood, fix the dogfood at the same time.
- **No raw prints.** All user-facing output goes through `src/output.rs` (`output::status`, `output::warn`, `output::info`, `output::header`, `output::error`). The only exception is machine-readable §12 contract output in `agent_help.rs`. Use `log::debug!` for verbose diagnostics.
