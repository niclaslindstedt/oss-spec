# oss-spec

> Bootstrap new open source repositories that conform to OSS_SPEC.md.

## Synopsis

```
oss-spec <PROMPT>
oss-spec new <NAME> [-d <DESCRIPTION>] [flags]
oss-spec init [-d <DESCRIPTION>] [flags]
oss-spec check [--path .] [--url <URL>] [--create-issues]
oss-spec fix   [--path .] [--url <URL>] [--create-issues] [--max-turns N]
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

The default invocation takes a freeform prompt, sends it to the `zag` library
for interpretation, shows the proposed manifest, and on confirmation writes
the project to disk.

## Subcommands

| Command | Description |
|---|---|
| (default) | Interpret a prompt via zag and bootstrap. |
| `new` | Explicit bootstrap with flags only. |
| `init` | Bootstrap into the current directory. |
| `check` | Validate a local or remote repo against OSS_SPEC.md §19. |
| `fix` | Fix §19 violations in place, or file one GitHub issue per violation. |
| `fetch` | Clone the public oss-spec repo for local reference. |
| `commands` | Stable, machine-readable command index. |
| `docs` | Print an embedded `docs/` topic. |
| `man` | Print an embedded manpage. |

## Global flags

| Flag | Type | Default | Description |
|---|---|---|---|
| `--debug` | bool | false | Show debug-level output on stderr. |
| `--no-ai` | bool | false | Skip zag/AI calls. |
| `--no-git` | bool | false | Skip `git init` and the first commit. |
| `--no-gh` | bool | false | Skip `gh repo create`. |
| `-y, --yes` | bool | false | Accept defaults; non-interactive. |
| `--path <DIR>` | path | cwd | Parent directory for the new repo. |
| `--name <NAME>` | string | — | Override project name. |
| `--lang <L>` | enum | rust | rust\|python\|node\|go\|generic |
| `--kind <K>` | enum | cli | lib\|cli\|service |
| `--license <L>` | enum | MIT | MIT\|Apache-2.0\|MPL-2.0 |
| `--visibility <V>` | enum | public | public\|private |
| `--help-agent` | bool | false | Print plain-text agent help dump. |
| `--debug-agent` | bool | false | Print plain-text troubleshooting dump. |

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
| 1 | Bootstrap or check failure |
| 2 | Usage error / unknown command |

## Examples

```sh
oss-spec "create a python cli for finding stock buys"
oss-spec new demo --lang rust --kind cli --license MIT --no-ai --yes
cd demo && oss-spec check
oss-spec check --url https://github.com/niclaslindstedt/oss-spec.git
oss-spec check --url https://github.com/foo/bar.git --create-issues --yes
oss-spec fix   --url https://github.com/foo/bar.git --create-issues --yes
oss-spec commands --examples
```

## `check` / `fix` flags

| Flag | Applies to | Description |
|---|---|---|
| `--path <DIR>` | check, fix | Local repo to validate / fix. Defaults to `.`. |
| `--url <URL>` | check, fix | Clone a remote git repo into a temp dir and run against the clone. The clone is removed on exit. Mutually exclusive with `--path`. |
| `--shallow` | check, fix | Use `git clone --depth 1` when `--url` is given. Defaults to `true`. |
| `--create-issues` | check, fix | After reporting, file one GitHub issue per violation via `gh`. On `fix`, required whenever `--url` is used. |
| `--max-turns <N>` | check, fix | Iteration budget for the issue-filing / fix agent. Default 30. |

## See also

- `oss-spec --help-agent`
- `oss-spec docs getting-started`
- [OSS_SPEC.md](../OSS_SPEC.md)
