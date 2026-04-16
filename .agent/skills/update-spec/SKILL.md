---
name: update-spec
description: "Use when OSS_SPEC.md has been edited. Bumps the spec version field, propagates the new mandate through validate.rs, tests, templates, docs, README, AGENTS.md, and the self-conformance test."
---

# Updating OSS_SPEC.md

`OSS_SPEC.md` is the single source of truth for every mandate this CLI emits into bootstrapped projects and enforces in `oss-spec validate`. Because the spec is embedded into generated projects via the symlink `templates/_common/OSS_SPEC.md -> ../../OSS_SPEC.md`, every edit ripples outward. This skill exists to ensure the ripples land.

## Tracking mechanism

`.agent/skills/update-spec/.last-updated` contains the git commit hash from the last successful run.

## Discovery process

1. Read the baseline:

   ```sh
   BASELINE=$(cat .agent/skills/update-spec/.last-updated)
   ```

2. Check whether `OSS_SPEC.md` changed since the baseline:

   ```sh
   git diff --name-only "$BASELINE"..HEAD -- OSS_SPEC.md
   ```

3. Read the diff itself:

   ```sh
   git diff "$BASELINE"..HEAD -- OSS_SPEC.md
   ```

   Decide whether the change is:
   - **Major** — a breaking change to an existing mandate or a removed section.
   - **Minor** — a new section, a new mandate, or a new required file.
   - **Patch** — wording clarifications, typo fixes, examples.

## Mapping table

| What changed in OSS_SPEC.md | What else must change |
|---|---|
| Version field in YAML front matter | bump per semver — major / minor / patch |
| New required root file | `src/validate.rs` (`required_files`), `templates/_common/<file>`, `README.md`, tests |
| New required directory | `src/validate.rs` (`required_dirs`), `templates/_common/<dir>/`, tests |
| New required symlink | `src/validate.rs` (`symlinks`), `src/bootstrap.rs::create_agents_symlinks` (if rooted at AGENTS.md), tests |
| New §19 content rule | `src/validate.rs` (new validator fn), `tests/validate_test.rs`, `src/fix.rs` if auto-fixable |
| New required workflow | `src/validate.rs` (`required_workflows`), `templates/_common/.github/workflows/<file>.tmpl`, tests |
| New required agent skill | `src/validate.rs::check_agent_skills`, `templates/_common/.agent/skills/<name>/`, tests |
| Spec-wide guidance touching AGENTS.md | `templates/_common/AGENTS.md.tmpl`, this repo's `AGENTS.md` |
| README-visible change | `README.md` (run `update-readme` afterwards) |
| Docs-visible change | `docs/` (run `update-docs` afterwards) |

## Update checklist

- [ ] Bump the `version:` field in the YAML front matter (major/minor/patch)
- [ ] Walk the mapping table — update every affected file
- [ ] Update `CHANGELOG.md`? No — it is generated at release time
- [ ] Run `make fmt`, `make lint`, `make test`
- [ ] Run `oss-spec validate .` — the self-conformance test must still pass
- [ ] Run `update-readme` and `update-docs` skills if they are now stale
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/update-spec/.last-updated

## Verification

1. `cargo test -p oss-spec` (the full suite)
2. `oss-spec validate .` against a freshly bootstrapped demo repo
3. Confirm the new spec version renders in `README.md` and (if applicable) the website
4. Confirm `.last-updated` was rewritten

## Skill self-improvement

1. **Grow the mapping table** with every new propagation path you discover.
2. **Record any cross-cutting invariant** (e.g. "adding X to §A also requires Y in §B").
3. **Commit the skill edit** alongside the spec change so the propagation knowledge is captured.
