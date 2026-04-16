import type { TerminalTab } from "./logStyles";

export const terminalDemos: TerminalTab[] = [
  {
    label: "Bootstrap",
    sequence: [
      { type: "comment", text: "# Bootstrap a new project from a prompt" },
      {
        type: "command",
        text: 'oss-spec init "create a python cli for finding stock buys"',
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u25b6 Interpreting prompt with zag...", style: "spec" },
          "",
        ],
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 100,
        lines: [
          { text: "  Name:     stock-finder", style: "diffStat" },
          { text: "  Language: python  |  Kind: cli  |  License: MIT", style: "diffStat" },
          "",
          { text: "\u25b6 Bootstrapping stock-finder...", style: "spec" },
        ],
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        delay: 80,
        lines: [
          { text: "  \u2713 LICENSE (MIT)", style: "success" },
          { text: "  \u2713 README.md", style: "success" },
          { text: "  \u2713 AGENTS.md + 5 symlinks", style: "success" },
          { text: "  \u2713 pyproject.toml + src/stock_finder/main.py", style: "success" },
          { text: "  \u2713 .github/workflows/ci.yml", style: "success" },
          { text: "  \u2713 docs/ (4 topics)", style: "success" },
          { text: "  \u2713 website/ scaffold", style: "success" },
        ],
      },
      { type: "pause", duration: 300 },
      {
        type: "output",
        lines: [
          "",
          { text: "\u2713 git init && git add -A && git commit", style: "assistant" },
          { text: "\u2713 gh repo create niclas/stock-finder --public", style: "assistant" },
          "",
          { text: "\u2713 Bootstrapped stock-finder at ./stock-finder", style: "success" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Validate",
    sequence: [
      { type: "comment", text: "# Validate a remote repo against OSS_SPEC.md" },
      {
        type: "command",
        text: "oss-spec validate --url https://github.com/example/my-project",
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u25b6 Cloning (shallow)...", style: "spec" },
          { text: "\u25b6 Checking conformance...", style: "spec" },
        ],
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 120,
        lines: [
          "",
          { text: "\u2717 5 violations:", style: "failure" },
          { text: "   1. [\u00a72]  missing LICENSE", style: "warn" },
          { text: "   2. [\u00a77]  missing AGENTS.md", style: "warn" },
          { text: "   3. [\u00a77.1] CLAUDE.md must be a symlink to AGENTS.md", style: "warn" },
          { text: "   4. [\u00a710] missing .github/workflows/ci.yml", style: "warn" },
          { text: "   5. [\u00a711] missing directory docs", style: "warn" },
        ],
      },
      { type: "pause", duration: 800 },
      { type: "comment", text: "# Auto-file GitHub issues for each violation" },
      {
        type: "command",
        text: "oss-spec validate --url https://github.com/example/my-project --create-issues --yes",
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        delay: 100,
        lines: [
          { text: "\u25b6 Filing issues via gh...", style: "spec" },
          { text: "  \u2713 Created issue #1: Add LICENSE file (\u00a72)", style: "success" },
          { text: "  \u2713 Created issue #2: Add AGENTS.md + symlinks (\u00a77)", style: "success" },
          { text: "  \u2713 Created issue #3: Add CI workflow (\u00a710)", style: "success" },
          { text: "  \u2713 Created issue #4: Add docs directory (\u00a711)", style: "success" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Fix",
    sequence: [
      { type: "comment", text: "# Auto-fix violations in the current repo" },
      {
        type: "command",
        text: "oss-spec fix",
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u25b6 Running oss-spec validate...", style: "spec" },
          { text: "  5 violations found", style: "warn" },
          "",
          { text: "\u25b6 Fixing via zag agent...", style: "spec" },
        ],
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 120,
        lines: [
          { text: "  \u2713 Added LICENSE (MIT)", style: "success" },
          { text: "  \u2713 Created AGENTS.md + 5 symlinks", style: "success" },
          { text: "  \u2713 Added .github/workflows/ci.yml", style: "success" },
          { text: "  \u2713 Added CONTRIBUTING.md", style: "success" },
          { text: "  \u2713 Added docs/ directory", style: "success" },
        ],
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        lines: [
          "",
          { text: "\u2713 All violations resolved", style: "success" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Agent Contract",
    sequence: [
      { type: "comment", text: "# \u00a712 CLI discoverability contract" },
      {
        type: "command",
        text: "oss-spec commands",
      },
      { type: "pause", duration: 200 },
      {
        type: "output",
        delay: 80,
        lines: [
          { text: "  new         Bootstrap a new project", style: "diffStat" },
          { text: "  init        Bootstrap into the current directory", style: "diffStat" },
          { text: "  validate    Validate repo against \u00a719 checklist", style: "diffStat" },
          { text: "  fix         Fix \u00a719 violations in place", style: "diffStat" },
          { text: "  fetch       Clone oss-spec repo for reference", style: "diffStat" },
          { text: "  commands    Machine-readable command index", style: "diffStat" },
          { text: "  docs        Print embedded docs topic", style: "diffStat" },
          { text: "  man         Print embedded manpage", style: "diffStat" },
        ],
      },
      { type: "pause", duration: 800 },
      { type: "comment", text: "# Agents can self-serve documentation" },
      {
        type: "command",
        text: "oss-spec docs getting-started",
      },
      { type: "pause", duration: 200 },
      {
        type: "output",
        delay: 60,
        lines: [
          { text: "  # Getting started with oss-spec", style: "assistant" },
          { text: "  ", style: "dim" },
          { text: "  ## Quick start", style: "assistant" },
          { text: '  oss-spec init "create a python cli for stock analysis"', style: "diffStat" },
          { text: "  ", style: "dim" },
          { text: "  ## Deterministic mode", style: "assistant" },
          { text: "  oss-spec new my-tool --lang rust --kind cli --no-ai -y", style: "diffStat" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
];
