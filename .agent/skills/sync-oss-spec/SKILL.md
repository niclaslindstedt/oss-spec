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

2. Check whether `OSS_SPEC.md` or `src/validate/` changed since the baseline — those are the two inputs that can invalidate previously-passing conformance:

   ```sh
   git log --oneline "$BASELINE"..HEAD -- OSS_SPEC.md src/validate/
   git diff --name-only "$BASELINE"..HEAD
   ```

3. Run the validator against the repo itself. This is the source of truth for what is currently out of spec:

   ```sh
   cargo run -q -- validate --no-ai   # structural violations only
   cargo run -q -- validate            # also runs AI quality review (defaults to --path .)
   ```

   Each structural violation names the spec section (e.g. `§7.1`, `§10.3`, `§21.5`) and the file or directory at fault. AI findings name the file, the section, a severity, and a suggestion.

   **Nonbinary fallback.** If the `oss-spec` binary and `cargo` are both unavailable in the current environment (sandboxed agent sessions, ephemeral CI without a Rust toolchain, freshly-cloned checkouts where `cargo build` would be too slow), use the language-agnostic bash mirror at `scripts/validate.sh` instead. It implements the same deterministic §19 checks as `src/validate/` and prints the AI quality checklist as a manual prompt at the end:

   ```sh
   ./scripts/validate.sh .                                                                   # local copy
   curl -fsSL https://raw.githubusercontent.com/niclaslindstedt/oss-spec/main/scripts/validate.sh | bash -s -- .   # no checkout needed
   ```

   The two implementations are kept in lockstep by hand (there is no automated drift detector), so prefer the Rust validator whenever it is available — fall back to the bash script only when it is not. If you find a discrepancy between the two during a sync run, fix it in the same PR per the "Validate-script parity" rule in `AGENTS.md`.

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
| §10.3 floating or under-pinned toolchain | Edit the workflow to pin at or above the spec minimums in `MIN_TOOLCHAIN_VERSIONS` (`src/validate/toolchain.rs`) |
| §11.1 missing `docs/` content | Create the topic file, then run `update-docs` |
| §11.2 website drift | Run `make website` and inspect `website/src/generated/`; follow up with `update-website` |
| §13.5 `prompts/<name>/` has no versioned file | Add `prompts/<name>/1_0_0.md` with the required YAML front matter (`name`, `description`, `version: 1.0.0`) and `## System` / `## User` sections |
| §15 missing issue / PR templates | Create the templates under `.github/ISSUE_TEMPLATE/` or `.github/PULL_REQUEST_TEMPLATE.md` |
| §19 raw print statement outside `src/output.rs` | Route the call through `output::status` / `output::info` / `output::warn` / `output::error` |
| §20 inline `#[cfg(test)] mod { … }` block in `src/` | Move the tests to `tests/<module>_test.rs` and replace with `#[cfg(test)] mod <name>_test;` or delete the gate |
| §20.2 test file stem does not end with `_test(s)` / `Test(s)` | Rename the file so the stem matches the regex `_?[Tt]ests?$` |
| §20.5 source file exceeds 1000 lines | **Preferred:** split the file by concern into sibling modules / helpers (see `src/validate/` for the canonical example — one submodule per coherent group of checks). **Common easy case:** if the file also has a §20 inline-test violation, extracting the test block to `tests/<stem>_test.<ext>` usually resolves both at once. **Escape hatch:** if the size is genuinely justified (generated code, cohesive state machine, third-party snapshot), add `oss-spec:allow-large-file: <reason>` in any comment within the file's first 20 lines — the reason must be non-empty. |
| §21.2 `.claude/skills` is not a symlink | Replace it with `ln -s ../.agent/skills .claude/skills` |
| §21.3 SKILL.md missing front matter fields | Add `name:` / `description:` to the front matter |
| §21.4 missing `.last-updated` | Touch the file and record the current `HEAD`: `git rev-parse HEAD > .agent/skills/<skill>/.last-updated` |
| §21.5 missing required `update-*` skill | Create `.agent/skills/<skill>/SKILL.md` (+ `.last-updated`); register it in `maintenance/SKILL.md` |
| §21.6 `maintenance` skill registry row missing | Add the row in `maintenance/SKILL.md`, alphabetical, with a run-order slot |

## Update checklist

- [ ] Read the baseline from `.last-updated` and diff `OSS_SPEC.md` / `src/validate/`
- [ ] Run `cargo run -q -- validate --no-ai` and record every structural violation (or `./scripts/validate.sh .` if the binary/cargo is unavailable — see the Nonbinary fallback note above)
- [ ] Run `cargo run -q -- validate .` and record every AI finding worth acting on (the bash fallback emits the same checklist as a manual prompt at the end of its run)
- [ ] Walk the mapping table and fix each violation at its source
- [ ] If a fix requires a propagation step (e.g. new mandate in the spec), hand off to `update-spec` first, then re-run this skill
- [ ] Re-run the validator — it must report zero structural violations (Rust path preferred; `./scripts/validate.sh .` if the binary/cargo is unavailable)
- [ ] Run `make fmt`, `make lint`, `make test` — the self-conformance test (`tests/self_conformance.rs`) must pass (skip these if the toolchain is unavailable and note the gap in the PR description so a follow-up run with cargo can confirm)
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/sync-oss-spec/.last-updated

## Verification

1. `cargo run -q -- validate --no-ai` prints `repo conforms to OSS_SPEC.md` (see `Report::print`). If cargo is unavailable, `./scripts/validate.sh .` exits 0 instead.
2. `make test` passes, including the self-conformance test. Skip only if the toolchain is genuinely unavailable; in that case call out the unverified gap in the PR description.
3. Every violation present before this run has a matching edit in the diff — no violations were silenced by loosening the validator (or its bash mirror).
4. `.last-updated` was rewritten with the current `HEAD`.

## Skill self-improvement

After a run, extend this file:

1. **Grow the mapping table** whenever a new §X.Y section starts producing violations that the table does not yet cover.
2. **Record fix recipes** (exact commands or edit patterns) for violations that required more than a one-line change.
3. **Flag recurring drift** — if the same violation keeps coming back, either a CI check or a different skill's mapping table is missing a row. Fix the upstream cause, not just the symptom.
4. **Commit the skill edit** alongside the repo fixes so the knowledge compounds.
