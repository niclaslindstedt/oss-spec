# Example: bootstrap a Rust CLI

A minimal end-to-end demo of using `oss-spec` to create a new Rust CLI project.

## Run

```sh
cargo install oss-spec
oss-spec new stocks-cli \
    --lang rust \
    --kind cli \
    --license MIT \
    -d "Find stock buy signals from the command line." \
    --no-ai --no-gh --yes \
    --path /tmp
```

## What you get

```
/tmp/stocks-cli/
├── LICENSE
├── README.md
├── AGENTS.md
├── CLAUDE.md → AGENTS.md
├── .cursorrules → AGENTS.md
├── .windsurfrules → AGENTS.md
├── GEMINI.md → AGENTS.md
├── .github/copilot-instructions.md → ../AGENTS.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
├── CHANGELOG.md
├── .gitignore
├── .editorconfig
├── Makefile
├── Cargo.toml
├── src/{lib.rs, main.rs}
├── .github/{workflows/*, ISSUE_TEMPLATE/*, PULL_REQUEST_TEMPLATE.md, dependabot.yml, CODEOWNERS}
├── docs/{getting-started.md, configuration.md, architecture.md, troubleshooting.md}
├── examples/
├── scripts/{release.sh, generate-changelog.sh, update-versions.sh}
├── website/{package.json, index.html, scripts/extract-source-data.mjs, src/generated/}
├── man/main.md
└── .claude/{settings.local.json, skills/commit/SKILL.md}
```

## Verify it conforms

```sh
oss-spec validate --path /tmp/stocks-cli
# ✓ repo conforms to OSS_SPEC.md
```

## Build it

```sh
cd /tmp/stocks-cli && cargo build
```
