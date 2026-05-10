# Architecture of oss-spec

`oss-spec` is a single Rust crate that ships both a binary and a library.

```
src/
├── main.rs        # tokio entry, defers to lib::run
├── lib.rs         # public re-exports + run()
├── cli.rs         # clap derive + dispatch
├── interview.rs   # interactive Q&A → ProjectManifest
├── ai.rs          # thin zag wrappers (interpret_prompt, tailor_init, fix_conformance, verify_conformance)
├── manifest.rs    # ProjectManifest, Language, Kind, License enums
├── render.rs      # minijinja env + render_str
├── embedded.rs    # include_dir!("templates")
├── bootstrap.rs   # walks embedded tree → writes target dir
├── tailor.rs      # interactive post-bootstrap tailoring agent (§23)
├── git.rs         # git init / gh repo create wrappers
├── validate/      # §19 conformance validator
│   ├── mod.rs         # Report/Violation types and orchestrator
│   ├── structural.rs  # required files/dirs/symlinks/workflows
│   ├── content.rs     # §11.3 SEO scaffolding, §19.4 output module, §20 inline tests, §20.5 file size
│   ├── agent_skills.rs# §21 .agent/skills/ tree and per-skill checks
│   └── toolchain.rs   # §10.3/§10.5 pin-file and CI parity
├── fix.rs         # zag-driven auto-fix agent
├── agent_help.rs  # §12 CLI discoverability contract
└── output.rs      # central logging + styled output (§19 logging)
```

## Data flow for `oss-spec init "<prompt>"`

1. `main` parses `Cli` (clap).
2. `cli::dispatch` matches the `init` subcommand and sees a prompt →
   calls `interview::from_prompt`.
3. `interview::from_prompt` calls `ai::interpret_prompt`, which uses `zag`
   to get a JSON-schema-validated manifest.
4. The user confirms (or refines via `interview::run`).
5. `bootstrap::write` walks the embedded `templates/_common` tree, applies
   the language overlay (`templates/<lang>`) and the optional CLI overlay
   (`templates/cli`), renders each `*.tmpl` through minijinja, copies
   non-template files verbatim, and creates the AGENTS.md symlinks.
6. `tailor::run` (unless `--no-tailor` or `--no-ai`) launches an
   interactive `zag` agent that proposes edits to the scaffolding layer
   (README, AGENTS.md, docs, skills, workflows). Each tool call is
   surfaced to the user for approval — application source under `src/`
   and `tests/` is off-limits.
7. `git::init_and_commit` lands the first commit; `git::gh_create`
   (with confirmation) publishes to GitHub.

## Why embed everything?

`include_dir!` compiles `templates/`, `docs/`, and `man/` into the binary at
build time so a `cargo install oss-spec` user has zero runtime data
dependencies.
