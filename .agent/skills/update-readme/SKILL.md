---
name: update-readme
description: "Use when README.md may be stale. Discovers commits since the last README update, identifies what changed (CLI flags, subcommands, §19 rules, supported languages, spec version), and updates the affected README sections."
---

# Updating the README

`README.md` is the primary user-facing documentation for `oss-spec`. It covers installation, a quick-start example, the usage table, a summary of what `oss-spec check` enforces, and the spec version pointer. It gets stale whenever a CLI flag, subcommand, §19 rule, supported language, or the spec itself changes without a matching README edit.

## Tracking mechanism

The file `.agent/skills/update-readme/.last-updated` contains the git commit hash from the last time the README was comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery process

1. Read the baseline commit:

   ```sh
   BASELINE=$(cat .agent/skills/update-readme/.last-updated)
   ```

   An empty file means "never run" — use the repo's initial commit (`git rev-list --max-parents=0 HEAD`) as the baseline.

2. List commits since the baseline:

   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. List files changed since the baseline:

   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the mapping table below to decide which README sections need updating.

5. Read the current `README.md` so you can preserve voice, structure, and unrelated sections while editing.

## Mapping table

| Changed files / scope | README section(s) to update |
|---|---|
| `src/cli.rs` (new flag/subcommand) | **Usage** table; **Quick start** examples if the new flag is user-facing |
| `src/agent_help.rs` (`COMMANDS_TABLE`, `COMMAND_SPECS`, `EXAMPLES`) | **Usage** table (keep in sync with the generated help) |
| `src/check.rs` (new §19 rule) | **What `oss-spec check` enforces** summary list |
| `src/manifest.rs::Language` variant added | **Supported languages** list |
| `OSS_SPEC.md` front-matter `version:` bumped | The **spec version** line/badge in the README |
| `templates/<lang>/` new overlay | **Supported languages** list, examples that reference the language |
| `man/oss-spec.md` updated | Cross-check: any example in the README that mirrors the man page |

## Update checklist

- [ ] Read baseline from `.last-updated` and run `git log` / `git diff --name-only`
- [ ] Read the current `README.md`
- [ ] Update the **Usage** table if CLI flags or subcommands changed
- [ ] Update the **Quick start** if a user-facing default changed
- [ ] Update the **What `oss-spec check` enforces** list if a §19 rule was added or reworded
- [ ] Update the **Supported languages** list if a new language overlay landed
- [ ] Update the **spec version** pointer if `OSS_SPEC.md` was bumped
- [ ] Verify every shell snippet in the README is still syntactically valid against the current CLI
- [ ] Run `oss-spec check .` and make sure the repo still conforms
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/update-readme/.last-updated

## Verification

1. Read every edited section and confirm it matches the current source (`src/cli.rs`, `src/agent_help.rs`, `src/check.rs`, `OSS_SPEC.md`).
2. Copy each example command and run it in a scratch directory if it mutates state.
3. Confirm `.last-updated` was rewritten with the new `HEAD`.

## Skill self-improvement

After completing a run, improve this file:

1. **Expand the mapping table** if you discovered a new source-of-truth file that was not listed.
2. **Record new patterns** you had to invent (e.g. "README also mentions X, which lives in Y").
3. **Commit the skill edit** in the same PR as the README edit so the knowledge is preserved.
