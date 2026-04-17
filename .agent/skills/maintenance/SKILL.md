---
name: maintenance
description: "Use when you want to bring every drift-prone artifact in the repo back into sync. Dispatches to all individual update-* skills in the correct order, aggregates their results, and leaves a single combined PR ready to review."
---

# Maintenance

This is the umbrella skill. It does no rewriting itself — it decides which sync skills are stale, runs each one, and reports a combined summary. Use it when you do not know which specific artifact is out of date, or when several have likely drifted at once (for example, after a large merge).

## When to run

- After a big merge from `main` when you are not sure which surfaces moved.
- On a cadence (weekly / before a release) as a "drift sweep".
- When CI flags a staleness check but the agent is uncertain which skill to run.

Do **not** use this skill for a targeted fix — if you know the README is the only stale thing, call `update-readme` directly.

## Registry

The registry is the single source of truth for which sync skills exist in this repo. Keep it in alphabetical order; every `update-*` directory under `.agent/skills/` must appear here exactly once.

| Skill | Fixes | Run order |
|---|---|---|
| `update-spec`     | `OSS_SPEC.md` propagation into `check.rs`, templates, tests, docs | 1 — run first so downstream skills see the new mandate |
| `update-manpages` | `man/oss-spec.md` vs. `src/cli.rs` clap definitions                | 2 |
| `update-docs`     | `docs/*.md` vs. source of truth                                    | 3 |
| `update-readme`   | `README.md` vs. CLI, spec, and supported languages                 | 4 |
| `update-website`  | `website/` vs. `README.md`, `docs/`, and `OSS_SPEC.md`             | 5 |
| `sync-oss-spec`   | Repo contents vs. `OSS_SPEC.md` (runs `oss-spec validate .`)       | 6 — run last, it verifies that every upstream fix landed correctly |

Run order matters: upstream fixes must land before downstream skills read them. A skill that depends on README text (for example, `update-website`) must run *after* `update-readme`.

## Discovery process

For each skill in the registry, decide whether it needs to run:

1. Read the skill's `.last-updated` file:

   ```sh
   BASELINE=$(cat .agent/skills/<skill>/.last-updated)
   ```

   An empty or missing file means "never run" — schedule it.

2. Diff the watched paths for that skill against the baseline:

   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

   If any file in the skill's mapping table appears in the diff, schedule the skill.

3. Build the list of skills to run. Preserve the run order from the registry.

## Execution

For each scheduled skill, in order:

1. Load `.agent/skills/<skill>/SKILL.md`.
2. Follow its discovery process, mapping table, and update checklist exactly.
3. Verify the skill's own verification section passes.
4. Record the `git rev-parse HEAD` value the skill wrote to its `.last-updated`.

Between skills, do **not** commit — aggregate all edits into a single working tree so the final commit covers the whole sync sweep.

## Update checklist

- [ ] Read every skill's `.last-updated` and build the schedule
- [ ] Run each scheduled skill in the order above
- [ ] After all skills finish, run:
    - [ ] `make fmt`
    - [ ] `make lint`
    - [ ] `make test`
    - [ ] `oss-spec validate .`
- [ ] Stage every touched file (including each updated `.last-updated`)
- [ ] Commit with a conventional-commit message:

      docs: drift sweep — sync readme, docs, manpages, website

  Scopes may be comma-separated if multiple artifacts moved.

- [ ] Update `.agent/skills/maintenance/.last-updated`:

      git rev-parse HEAD > .agent/skills/maintenance/.last-updated

- [ ] Hand off to the `commit` skill to push and open / update the PR

## Verification

1. Every scheduled skill's verification section must pass.
2. `oss-spec validate .` must report zero violations.
3. The final diff should touch only documentation files, skill `.last-updated` files, and (rarely) small code adjustments that the skills flagged.
4. Every skill that ran must have its `.last-updated` rewritten with the same commit hash — this is what tells the next run of `maintenance` that the sweep completed.

## Skill self-improvement

After every run, update this file:

1. **Add new sync skills to the registry.** Every new `update-*` skill must appear in the registry table, in alphabetical order, with a clear "run order" slot.
2. **Adjust run order** if you discovered a hidden dependency (e.g. skill A reads files that skill B rewrites).
3. **Record drift signals.** If you found a change that should have triggered a skill but did not appear in any skill's mapping table, extend the offending skill's mapping table — not this one.
4. **Commit the skill edits** together with the drift sweep so the orchestration knowledge compounds.
