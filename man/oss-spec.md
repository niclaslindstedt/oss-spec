# oss-spec

> Bootstrap new open source repositories that conform to OSS_SPEC.md.

## Synopsis

```
oss-spec init [<PROMPT>] [-d <DESCRIPTION>] [--name <NAME>] [flags]
oss-spec new <NAME> [-d <DESCRIPTION>] [flags]
oss-spec validate [--path .] [--url <URL>] [--no-ai] [--fix] [--create-issues]
oss-spec fix   [--path .] [--url <URL>] [--create-issues] [--max-turns N] [--yes] [--no-ai]
oss-spec fetch [--into <DIR>] [--url <URL>] [--shallow]
oss-spec commands [<NAME>] [--examples]
oss-spec docs [<TOPIC>]
oss-spec man [<COMMAND>]
```

## Description

`oss-spec` materializes a complete open source repo (LICENSE, README,
AGENTS.md + symlinks, CI workflows, docs, examples, website, language
manifests, Makefile, .claude skills, etc.) following the conventions in
`OSS_SPEC.md`.

Use `oss-spec init` to bootstrap a project — optionally with a freeform
prompt that the `zag` library interprets into a structured manifest.

## Subcommands

| Command | Description |
|---|---|
| `init` | Bootstrap into the current directory (with optional AI prompt). |
| `new` | Explicit bootstrap with flags only. |
| `validate` | Validate a local or remote repo against OSS_SPEC.md §19. |
| `fix` | Fix §19 violations in place, or file one GitHub issue per violation. |
| `fetch` | Clone the public oss-spec repo for local reference. |
| `commands` | Stable, machine-readable command index. |
| `docs` | Print an embedded `docs/` topic. |
| `man` | Print an embedded manpage. |

## Global flags

| Flag | Type | Default | Description |
|---|---|---|---|
| `--debug` | bool | false | Show debug-level output on stderr. |
| `--help-agent` | bool | false | Print plain-text agent help dump (§12.1). |
| `--debug-agent` | bool | false | Print plain-text troubleshooting dump (§12.2). |

## Bootstrap flags (`init`, `new`)

These flags are shared by the `init` and `new` subcommands via `BootstrapOpts`:

| Flag | Type | Default | Description |
|---|---|---|---|
| `--no-ai` | bool | false | Skip zag/AI calls. Deterministic skeleton only. |
| `--no-git` | bool | false | Skip `git init` and the first commit. |
| `--no-gh` | bool | false | Skip `gh repo create`. |
| `-y, --yes` | bool | false | Accept defaults; non-interactive. |
| `--path <DIR>` | path | cwd | Parent directory for the new repo. |
| `--lang <L>` | enum | rust | rust\|python\|node\|go\|generic |
| `--kind <K>` | enum | cli | lib\|cli\|service |
| `--license <L>` | enum | MIT | MIT\|Apache-2.0\|MPL-2.0 |
| `--visibility <V>` | enum | public | public\|private |

Additionally, `init` accepts:

| Flag | Type | Default | Description |
|---|---|---|---|
| `<PROMPT>` | positional | — | Freeform prompt interpreted by zag into a manifest. |
| `-d, --description` | string | — | Project description (used when no prompt is given). |
| `--name <NAME>` | string | dir name | Override the project name. |

And `new` accepts:

| Flag | Type | Default | Description |
|---|---|---|---|
| `<NAME>` | positional | — | Project name (kebab-case). Required. |
| `-d, --description` | string | — | Description; if omitted you will be prompted (or AI will infer). |

## `validate` flags

| Flag | Type | Default | Description |
|---|---|---|---|
| `--path <DIR>` | path | `.` | Local repo to validate. |
| `--url <URL>` | string | — | Clone a remote repo into a temp dir first. Mutually exclusive with `--path`. |
| `--shallow` | bool | true | Use `git clone --depth 1` when `--url` is given. |
| `--create-issues` | bool | false | After reporting, file one GitHub issue per violation via `gh`. |
| `--max-turns <N>` | u32 | 30 | Iteration budget for the issue-filing agent (with `--create-issues`). |
| `--no-ai` | bool | false | Skip AI quality review. Deterministic structural checks only. |
| `--fix` | bool | false | After AI verification, automatically fix all findings via a zag agent. |

## `fix` flags

| Flag | Type | Default | Description |
|---|---|---|---|
| `--path <DIR>` | path | `.` | Repo to fix. |
| `--url <URL>` | string | — | Fix a remote repo by cloning first. Requires `--create-issues`. |
| `--shallow` | bool | true | Use `git clone --depth 1` when `--url` is given. |
| `--create-issues` | bool | false | File one GitHub issue per violation instead of editing in place. |
| `--max-turns <N>` | u32 | 30 | Cap the agent's iteration budget. |
| `-y, --yes` | bool | false | Skip the confirmation prompt. |
| `--no-ai` | bool | false | Skip AI calls. |

## Environment variables

| Variable | Description |
|---|---|
| `ZAG_PROVIDER` | LLM provider override (passed through to `zag`). |
| `ZAG_MODEL` | Model size/name override. |
| `NO_COLOR` | Disable ANSI color in output. |

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Bootstrap or validation failure |
| 2 | Usage error / unknown command |

## Examples

```sh
oss-spec init "create a python cli for finding stock buys"
oss-spec new demo --lang rust --kind cli --license MIT --no-ai --yes
cd demo && oss-spec validate
oss-spec validate --url https://github.com/niclaslindstedt/oss-spec.git
oss-spec validate --url https://github.com/foo/bar.git --create-issues --yes
oss-spec validate --fix
oss-spec fix   --url https://github.com/foo/bar.git --create-issues --yes
oss-spec commands --examples
```

## See also

- `oss-spec --help-agent`
- `oss-spec docs getting-started`
- [OSS_SPEC.md](../OSS_SPEC.md)
