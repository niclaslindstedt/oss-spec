# Getting started with oss-spec

`oss-spec` bootstraps a new open source repository that follows
[`OSS_SPEC.md`](../OSS_SPEC.md). The fastest way to use it:

```sh
cargo install oss-spec
oss-spec "create a python cli for finding stock buys"
```

That single command:

1. Sends the prompt to the [`zag`](https://crates.io/crates/zag) library, which
   asks an LLM to interpret it into a structured manifest (name, language,
   kind, license, README "Why?" bullets).
2. Shows the proposed manifest and asks you to confirm.
3. Materializes a complete OSS_SPEC.md-compliant repo on disk: LICENSE,
   README, AGENTS.md (with all the symlinks), CONTRIBUTING/COC/SECURITY,
   `.github/` (CI, release, pages, dependabot, issue/PR templates), `docs/`,
   `examples/`, `website/`, `Makefile`, `scripts/`, language-specific
   manifests (Cargo.toml / pyproject.toml / package.json / go.mod), and a
   `.claude/` directory with a starter `commit` skill.
4. Runs `git init`, makes the first commit, and (with confirmation) calls
   `gh repo create` to publish.

## Without AI

```sh
oss-spec new my-tool --lang rust --kind cli --license MIT --no-ai --yes
```

## Validate an existing repo

```sh
oss-spec check --path .
```

Exits 0 when the repo conforms to the §19 checklist; otherwise prints a
numbered violation report and exits 1.
