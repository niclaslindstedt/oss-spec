---
name: update-docs
description: "Use when files under docs/ may be stale. Discovers commits since the last docs update, maps changed source files to affected conceptual documentation, and brings docs/*.md back into sync with the implementation."
---

# Updating the Docs

The `docs/` directory contains conceptual documentation for oss-spec. Unlike the man page (command-level reference) or the README (overview), docs/ files explain *why* and *how* in depth. They go stale when §19 rules, CLI flags, supported languages, or spec clauses change without a matching docs edit.

## Tracking mechanism

`.agent/skills/update-docs/.last-updated` holds the git commit hash from the last time this skill ran successfully. Use it as the baseline for diffing.

## Discovery process

1. Read the baseline:

   ```sh
   BASELINE=$(cat .agent/skills/update-docs/.last-updated)
   ```

   Empty file → use `git rev-list --max-parents=0 HEAD`.

2. List commits since the baseline:

   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. List changed files:

   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the mapping table below.

5. Read each affected doc before editing so unrelated sections are preserved.

## Mapping table

| Changed files / scope | Doc(s) to update |
|---|---|
| `src/cli.rs` (new/renamed subcommand or flag) | `docs/getting-started.md`, `docs/architecture.md` if structural |
| `src/check.rs` (new §19 rule) | `docs/architecture.md` (what check does), `docs/troubleshooting.md` (new violations) |
| `src/manifest.rs::Language` variant added | `docs/getting-started.md` (the list of languages), `docs/architecture.md` (overlay structure) |
| `src/bootstrap.rs` (new rendering step) | `docs/architecture.md` |
| `templates/` tree reshaped | `docs/architecture.md` |
| `OSS_SPEC.md` new section | `docs/getting-started.md` pointer, plus a mention in `docs/architecture.md` if structural |

## Update checklist

- [ ] Read baseline from `.last-updated` and run `git log` / `git diff --name-only`
- [ ] Read the current `docs/*.md` files that were flagged
- [ ] Update each affected doc in place
- [ ] Verify every shell snippet still reflects current CLI syntax
- [ ] Run `oss-spec validate .` to confirm the repo still conforms
- [ ] Run `make test` — the self-conformance test must still pass
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/update-docs/.last-updated

## Verification

1. Read every edited doc section and verify it matches the corresponding source file.
2. Confirm cross-links between docs still resolve.
3. Confirm the `.last-updated` file was updated.

## Skill self-improvement

After a run, extend this file:

1. **Grow the mapping table** with any new source → doc relationship you discovered.
2. **Add patterns** you had to invent for recurring doc updates.
3. **Commit skill edits** together with the docs changes so the knowledge compounds.
