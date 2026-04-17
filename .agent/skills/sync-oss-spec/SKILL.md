---
name: sync-oss-spec
description: "Use when the repository may have drifted out of conformance with OSS_SPEC.md. Runs `oss-spec validate .`, walks the violations, and fixes each one so the self-conformance test keeps passing."
---

# Syncing the repo with OSS_SPEC.md

`OSS_SPEC.md` is the specification this repo claims to conform to, and `oss-spec validate .` is the machine-readable enforcement of that claim. Because this project is the reference implementation of the spec (see the "Reference implementation" section of `AGENTS.md`), every new mandate in the spec and every new file/directory/symlink requirement must be reflected on disk here too. This skill is the playbook for closing that gap: it runs the validator, inspects each violation, and brings the repo back into conformance.

This skill is complementary to `update-spec`. `update-spec` propagates a spec edit into code (`validate.rs`, templates, tests, docs). `sync-oss-spec` goes the other direction: it treats the current spec as the target and fixes the repository contents (missing files, broken symlinks, stale workflows, non-conformant templates) until `oss-spec validate .` reports zero violations and the AI quality findings are addressed.

## Tracking mechanism

`.agent/skills/sync-oss-spec/.last-updated` contains the git commit hash of the last successful run. Empty means "never run" — use the repo's initial commit (`git rev-list --max-parents=0 HEAD`) as the baseline.

## Discovery process

1. Read the baseline:

   ```sh
   BASELINE=$(cat .agent/skills/sync-oss-spec/.last-updated)
   ```

2. Check whether `OSS_SPEC.md` or `src/validate.rs` changed since the baseline — those are the two inputs that can invalidate previously-passing conformance:

   ```sh
   git log --oneline "$BASELINE"..HEAD -- OSS_SPEC.md src/validate.rs
   git diff --name-only "$BASELINE"..HEAD
   ```

3. Run the validator against the repo itself. This is the source of truth for what is currently out of spec:

   ```sh
   cargo run -q -- validate --no-ai   # structural violations only
   cargo run -q -- validate            # also runs AI quality review (defaults to --path .)
   ```

   Each structural violation names the spec section (e.g. `§7.1`, `§10.3`, `§21.5`) and the file or directory at fault. AI findings name the file, the section, a severity, and a suggestion.

4. For each violation, read the relevant section of `OSS_SPEC.md` so the fix matches the spec's intent rather than just silencing the check.

## Mapping table

| Violation spec section | Where to fix it |
|---|---|
| §2 missing `LICENSE` | Create `LICENSE` with the SPDX-identified license text and the correct copyright holder |
| §3 missing `README.md` sections | Edit `README.md`; run `update-readme` afterwards if extensive rewording is needed |
| §4/§5/§6 missing `CONTRIBUTING.md` / `CODE_OF_CONDUCT.md` / `SECURITY.md` | Create the file with the minimum content mandated by the corresponding spec section |
| §7.1 tool-specific guidance file is not a symlink | Replace the regular file with `ln -s AGENTS.md <path>` (or `ln -s ../AGENTS.md .github/copilot-instructions.md`) |
| §8.4 missing `CHANGELOG.md` | Create an empty Keep-a-Changelog-formatted file; do **not** hand-author entries |
| §9 Makefile target missing | Add the missing target to `Makefile` and verify it runs end-to-end |
| §10.1/§10.3/§10.4 missing workflow | Create `.github/workflows/<file>.yml`; cross-reference `templates/_common/.github/workflows/` for the canonical template |
| §10.3 floating or under-pinned toolchain | Edit the workflow to pin at or above the spec minimums in `MIN_TOOLCHAIN_VERSIONS` (`validate.rs`) |
| §11.1 missing `docs/` content | Create the topic file, then run `update-docs` |
| §11.2 website drift | Run `make website` and inspect `website/src/generated/`; follow up with `update-website` |
| §13.5 `prompts/<name>/` has no versioned file | Add `prompts/<name>/1_0.md` with the required `## System` / `## User` sections |
| §15 missing issue / PR templates | Create the templates under `.github/ISSUE_TEMPLATE/` or `.github/PULL_REQUEST_TEMPLATE.md` |
| §19 raw print statement outside `src/output.rs` | Route the call through `output::status` / `output::info` / `output::warn` / `output::error` |
| §20 inline `#[cfg(test)] mod { … }` block in `src/` | Move the tests to `tests/<module>_test.rs` and replace with `#[cfg(test)] mod <name>_test;` or delete the gate |
| §20.2 test file stem does not end with `_test(s)` / `Test(s)` | Rename the file so the stem matches the regex `_?[Tt]ests?$` |
| §21.2 `.claude/skills` is not a symlink | Replace it with `ln -s ../.agent/skills .claude/skills` |
| §21.3 SKILL.md missing front matter fields | Add `name:` / `description:` to the front matter |
| §21.4 missing `.last-updated` | Touch the file and record the current `HEAD`: `git rev-parse HEAD > .agent/skills/<skill>/.last-updated` |
| §21.5 missing required `update-*` skill | Create `.agent/skills/<skill>/SKILL.md` (+ `.last-updated`); register it in `maintenance/SKILL.md` |
| §21.6 `maintenance` skill registry row missing | Add the row in `maintenance/SKILL.md`, alphabetical, with a run-order slot |

## Update checklist

- [ ] Read the baseline from `.last-updated` and diff `OSS_SPEC.md` / `src/validate.rs`
- [ ] Run `cargo run -q -- validate --no-ai` and record every structural violation
- [ ] Run `cargo run -q -- validate .` and record every AI finding worth acting on
- [ ] Walk the mapping table and fix each violation at its source
- [ ] If a fix requires a propagation step (e.g. new mandate in the spec), hand off to `update-spec` first, then re-run this skill
- [ ] Re-run `cargo run -q -- validate --no-ai` — it must report zero structural violations
- [ ] Run `make fmt`, `make lint`, `make test` — the self-conformance test (`tests/self_conformance.rs`) must pass
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/sync-oss-spec/.last-updated

## Verification

1. `cargo run -q -- validate --no-ai` prints `repo conforms to OSS_SPEC.md` (see `Report::print`).
2. `make test` passes, including the self-conformance test.
3. Every violation present before this run has a matching edit in the diff — no violations were silenced by loosening the validator.
4. `.last-updated` was rewritten with the current `HEAD`.

## Skill self-improvement

After a run, extend this file:

1. **Grow the mapping table** whenever a new §X.Y section starts producing violations that the table does not yet cover.
2. **Record fix recipes** (exact commands or edit patterns) for violations that required more than a one-line change.
3. **Flag recurring drift** — if the same violation keeps coming back, either a CI check or a different skill's mapping table is missing a row. Fix the upstream cause, not just the symptom.
4. **Commit the skill edit** alongside the repo fixes so the knowledge compounds.
