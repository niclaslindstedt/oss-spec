/**
 * Log line styles for the simulated terminal.
 */

// ---------------------------------------------------------------------------
// Style registry
// ---------------------------------------------------------------------------

export interface LogStyle {
  /** Tailwind CSS class(es) applied to the rendered line */
  className: string;
}

/** Named styles that output lines can reference. */
export const LOG_STYLES = {
  /** success / completion (green) */
  success: { className: "text-success" },
  /** failure / error (red) */
  failure: { className: "text-error" },
  /** spec-colored status (amber) */
  spec: { className: "text-spec-light" },
  /** assistant / tool-call activity (accent) */
  assistant: { className: "text-accent-light" },
  /** tool result arrow (green) */
  toolResult: { className: "text-success" },
  /** diff-stat lines */
  diffStat: { className: "text-text-secondary" },
  /** warning */
  warn: { className: "text-warn" },
  /** default dim output */
  dim: { className: "text-text-dim" },
} as const;

export type LogStyleName = keyof typeof LOG_STYLES;

// ---------------------------------------------------------------------------
// Terminal line types (shared between demo data and animation hook)
// ---------------------------------------------------------------------------

/** A single output line: plain string (defaults to "dim") or annotated. */
export type OutputLine = string | { text: string; style: LogStyleName };

export type TerminalLine =
  | { type: "command"; text: string; typingSpeed?: number }
  | { type: "output"; lines: OutputLine[]; delay?: number }
  | { type: "comment"; text: string }
  | { type: "pause"; duration: number };

export type TerminalTab = {
  label: string;
  sequence: TerminalLine[];
};

/** Produced by useTerminalAnimation, consumed by TerminalLine renderer. */
export type RenderedLine = {
  text: string;
  type: "command" | "output" | "comment";
  style?: LogStyleName;
  isActive: boolean;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Resolve an OutputLine to its text and style name. */
export function resolveOutputLine(line: OutputLine): {
  text: string;
  style: LogStyleName;
} {
  if (typeof line === "string") {
    return { text: line, style: "dim" };
  }
  return line;
}
