---
name: update-manpages
description: "Use when files under man/ may be stale. Discovers commits since the last manpage update, maps changed CLI definitions to affected pages, and updates man/<cmd>.md to match the current clap definitions."
---

# Updating the Manpages

`man/` contains the reference-style command documentation shipped with `oss-spec` — one markdown file per command, embedded in the binary via `include_dir!` and read by `oss-spec man <command>`. These pages are the authoritative command-level reference and rot whenever a clap flag, subcommand, or default changes without a matching edit.

## Tracking mechanism

`.agent/skills/update-manpages/.last-updated` contains the git commit hash from the last successful run. Use it as the baseline for diffing.

## Discovery process

1. Read the baseline:

   ```sh
   BASELINE=$(cat .agent/skills/update-manpages/.last-updated)
   ```

   Empty → use `git rev-list --max-parents=0 HEAD`.

2. List commits since the baseline:

   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. List changed files:

   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Map the changes using the table below.

## Mapping table

| Changed files / scope | Manpage(s) to update |
|---|---|
| `src/cli.rs` (new or renamed subcommand) | `man/oss-spec.md` synopsis, commands list; create new `man/<cmd>.md` if absent |
| `src/cli.rs` (new flag on existing command) | The corresponding command section in `man/oss-spec.md` |
| `src/agent_help.rs::COMMAND_SPECS` | Cross-check examples in `man/oss-spec.md` |
| `src/validate.rs` (new §19 rule) | `man/oss-spec.md` section describing `oss-spec validate` |
| Language enum or overlay added | `man/oss-spec.md` section listing supported languages |

## Format conventions

Preserve these when editing:

- H1 title `# oss-spec <cmd>` matching what `oss-spec man <cmd>` prints.
- Standard sections: Synopsis → Description → Arguments → Flags → Examples → See Also.
- Synopsis and flag lines are 4-space indented code blocks (not fenced).
- See Also entries are `oss-spec man <cmd>` pointers with one-line descriptions.

## Update checklist

- [ ] Read baseline from `.last-updated` and run `git log` / `git diff --name-only`
- [ ] Read `src/cli.rs` for the current clap definitions
- [ ] Read every affected `man/*.md`
- [ ] Update command-level flags and examples
- [ ] Create `man/<cmd>.md` for any new subcommand
- [ ] Verify flag names, shorts, and value placeholders match `cli.rs` exactly
- [ ] Run `cargo build` so `include_dir!` picks up new files
- [ ] Run `make test` — manpage parity tests must still pass
- [ ] Run `oss-spec validate .`
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/update-manpages/.last-updated

## Verification

1. Build the binary and run `oss-spec man <cmd>` for every edited page; confirm it renders.
2. Compare every flag block against `src/cli.rs`.
3. Confirm `.last-updated` was rewritten.

## Skill self-improvement

After a run:

1. **Extend the mapping table** with any new flag group / command structure you discovered.
2. **Record format quirks** (e.g. alignment rules, See Also conventions) you had to normalize.
3. **Commit the skill edit** alongside the manpage edits.
