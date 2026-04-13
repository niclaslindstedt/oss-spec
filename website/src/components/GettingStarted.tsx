import { commands } from "../data/sourceData";

const methods = [
  {
    title: "From crates.io",
    command: "cargo install oss-spec",
    note: "Requires Rust 1.85+",
  },
  {
    title: "From source",
    command: "git clone https://github.com/niclaslindstedt/oss-spec\ncd oss-spec && cargo install --path .",
    note: "Build from latest source",
  },
  {
    title: "GitHub Releases",
    command: "# Download pre-built binary from\n# github.com/niclaslindstedt/oss-spec/releases",
    note: "Pre-built for major platforms",
  },
];

const prereqs = [
  { name: "Rust 1.85+", cmd: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh", required: true },
  { name: "zag CLI", cmd: "cargo install zag-cli", required: false },
  { name: "gh CLI", cmd: "brew install gh  /  apt install gh", required: false },
];

export default function GettingStarted() {
  return (
    <section id="get-started" className="border-t border-border bg-surface-alt py-16 md:py-28">
      <div className="mx-auto max-w-5xl px-4 sm:px-6">
        <h2 className="text-balance text-center text-3xl font-bold text-text-primary md:text-4xl">
          Get started in seconds
        </h2>
        <p className="mx-auto mt-4 max-w-xl text-center text-text-secondary">
          Install oss-spec and you're ready to bootstrap projects or validate existing repos.
        </p>

        {/* Install methods */}
        <div className="mt-10 grid gap-5 md:mt-12 md:grid-cols-3 md:gap-6">
          {methods.map((m) => (
            <div key={m.title} className="min-w-0 rounded-xl border border-border bg-surface p-5">
              <h3 className="mb-1 text-sm font-semibold text-text-primary">{m.title}</h3>
              <p className="mb-3 text-xs text-text-dim">{m.note}</p>
              <pre className="overflow-x-auto rounded-lg bg-surface-alt p-3 text-xs leading-relaxed text-accent">
                <code>{m.command}</code>
              </pre>
            </div>
          ))}
        </div>

        {/* Prerequisites */}
        <div className="mt-12">
          <h3 className="mb-4 text-center text-lg font-semibold text-text-primary">
            Prerequisites
          </h3>
          <div className="mx-auto max-w-2xl space-y-2">
            {prereqs.map((p) => (
              <div key={p.name} className="flex flex-col gap-2 rounded-lg border border-border bg-surface px-4 py-2.5 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
                <span className="shrink-0 text-sm font-medium text-text-secondary">
                  {p.name}
                  {p.required && <span className="ml-2 text-xs text-accent">(required)</span>}
                  {!p.required && <span className="ml-2 text-xs text-text-dim">(optional)</span>}
                </span>
                <code className="block overflow-x-auto whitespace-nowrap text-xs text-text-dim sm:min-w-0 sm:text-right">
                  {p.cmd}
                </code>
              </div>
            ))}
          </div>
        </div>

        {/* Command reference */}
        <div className="mt-12">
          <h3 className="mb-4 text-center text-lg font-semibold text-text-primary">
            Command reference
          </h3>
          <div className="mx-auto max-w-2xl space-y-2">
            {/* Default prompt form */}
            <div className="flex flex-col gap-1 rounded-lg border border-border bg-surface px-4 py-2.5 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
              <code className="shrink-0 text-sm font-semibold text-accent">oss-spec &lt;PROMPT&gt;</code>
              <span className="text-xs text-text-dim sm:text-right">AI-powered project creation from a freeform prompt</span>
            </div>
            {commands.map((c) => (
              <div key={c.name} className="flex flex-col gap-1 rounded-lg border border-border bg-surface px-4 py-2.5 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
                <code className="shrink-0 text-sm font-semibold text-accent">oss-spec {c.name}</code>
                <span className="text-xs text-text-dim sm:text-right">{c.description}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Quick verify */}
        <div className="mx-auto mt-12 max-w-lg rounded-xl border border-border bg-surface p-5">
          <p className="mb-3 text-center text-sm text-text-secondary">Try it out:</p>
          <pre className="overflow-x-auto text-xs text-text-secondary sm:text-sm">
            <code className="whitespace-pre">
              <span className="text-accent">$</span> oss-spec new hello-world --lang rust --kind cli --no-ai -y
            </code>
          </pre>
        </div>
      </div>
    </section>
  );
}
