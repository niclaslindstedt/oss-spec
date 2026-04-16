import { useState } from "react";
import { specChecks } from "../data/sourceData";

const repoTree = [
  { name: "LICENSE", indent: 0 },
  { name: "README.md", indent: 0 },
  { name: "CONTRIBUTING.md", indent: 0 },
  { name: "CODE_OF_CONDUCT.md", indent: 0 },
  { name: "SECURITY.md", indent: 0 },
  { name: "AGENTS.md", indent: 0 },
  { name: "CLAUDE.md", indent: 0, note: "-> AGENTS.md" },
  { name: ".cursorrules", indent: 0, note: "-> AGENTS.md" },
  { name: ".windsurfrules", indent: 0, note: "-> AGENTS.md" },
  { name: "GEMINI.md", indent: 0, note: "-> AGENTS.md" },
  { name: "CHANGELOG.md", indent: 0 },
  { name: "Makefile", indent: 0 },
  { name: ".gitignore", indent: 0 },
  { name: ".editorconfig", indent: 0 },
  { name: "src/", indent: 0, isDir: true },
  { name: "docs/", indent: 0, isDir: true },
  { name: "getting-started.md", indent: 1 },
  { name: "configuration.md", indent: 1 },
  { name: "architecture.md", indent: 1 },
  { name: "troubleshooting.md", indent: 1 },
  { name: ".github/", indent: 0, isDir: true },
  { name: "workflows/", indent: 1, isDir: true },
  { name: "ci.yml", indent: 2 },
  { name: "release.yml", indent: 2 },
  { name: "version-bump.yml", indent: 2 },
  { name: "pages.yml", indent: 2 },
  { name: "ISSUE_TEMPLATE/", indent: 1, isDir: true },
  { name: "PULL_REQUEST_TEMPLATE.md", indent: 1 },
  { name: "copilot-instructions.md", indent: 1, note: "-> AGENTS.md" },
  { name: "prompts/", indent: 0, isDir: true },
  { name: "scripts/", indent: 0, isDir: true },
  { name: "website/", indent: 0, isDir: true },
];

const INITIAL_CHECKS = 10;

export default function TheSpec() {
  const [showAll, setShowAll] = useState(false);
  const visibleChecks = showAll ? specChecks : specChecks.slice(0, INITIAL_CHECKS);

  return (
    <section id="the-spec" className="border-t border-border py-16 md:py-28">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <h2 className="text-balance text-center text-3xl font-bold text-text-primary md:text-4xl">
          Driven by{" "}
          <span className="text-spec-light">OSS_SPEC.md</span>
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Every generated file traces back to a section of the spec. Run{" "}
          <code className="rounded bg-surface-alt px-1.5 py-0.5 text-sm text-accent">oss-spec validate</code>{" "}
          to validate any repo against it.
        </p>

        <div className="mt-10 grid gap-6 md:mt-14 md:gap-8 lg:grid-cols-2">
          {/* Left: generated repo tree */}
          <div className="min-w-0 rounded-xl border border-border bg-surface-alt overflow-hidden">
            <div className="flex items-center border-b border-border px-4 py-3">
              <div className="flex items-center gap-2 mr-4">
                <div className="h-3 w-3 rounded-full bg-[#ff5f57]" />
                <div className="h-3 w-3 rounded-full bg-[#febc2e]" />
                <div className="h-3 w-3 rounded-full bg-[#28c840]" />
              </div>
              <span className="text-xs text-text-dim font-mono">my-project/</span>
            </div>
            <div className="p-4 sm:p-5 font-mono text-xs sm:text-sm leading-relaxed max-h-[480px] overflow-auto">
              {repoTree.map((entry, i) => (
                <div key={i} className="whitespace-nowrap" style={{ paddingLeft: `${entry.indent * 20}px` }}>
                  <span className={entry.isDir ? "text-accent" : "text-text-primary"}>
                    {entry.name}
                  </span>
                  {entry.note && (
                    <span className="text-text-dim ml-2">{entry.note}</span>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Right: spec check reference */}
          <div className="min-w-0">
            <h3 className="mb-4 text-lg font-semibold text-text-primary">
              Conformance checks ({specChecks.length} items)
            </h3>
            <div className="space-y-2">
              {visibleChecks.map((check, i) => (
                <div
                  key={i}
                  className="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface-alt px-4 py-2.5"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <span className={`shrink-0 text-xs font-medium ${
                      check.type === "file" ? "text-accent" :
                      check.type === "symlink" ? "text-spec-light" :
                      check.type === "directory" ? "text-accent-light" :
                      "text-text-secondary"
                    }`}>
                      {check.type}
                    </span>
                    <code className="text-sm text-text-primary truncate">{check.item}</code>
                  </div>
                  <span className="shrink-0 text-xs text-text-dim">{check.section}</span>
                </div>
              ))}
            </div>
            {specChecks.length > INITIAL_CHECKS && (
              <button
                onClick={() => setShowAll(!showAll)}
                className="mt-4 w-full rounded-lg border border-border bg-surface-alt py-2.5 text-sm text-text-secondary hover:border-accent/40 hover:text-text-primary transition-all"
              >
                {showAll
                  ? "Show fewer"
                  : `Show all ${specChecks.length} checks`}
              </button>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
