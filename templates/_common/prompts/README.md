# `prompts/`

Versioned LLM prompts. See [`OSS_SPEC.md` §13.5](../OSS_SPEC.md#135-llm-prompts-prompts).

If this project sends prompts to a language model — directly via an SDK
or indirectly via a wrapper — every prompt must live here as a versioned
file rather than as an inline string in source code.

## Layout

```
prompts/
└── <prompt-name>/
    ├── 1_0_0.md
    ├── 1_0_1.md   # wording fix, bumps patch
    ├── 1_1_0.md   # new placeholder / expanded scope, bumps minor
    └── 2_0_0.md   # breaking rewrite, bumps major
```

## File format

Each `<major>_<minor>_<patch>.md` file starts with YAML front matter and
contains two required section headings:

```markdown
---
name: <prompt-name>
description: "<one-sentence description>"
version: <major>.<minor>.<patch>
---

# <prompt-name>

## System

…system instructions for the model…

## User

…user message body. May contain {{ jinja }} placeholders that the
loader renders with runtime values…
```

The loader strips the YAML front matter before passing the prompt to
the model — it is metadata, not instruction content. Anything outside
the `## System` and `## User` sections is also ignored.

## Versioning rule

Filenames use [semver](https://semver.org/):

- **Patch bump** (`1_0_0` → `1_0_1`): wording fix / typo / clarification.
- **Minor bump** (`1_0_0` → `1_1_0`): new placeholder, expanded scope,
  additional guidance.
- **Major bump** (`1_0_0` → `2_0_0`): breaking rewrite — removed
  placeholder, changed JSON schema, fundamentally new task.

**Prompts are immutable once committed.** Never edit an existing
`<major>_<minor>_<patch>.md` file — every change lands as a new file at
a new version. Keep every prior version on disk so behavior changes can
be diffed and bisected. Loaders pick the highest version unless
explicitly pinned.

If this project performs no LLM calls, leave this directory empty
(this README is enough to satisfy `oss-spec validate`).
