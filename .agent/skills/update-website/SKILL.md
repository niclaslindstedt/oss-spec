---
name: update-website
description: "Use when the marketing website may be stale. Discovers commits since the last website update, re-runs the source-extraction script, and updates website/ content so that generated pages match the current README, docs, and spec."
---

# Updating the Website

The `website/` directory contains the marketing site for `oss-spec`. Per §11.2 of `OSS_SPEC.md`, its source-derived content (hero copy, feature lists, version strings) must not be authored twice — it is extracted from `README.md`, `docs/`, and `OSS_SPEC.md` by a script, then rendered by the website build.

## Tracking mechanism

`.agent/skills/update-website/.last-updated` contains the git commit hash from the last successful run. Empty means "never run" — fall back to the initial commit.

## Discovery process

1. Read the baseline:

   ```sh
   BASELINE=$(cat .agent/skills/update-website/.last-updated)
   ```

2. Diff sources of truth against the baseline:

   ```sh
   git log --oneline "$BASELINE"..HEAD -- README.md docs/ OSS_SPEC.md
   git diff --name-only "$BASELINE"..HEAD -- README.md docs/ OSS_SPEC.md
   ```

3. If anything changed, re-run the source-extraction script:

   ```sh
   make website
   ```

4. Read the generated pages and confirm nothing committed under `website/src/generated/` disagrees with the current sources.

## Mapping table

| Changed file | Effect on website |
|---|---|
| `README.md` hero / quick start | Home page feature summary |
| `README.md` Usage table | CLI reference page |
| `docs/getting-started.md` | "Getting started" page |
| `docs/architecture.md` | Architecture page |
| `OSS_SPEC.md` front-matter `version:` | Version badge on the home page |

## Update checklist

- [ ] Read baseline and diff sources of truth
- [ ] Run `make website` to regenerate extracted content
- [ ] Review the diff under `website/src/generated/` and commit the changes
- [ ] Build the website locally (`make website-dev`) and smoke-test the home page
- [ ] Confirm the §11.2 staleness CI check would pass
- [ ] Run `oss-spec validate .`
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/update-website/.last-updated

## Verification

1. Open the rendered site locally and verify hero copy, version, and CLI table.
2. Run the website staleness CI check from §11.2 against HEAD.
3. Confirm `.last-updated` was rewritten.

## Skill self-improvement

1. **Expand the mapping table** if a new source file started feeding the website.
2. **Record extraction quirks** (e.g. "anchor X is parsed from heading Y").
3. **Commit the skill edit** alongside the website update.
