# `prompts/`

Versioned LLM prompts. See [`OSS_SPEC.md` §13.5](../OSS_SPEC.md#135-llm-prompts-prompts).

If this project sends prompts to a language model — directly via an SDK
or indirectly via a wrapper — every prompt must live here as a versioned
file rather than as an inline string in source code.

## Layout

```
prompts/
└── <prompt-name>/
    ├── 1_0.md
    ├── 1_1.md   # in-place edit, bumps minor
    └── 2_0.md   # breaking rewrite, bumps major
```

## File format

Each `<major>_<minor>.md` file is plain Markdown with two required
section headings:

```markdown
# <prompt-name> — v<major>.<minor>

## System

…system instructions for the model…

## User

…user message body. May contain {{ jinja }} placeholders that the
loader renders with runtime values…
```

Anything outside the `## System` and `## User` sections is ignored by
the loader and is purely for humans reading the file.

## Versioning rule

- **Minor bump** (`1_0` → `1_1`): in-place edit. Old version stays on
  disk so behavior changes can be diffed and bisected.
- **Major bump** (`1_x` → `2_0`): breaking rewrite that callers must be
  updated for.
- Loaders pick the highest version unless explicitly pinned.

If this project performs no LLM calls, leave this directory empty
(this README is enough to satisfy `oss-spec validate`).
