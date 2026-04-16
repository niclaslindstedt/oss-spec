import { languages, kinds, commands } from "../data/sourceData";

const features = [
  {
    title: "One Command to a Real Repo",
    description:
      'Run oss-spec init with a prompt like "create a python cli for stock analysis" and get a complete project — LICENSE, README, AGENTS.md, CI, docs, website, and more.',
    icon: "\u{1F680}",
  },
  {
    title: "Spec is the Source of Truth",
    description:
      "Every file is derived from OSS_SPEC.md — a 20-section prescriptive specification. Run `oss-spec validate` to see exactly which items a repo is missing.",
    icon: "\u{1F4DC}",
  },
  {
    title: "AI is Optional",
    description:
      "With --no-ai you get a deterministic skeleton from flags alone. With the default flow, zag interprets a freeform prompt into a structured manifest. Same engine either way.",
    icon: "\u{1F9E0}",
  },
  {
    title: `${languages.length} Languages`,
    description:
      `Supports ${languages.join(", ")}. Each language gets idiomatic project structure, build tooling, and CI configuration out of the box.`,
    icon: "\u{1F30D}",
  },
  {
    title: "Agent-Friendly (\u00a712)",
    description:
      "Generated repos ship with the full CLI discoverability contract: --help-agent, --debug-agent, commands, docs, and man — so coding agents can self-serve.",
    icon: "\u{1F916}",
  },
  {
    title: "Remote Validate & Fix",
    description:
      "Point validate or fix at any git URL with --url. Add --create-issues to automatically file one GitHub issue per violation. Works on repos you don't own.",
    icon: "\u{1F527}",
  },
  {
    title: "Self-Dogfooding",
    description:
      "oss-spec is its own first customer — `oss-spec validate .` against the oss-spec repo passes. The tool eats its own dog food.",
    icon: "\u{1F436}",
  },
  {
    title: "Embedded Docs & Man Pages",
    description:
      "Every generated project includes embedded documentation accessible via `docs` and `man` subcommands. No external dependencies needed.",
    icon: "\u{1F4D6}",
  },
  {
    title: `${kinds.length} Project Kinds`,
    description:
      `Choose from ${kinds.join(", ")}. Each kind gets appropriate structure — a CLI gets a manpage, a service gets health checks, a library gets API docs.`,
    icon: "\u{1F4E6}",
  },
];

export default function Features() {
  return (
    <section id="features" className="border-t border-border py-16 md:py-28">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <h2 className="text-balance text-center text-3xl font-bold text-text-primary md:text-4xl">
          Everything you need to bootstrap open source
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          From AI-powered project creation to spec conformance validation — {commands.length} commands cover the full lifecycle.
        </p>

        <div className="mt-10 grid gap-5 sm:mt-14 sm:grid-cols-2 sm:gap-6 lg:grid-cols-3">
          {features.map((f) => (
            <div
              key={f.title}
              className="group rounded-xl border border-border bg-surface-alt p-6 transition-all hover:border-accent/40 hover:bg-surface-hover"
            >
              <div className="mb-4 text-2xl">{f.icon}</div>
              <h3 className="mb-2 text-lg font-semibold text-text-primary">{f.title}</h3>
              <p className="text-sm leading-relaxed text-text-secondary">{f.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
