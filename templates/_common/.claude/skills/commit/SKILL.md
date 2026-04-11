---
name: commit
description: Run quality gates (build/test/lint/fmt), then create a Conventional Commit and push.
---

# /commit

When invoked:

1. Run `make build`, `make test`, `make lint`, `make fmt-check`. Stop on first failure.
2. Inspect `git status` and `git diff --cached`. If nothing is staged, ask whether to `git add -p`.
3. Draft a Conventional Commit message:
   - Type: feat|fix|perf|docs|test|refactor|chore|ci|build|style
   - Scope: optional, one word
   - Summary: imperative, < 72 chars
   - Body: only if non-trivial; explain *why*
4. Show the message and ask to confirm before committing.
5. After commit: ask whether to push.

Conventions:

- PRs are squash-merged. Branch commits are throwaway; the **PR title** is the commit on `main`.
- Breaking changes use `<type>!:` or `BREAKING CHANGE:` footer.
