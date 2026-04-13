import { useState } from "react";
import Terminal from "./terminal";
import { terminalDemos } from "../data/terminalDemos";
import { version } from "../data/sourceData";

export default function Hero() {
  const [copied, setCopied] = useState(false);

  const copyInstallCommand = () => {
    navigator.clipboard.writeText("cargo install oss-spec");
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section className="relative overflow-hidden pt-24 pb-16 md:pt-44 md:pb-32">
      {/* Background glow */}
      <div className="pointer-events-none absolute top-0 left-1/2 -translate-x-1/2 h-[600px] w-[800px] max-w-full rounded-full bg-accent/5 blur-3xl" />

      <div className="relative mx-auto max-w-6xl px-4 text-center sm:px-6">
        <a href="https://crates.io/crates/oss-spec" target="_blank" rel="noopener noreferrer" className="mb-6 inline-block rounded-full border border-border bg-surface-alt px-4 py-1.5 text-xs text-text-secondary hover:border-accent/50 hover:text-text-primary transition-colors">
          v{version} — Now available on crates.io
        </a>

        <h1 className="mx-auto max-w-4xl text-balance text-3xl leading-tight font-extrabold tracking-tight text-text-primary sm:text-4xl md:text-6xl md:leading-tight">
          Bootstrap{" "}
          <span className="bg-gradient-to-r from-accent to-accent-light bg-clip-text text-transparent">
            spec-compliant
          </span>{" "}
          open source repos
        </h1>

        <p className="mx-auto mt-6 max-w-2xl text-base text-text-secondary sm:text-lg md:text-xl">
          One command produces a complete project — LICENSE, README, AGENTS.md, CI workflows, docs,
          website, and more. All derived from{" "}
          <a href="https://github.com/niclaslindstedt/oss-spec/blob/main/OSS_SPEC.md" target="_blank" rel="noopener noreferrer" className="text-spec-light hover:underline">OSS_SPEC.md</a>.
          AI-optional via{" "}
          <a href="https://crates.io/crates/zag" target="_blank" rel="noopener noreferrer" className="text-accent-light hover:underline">zag</a>.
        </p>

        {/* Capability pills */}
        <div className="mt-8 flex flex-wrap items-center justify-center gap-2 sm:gap-3">
          {["AI-Optional", "5 Languages", "Spec-Driven", "Agent-Friendly", "Self-Dogfooding"].map((label) => (
            <span key={label} className="rounded-full border border-border bg-surface-alt px-3 py-1 text-xs font-medium text-accent sm:text-sm">
              {label}
            </span>
          ))}
        </div>

        {/* Animated terminal */}
        <Terminal tabs={terminalDemos} className="mx-auto mt-10 w-full max-w-2xl md:mt-12" />

        {/* Install CTA */}
        <div className="mt-8 flex flex-col items-center gap-4 sm:mt-10 sm:flex-row sm:justify-center">
          <a
            href="#get-started"
            className="rounded-lg bg-accent px-6 py-3 text-sm font-semibold text-white shadow-lg shadow-accent/20 hover:bg-accent-light transition-colors"
          >
            Get Started
          </a>
          <code className="relative inline-flex max-w-full items-center overflow-x-auto whitespace-nowrap rounded-lg border border-border bg-surface-alt pl-5 pr-10 py-3 text-sm text-text-secondary">
            cargo install oss-spec
            <button
              onClick={copyInstallCommand}
              className="absolute top-1/2 right-2 -translate-y-1/2 p-1 text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
              aria-label="Copy install command"
            >
              {copied ? (
                <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="m4.5 12.75 6 6 9-13.5" />
                </svg>
              ) : (
                <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 0 1-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 0 1 1.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 0 0-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 0 1-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 0 0-3.375-3.375h-1.5a1.125 1.125 0 0 1-1.125-1.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H9.75" />
                </svg>
              )}
            </button>
          </code>
        </div>
      </div>
    </section>
  );
}
