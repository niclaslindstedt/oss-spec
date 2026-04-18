---
name: update-prompts
description: "Use when prompts under prompts/ may be stale. Discovers changes to OSS_SPEC.md, the validator, manifest enums, and the ai/fix glue since the last run, and rewrites the affected prompt templates so they stay aligned with their sources of truth."
---

# Updating the LLM prompts

Every LLM-driven step in this CLI is defined by a versioned prompt under `prompts/<name>/<major>_<minor>_<patch>.md` with a required YAML front matter block (`name`, `description`, `version`) — see §13.5 of `OSS_SPEC.md`. Prompt files are **immutable once committed**; every change lands as a new version. Prompts drift whenever:

- `OSS_SPEC.md` changes — `verify-conformance` embeds the full spec and `fix-conformance` references section numbers, so any spec edit may invalidate them.
- `src/validate.rs` grows a new structural rule — the fix agent needs new guidance to handle the new violation shape.
- `src/ai.rs` or `src/fix.rs` change the rendering context — new Jinja placeholders may appear or disappear.
- `src/manifest.rs` gains or removes a `Language` / `Kind` / `License` enum variant — the `interpret-prompt` JSON schema must track it.

This skill exists so that drift between prompt text and the rest of the codebase is caught by the same `maintenance` sweep that keeps `README.md`, `docs/`, `man/`, and `website/` in sync.

## Tracking mechanism

`.agent/skills/update-prompts/.last-updated` contains the git commit hash from the last successful run. Empty means "never run" — fall back to the initial commit.

## Discovery process

1. Read the baseline:

   ```sh
   BASELINE=$(cat .agent/skills/update-prompts/.last-updated)
   ```

2. Enumerate every prompt file and note its current version:

   ```sh
   find prompts -name '[0-9]*_[0-9]*.md' | sort
   ```

3. Diff the watched paths against the baseline:

   ```sh
   git diff --name-only "$BASELINE"..HEAD -- \
       OSS_SPEC.md src/ai.rs src/fix.rs src/validate.rs src/manifest.rs prompts/
   ```

4. For each path that appears in the diff, walk the mapping table below and decide which prompts are now stale.

## Mapping table

| Source-of-truth change | Prompt(s) to audit | What to check |
|---|---|---|
| `OSS_SPEC.md` body (any `§N` edit) | `prompts/verify-conformance/*.md` | Embedded checklist — make sure every new or changed mandate is represented. |
| `OSS_SPEC.md` body (any `§N` edit) | `prompts/fix-conformance/*.md` | Per-section guidance block — add or amend bullets so the agent knows how to fix the new rule. |
| `OSS_SPEC.md` version field | all prompts that render `{{ spec_version }}` | Nothing to edit — version is pulled from `embedded::oss_spec_version()` at render time. Verify the placeholder is still referenced. |
| New check in `src/validate.rs` (new `Violation` producer) | `prompts/fix-conformance/*.md` | Add handling guidance; if the check is AI-only, make sure it is mentioned as a quality finding category. |
| New placeholder added in `src/ai.rs` `context! { ... }` | the matching prompt's `## User` section | Reference the new placeholder; re-render to confirm no leftover `{{ unused }}` tokens. |
| New `Language` / `Kind` / `License` variant in `src/manifest.rs` | `prompts/interpret-prompt/*.md` | Update the JSON schema `enum` list embedded in the prompt. |
| New versioned prompt added under `prompts/<name>/<major>_<minor>_<patch>.md` | `src/ai.rs` / `src/fix.rs` callers | Confirm the caller loads by name (not a pinned version) so the new file is auto-picked; if a caller pins a specific version, bump it. |

## Update checklist

- [ ] Read the baseline from `.last-updated`
- [ ] Run the `git diff --name-only` above; bail out if nothing watched changed
- [ ] **Never edit an existing `<major>_<minor>_<patch>.md` file** (§13.5). Every change — typo to rewrite — lands as a new file. Decide the semver bump:
    - **patch** (`1_0_0` → `1_0_1`): wording / typo / clarification, contract unchanged
    - **minor** (`1_0_0` → `1_1_0`): new placeholder, expanded scope, new guidance
    - **major** (`1_0_0` → `2_0_0`): breaking rewrite, removed placeholder, changed JSON schema
- [ ] Copy the latest version to the new filename, then edit; keep every prior version on disk for diff/bisect
- [ ] Update the YAML front matter in the new file: `version: <major>.<minor>.<patch>` must match the stem
- [ ] Update any callers in `src/ai.rs` or `src/fix.rs` that pin a specific prompt version
- [ ] Re-run `cargo test` — `prompts_test.rs` must still load every prompt
- [ ] Run `make fmt`, `make lint`, `make test`
- [ ] Run `oss-spec validate .` — the self-conformance check exercises §13.5 prompt layout
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/update-prompts/.last-updated

## Verification

1. Every prompt that references a `{{ placeholder }}` must have a matching key in its caller's `context! { ... }` in `src/ai.rs`.
2. Every placeholder a caller passes must be referenced at least once by the rendered prompt.
3. `cargo test` passes — this covers prompt loading, rendering, and the self-conformance test.
4. `.last-updated` has been rewritten with the current `HEAD`.

## Skill self-improvement

After a run, edit this file in place:

1. **Grow the mapping table** with any new source → prompt path you discovered.
2. **Record drift signals** — if a prompt went stale through a path not captured above, add the path.
3. **Commit the skill edit** together with the prompt edits so the knowledge compounds.
