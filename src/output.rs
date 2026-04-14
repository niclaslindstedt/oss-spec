//! Central output and logging module.
//!
//! All user-facing output flows through the semantic functions in this module
//! (`status`, `warn`, `info`, `header`, `error`). Raw `println!`/`eprintln!`
//! must not appear outside this module except for machine-readable §12 contract
//! surfaces (--help-agent, --debug-agent, commands, docs, man).
//!
//! Logging is dual-target:
//! - **File**: always-on, appends to `<data_local_dir>/oss-spec/debug.log` at
//!   `Debug` level with timestamps.
//! - **Stderr**: `log::debug!` messages appear only when `--debug` is set.
//!   Semantic output functions (`status`, `warn`, etc.) always write to stderr
//!   directly with styling, bypassing the log framework for terminal output.

use std::io::Write as _;

/// Target prefix used by semantic output functions. The stderr dispatch
/// filters this out to avoid double-printing (the functions already write
/// styled output to stderr directly).
const OUTPUT_TARGET: &str = "oss_spec::_output";

/// Initialise dual-target logging. Call once from `main`, before any output.
pub fn init(debug: bool) -> anyhow::Result<()> {
    let file_dispatch = {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("oss-spec");
        std::fs::create_dir_all(&log_dir)?;
        let log_path = log_dir.join("debug.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} [{}] {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    record.level(),
                    message,
                ))
            })
            .level(log::LevelFilter::Debug)
            .chain(file)
    };

    let stderr_level = if debug {
        log::LevelFilter::Debug
    } else {
        // Even without --debug, we need Warn so that log::warn! from other
        // crates can surface. But our own semantic functions write to stderr
        // directly, so we exclude the OUTPUT_TARGET below.
        log::LevelFilter::Warn
    };

    let stderr_dispatch = fern::Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("{message}")))
        .level(stderr_level)
        .filter(|meta| {
            // Always suppress the semantic output target (those functions
            // already wrote styled output to stderr).
            if meta.target().starts_with("oss_spec::_output") {
                return false;
            }
            // Only show oss_spec messages (skip noisy dependency crates).
            meta.target().starts_with("oss_spec")
        })
        .chain(std::io::stderr());

    fern::Dispatch::new()
        .chain(file_dispatch)
        .chain(stderr_dispatch)
        .apply()
        .map_err(|e| anyhow::anyhow!("failed to initialise logging: {e}"))?;

    Ok(())
}

/// Return the path to the debug log file.
pub fn log_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("oss-spec")
        .join("debug.log")
}

/// Success message with green checkmark prefix.
pub fn status(msg: &str) {
    let styled = format!("{}  {msg}", console::style("✓").green().bold());
    let _ = writeln!(std::io::stderr(), "{styled}");
    log::info!(target: OUTPUT_TARGET, "{msg}");
}

/// Warning message with yellow `!` prefix.
pub fn warn(msg: &str) {
    let styled = format!("{} {msg}", console::style("!").yellow());
    let _ = writeln!(std::io::stderr(), "{styled}");
    log::warn!(target: OUTPUT_TARGET, "{msg}");
}

/// Informational message, no prefix.
pub fn info(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{msg}");
    log::info!(target: OUTPUT_TARGET, "{msg}");
}

/// Bold section header.
pub fn header(msg: &str) {
    let styled = format!("{}", console::style(msg).bold());
    let _ = writeln!(std::io::stderr(), "{styled}");
    log::info!(target: OUTPUT_TARGET, "{msg}");
}

/// Error message with red `✗` prefix.
pub fn error(msg: &str) {
    let styled = format!("{} {msg}", console::style("✗").red().bold());
    let _ = writeln!(std::io::stderr(), "{styled}");
    log::error!(target: OUTPUT_TARGET, "{msg}");
}

/// Debug message — only visible with `--debug` on stderr, always in log file.
pub fn debug(msg: &str) {
    log::debug!(target: "oss_spec", "{msg}");
}

/// A terminal spinner for long-running operations. Shows an animated indicator
/// with a message on stderr while work proceeds. Call [`Spinner::finish`],
/// [`Spinner::fail`], or [`Spinner::clear`] when done — dropping without
/// finishing abandons the spinner in place.
pub struct Spinner {
    pb: indicatif::ProgressBar,
}

impl Spinner {
    /// Start a new spinner with the given message.
    pub fn start(msg: &str) -> Self {
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_style(
            indicatif::ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner} {msg}")
                .expect("valid template"),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        log::info!(target: OUTPUT_TARGET, "{msg}");
        Self { pb }
    }

    /// Update the spinner message mid-flight.
    pub fn set_message(&self, msg: &str) {
        self.pb.set_message(msg.to_string());
        log::info!(target: OUTPUT_TARGET, "{msg}");
    }

    /// Stop the spinner with a green checkmark.
    pub fn finish(self, msg: &str) {
        self.pb
            .finish_with_message(format!("{}  {msg}", console::style("✓").green().bold()));
        log::info!(target: OUTPUT_TARGET, "{msg}");
    }

    /// Stop the spinner and clear its line entirely.
    pub fn clear(self) {
        self.pb.finish_and_clear();
    }

    /// Stop the spinner with a red cross.
    pub fn fail(self, msg: &str) {
        self.pb
            .finish_with_message(format!("{} {msg}", console::style("✗").red().bold()));
        log::error!(target: OUTPUT_TARGET, "{msg}");
    }
}
