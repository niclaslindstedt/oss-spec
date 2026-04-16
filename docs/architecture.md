# Architecture of oss-spec

`oss-spec` is a single Rust crate that ships both a binary and a library.

```
src/
├── main.rs        # tokio entry, defers to lib::run
├── lib.rs         # public re-exports + run()
├── cli.rs         # clap derive + dispatch
├── interview.rs   # interactive Q&A → ProjectManifest
├── ai.rs          # thin zag wrappers (interpret_prompt, draft_readme_why)
├── manifest.rs    # ProjectManifest, Language, Kind, License enums
├── render.rs      # minijinja env + render_str
├── embedded.rs    # include_dir!("templates")
├── bootstrap.rs   # walks embedded tree → writes target dir
├── git.rs         # git init / gh repo create wrappers
├── validate.rs    # §19 conformance validator
└── agent_help.rs  # §12 CLI discoverability contract
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
6. `git::init_and_commit` lands the first commit; `git::gh_create`
   (with confirmation) publishes to GitHub.

## Why embed everything?

`include_dir!` compiles `templates/`, `docs/`, and `man/` into the binary at
build time so a `cargo install oss-spec` user has zero runtime data
dependencies.
