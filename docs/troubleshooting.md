# Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `zag agent execution failed` | LLM provider not configured | Run with `--no-ai`, or install/auth your provider CLI |
| `gh: command not found` | GitHub CLI missing | Install `gh` or pass `--no-gh` |
| `fatal: empty ident` | git user.name/email unset | `git config --global user.name "..."` and `user.email "..."` |
| Symlinks fail on Windows | Symlink permission required | Enable Developer Mode or run as admin |
| `templates/_common missing` | Built without templates/ present | Reinstall from a clean source tree |
| `[§24] … usedBy [...] does not match the files that cite it [...]` | A `[ref:<id>]` tag was added to or removed from a file without updating the registry | Set that entry's `usedBy` in `docs/references.json` to exactly the files the message lists |
| `[§24] [ref:<id>] is cited in … but has no entry` | A source is cited in code but not recorded | Add the entry from the source itself — title, year, authors, DOI/URL, verbatim quotes. Never invent one |
| `[§24] … \`summary\` must be an object …` / `\`topics\` must be a non-empty list …` / `\`language\` is not a BCP 47 language tag` | An optional §24.2 field is present but out of shape | Key `summary` by language tag (`{ "en": "…" }`), make `topics` a list of kebab-case names, write `language` as a tag (`en`, not `English`) |
| `jq not found; skipping the §24 references registry checks` | `scripts/validate.sh` reads the registry with `jq` | Install `jq`, or run the `oss-spec` binary |
