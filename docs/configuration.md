# Configuration

`oss-spec` has no configuration file. All inputs come from one of three sources,
in order of precedence:

1. **CLI flags** — `--name`, `--lang`, `--kind`, `--license`, `--visibility`,
   `--path`.
2. **AI-interpreted prompt** — when you call `oss-spec init "<prompt>"`, the
   freeform string is passed to `zag` and the resulting manifest fills any
   field a flag did not.
3. **Environment defaults** — `git config user.name` / `user.email` for
   author, `gh api user -q .login` for the GitHub owner.

## Bootstrap flags (`init`, `new`)

These flags are scoped to the `init` and `new` subcommands:

| Flag | Default | Effect |
|---|---|---|
| `--no-ai` | off | Skip every zag/LLM call. Use deterministic skeleton manifest. |
| `--no-git` | off | Skip `git init` and the first commit. |
| `--no-gh` | off | Skip `gh repo create`. |
| `-y, --yes` | off | Accept defaults for every interactive prompt. |

`--no-ai` also appears on `validate` (skip AI quality review) and `fix` (skip
AI calls). `--yes` also appears on `fix` (skip the confirmation prompt).
