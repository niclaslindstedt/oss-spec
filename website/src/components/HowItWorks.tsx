const steps = [
  {
    number: "1",
    title: "Describe",
    subtitle: "Tell oss-spec what to build",
    description:
      "Use a freeform prompt or explicit flags. The AI flow interprets your intent via zag; the --no-ai path uses flags directly. Either way you get a structured manifest.",
    code: `oss-spec "create a go service for user auth"`,
    color: "text-accent",
  },
  {
    number: "2",
    title: "Bootstrap",
    subtitle: "Complete repo generated",
    description:
      "oss-spec writes every file the spec mandates: LICENSE, README, AGENTS.md (with all agent symlinks), CONTRIBUTING, SECURITY, CI workflows, docs, website skeleton, language manifest, Makefile, and more.",
    code: `my-project/
  LICENSE          README.md
  AGENTS.md        CLAUDE.md -> AGENTS.md
  CONTRIBUTING.md  CODE_OF_CONDUCT.md
  SECURITY.md      CHANGELOG.md
  Makefile         .editorconfig
  src/             docs/
  .github/         website/
  prompts/         scripts/`,
    color: "text-spec-light",
  },
  {
    number: "3",
    title: "Ship",
    subtitle: "git init, gh create, ready to go",
    description:
      "oss-spec initializes the git repo, makes the first commit, and creates the GitHub remote. Your project is live and CI is already running.",
    code: `$ cd my-project && git log --oneline
a1b2c3d Initial commit (oss-spec bootstrap)

$ gh repo view --web
Opening github.com/you/my-project...`,
    color: "text-success",
  },
];

export default function HowItWorks() {
  return (
    <section id="how-it-works" className="border-t border-border bg-surface-alt py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Three steps to a real repo
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Describe what you want, bootstrap the project, ship it.
        </p>

        <div className="mt-14 grid gap-8 lg:grid-cols-3">
          {steps.map((s) => (
            <div key={s.number} className="relative rounded-xl border border-border bg-surface p-6">
              <div className={`mb-4 inline-flex h-10 w-10 items-center justify-center rounded-full border border-border text-lg font-bold ${s.color}`}>
                {s.number}
              </div>
              <h3 className={`mb-1 text-xl font-bold ${s.color}`}>{s.title}</h3>
              <p className="mb-4 text-xs font-medium uppercase tracking-wider text-text-dim">{s.subtitle}</p>
              <p className="mb-4 text-sm leading-relaxed text-text-secondary">{s.description}</p>
              <pre className="overflow-x-auto rounded-lg bg-surface-alt p-3 text-xs leading-relaxed text-accent">
                <code>{s.code}</code>
              </pre>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
