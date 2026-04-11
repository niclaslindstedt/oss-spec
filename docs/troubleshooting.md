# Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `zag agent execution failed` | LLM provider not configured | Run with `--no-ai`, or install/auth your provider CLI |
| `gh: command not found` | GitHub CLI missing | Install `gh` or pass `--no-gh` |
| `fatal: empty ident` | git user.name/email unset | `git config --global user.name "..."` and `user.email "..."` |
| Symlinks fail on Windows | Symlink permission required | Enable Developer Mode or run as admin |
| `templates/_common missing` | Built without templates/ present | Reinstall from a clean source tree |
