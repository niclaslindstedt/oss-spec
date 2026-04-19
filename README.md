# oss-spec

> Bootstrap new open source repositories that conform to [`OSS_SPEC.md`](OSS_SPEC.md), with optional AI-driven project interpretation via [`zag`](https://crates.io/crates/zag).

[![ci](https://github.com/niclaslindstedt/oss-spec/actions/workflows/ci.yml/badge.svg)](https://github.com/niclaslindstedt/oss-spec/actions/workflows/ci.yml)
[![release](https://github.com/niclaslindstedt/oss-spec/actions/workflows/release.yml/badge.svg)](https://github.com/niclaslindstedt/oss-spec/actions/workflows/release.yml)
[![pages](https://github.com/niclaslindstedt/oss-spec/actions/workflows/pages.yml/badge.svg)](https://github.com/niclaslindstedt/oss-spec/actions/workflows/pages.yml)
[![crates](https://img.shields.io/crates/v/oss-spec.svg)](https://crates.io/crates/oss-spec)
[![spec](https://img.shields.io/badge/OSS__SPEC-v2.3.0-blueviolet)](OSS_SPEC.md)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Why?

- **One command to a real repo.** `oss-spec init "create a python cli for finding stock buys"` produces a complete project — LICENSE, README, AGENTS.md (with all the agent symlinks), CONTRIBUTING/COC/SECURITY, CI workflows, release/pages pipelines, docs, examples, website skeleton, language manifest, Makefile, and a starter `.claude/` skill — and creates the GitHub remote.
- **The spec is the source of truth.** Every file is derived from [`OSS_SPEC.md`](OSS_SPEC.md). `oss-spec validate` will tell you exactly which §19 items an existing repo is missing.
- **AI is a feature, not a dependency.** With `--no-ai` you get a deterministic skeleton; with `oss-spec init` `zag` interprets a freeform prompt into a structured manifest. Either way the bootstrap engine is the same.
- **Agent-friendly out of the box.** The generated repo includes the OSS_SPEC.md §12 CLI discoverability contract: `--help-agent`, `--debug-agent`, `commands`, `docs`, and `man` are all wired up so coding agents can self-serve.
- **Built on the same conventions it ships.** oss-spec is its own first customer — `oss-spec validate .` against this very repo passes.

## Prerequisites

- Rust 1.85+ (edition 2024)
- `git` (skip with `--no-git`)
- `gh` GitHub CLI (skip with `--no-gh`)
- A configured `zag` provider for the AI flow (skip with `--no-ai`)

## Install

```sh
cargo install oss-spec
```

## Quick start

```sh
oss-spec init "create a python cli for finding stock buys"
```

That sends the prompt to `zag`, shows the proposed manifest, and on confirmation writes a complete repo to disk and creates the GitHub remote.

For a deterministic, offline run:

```sh
oss-spec init --name my-tool --lang rust --kind cli --license MIT --no-ai --yes
```

To validate an existing repo against [`OSS_SPEC.md`](OSS_SPEC.md):

```sh
oss-spec validate --path .
```

Or point `validate` at any git URL to clone + validate in one step:

```sh
oss-spec validate --url https://github.com/niclaslindstedt/oss-spec.git
```

To auto-fix all findings in one pass:

```sh
oss-spec validate --fix
```

Add `--create-issues` to open one GitHub issue per violation on the source
repo (works with both `validate` and `fix`):

```sh
oss-spec validate --url https://github.com/foo/bar.git --create-issues --yes
```

## Usage

| Command | What it does |
|---|---|
| `oss-spec init [<PROMPT>] [--name <NAME>]` | Bootstrap into the current directory, or a new `--name` subdirectory. With a prompt, zag interprets it into a manifest. |
| `oss-spec validate [--path .] [--url URL] [--no-ai] [--fix] [--create-issues]` | Validate a local or remote repo against `OSS_SPEC.md` §19; includes AI quality review by default. |
| `oss-spec fix [--path .] [--url URL] [--create-issues] [--yes] [--no-ai]` | Fix §19 violations in place, or file one GitHub issue per violation cluster. |
| `oss-spec fetch [--into DIR]` | Clone the public oss-spec repo so a coding agent can browse the spec, templates, and the dogfood implementation locally. |
| `oss-spec commands [<NAME>] [--examples]` | Stable, machine-readable command index (§12.4). |
| `oss-spec docs [<TOPIC>]` | Print an embedded `docs/` topic (§12.3). |
| `oss-spec man [<COMMAND>]` | Print an embedded manpage (§12.3). |
| `oss-spec --help-agent` | Plain-text dump for prompt injection (§12.1). |
| `oss-spec --debug-agent` | Plain-text troubleshooting context (§12.2). |

Run `oss-spec --help` for the full flag list.

## Configuration

oss-spec has no configuration file. See [`docs/configuration.md`](docs/configuration.md) for the precedence rules between flags, AI-interpreted prompts, and environment-derived defaults (`git config`, `gh api user`).

## Examples

See [`examples/bootstrap-rust-cli/`](examples/bootstrap-rust-cli/) for a runnable end-to-end demo.

## Documentation

- [Getting started](docs/getting-started.md)
- [Configuration](docs/configuration.md)
- [Architecture](docs/architecture.md)
- [Troubleshooting](docs/troubleshooting.md)
- [`OSS_SPEC.md`](OSS_SPEC.md) — the spec this tool implements

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under [MIT](LICENSE).
