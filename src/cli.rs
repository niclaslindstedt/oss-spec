//! Clap definitions for the `oss-spec` binary.
//!
//! Bootstrap is a single subcommand: `init`. Without `--name` it fills the
//! current directory (or `--path`); with `--name NAME` it creates a new
//! subdirectory and bootstraps there. Other subcommands (`validate`, `fix`,
//! `fetch`, `commands`, `docs`, `man`) cover the validation / power-user
//! paths and the §12 CLI discoverability contract.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Flags shared by the `init` bootstrap path.
#[derive(Debug, Clone, clap::Args)]
pub struct BootstrapOpts {
    /// Skip AI calls — produces a deterministic skeleton from defaults/flags only.
    #[arg(long)]
    pub no_ai: bool,

    /// Skip `git init` and the first commit.
    #[arg(long)]
    pub no_git: bool,

    /// Skip `gh repo create`.
    #[arg(long)]
    pub no_gh: bool,

    /// Assume defaults for every interactive prompt.
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Target directory for the new repo. With `--name`, treated as the
    /// parent directory (target = <path>/<name>). Without `--name`, the
    /// repo is materialized directly into this directory. Defaults to CWD.
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,

    /// Override language: rust|python|node|go|generic.
    #[arg(long)]
    pub lang: Option<String>,

    /// Override kind: lib|cli|service.
    #[arg(long)]
    pub kind: Option<String>,

    /// Override license: MIT|Apache-2.0|MPL-2.0.
    #[arg(long)]
    pub license: Option<String>,

    /// gh visibility: public|private. Defaults to public.
    #[arg(long)]
    pub visibility: Option<String>,
}

#[derive(Debug, Parser)]
#[command(
    name = "oss-spec",
    version,
    about = "Bootstrap new OSS_SPEC.md-compliant repositories",
    long_about = "oss-spec materializes new open source repositories that follow the conventions in OSS_SPEC.md (LICENSE, README, AGENTS.md + symlinks, CI workflows, docs, examples, website, CLI agent contract). Use `oss-spec init` to bootstrap a project — optionally with a freeform prompt that the zag library interprets into a structured manifest."
)]
pub struct Cli {
    /// Print a plain-text dump suitable for prompt injection (§12.1).
    #[arg(long, exclusive = true)]
    pub help_agent: bool,

    /// Print a plain-text troubleshooting dump (§12.2).
    #[arg(long, exclusive = true)]
    pub debug_agent: bool,

    /// Enable debug-level output on stdout and verbose file logging.
    #[arg(long, global = true)]
    pub debug: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Bootstrap a project. Without `--name`, fills the current directory
    /// (or `--path`); with `--name NAME`, creates `<path|cwd>/<NAME>` and
    /// bootstraps there. A freeform prompt triggers zag interpretation;
    /// without one, runs the interactive interview.
    Init {
        /// Freeform prompt — e.g. "create a python cli for finding stock buys".
        /// When given, oss-spec interprets it via zag to infer language, kind,
        /// license, description, and README "Why?" bullets.
        #[arg(value_name = "PROMPT")]
        prompt: Option<String>,
        /// Project description (used when no prompt is given).
        #[arg(long, short = 'd')]
        description: Option<String>,
        /// Project name. When set, a new subdirectory of this name is
        /// created under `--path` (or CWD) and bootstrapped. When unset,
        /// the bootstrap fills the current/`--path` directory and defaults
        /// the project name to that directory's name.
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        opts: BootstrapOpts,
    },
    /// Validate an existing repo against the §19 checklist. With `--url`,
    /// clones a remote repo into a temp dir first. With `--create-issues`,
    /// files one GitHub issue per violation after reporting.
    ///
    /// By default, also runs an AI quality review of file contents (skip
    /// with `--no-ai`). With `--fix`, automatically fixes all findings.
    Validate {
        /// Local repo to validate. Defaults to the current directory.
        #[arg(long, default_value = ".", conflicts_with = "url")]
        path: PathBuf,
        /// Validate a remote git repo by cloning it into a temp dir first.
        /// The clone is removed after the command finishes.
        #[arg(long, value_name = "URL")]
        url: Option<String>,
        /// Use a shallow (--depth 1) clone when `--url` is given.
        #[arg(long, default_value_t = true)]
        shallow: bool,
        /// After reporting, file one GitHub issue per violation via `gh`.
        #[arg(long)]
        create_issues: bool,
        /// Cap the issue-filing agent's iteration budget (with `--create-issues`).
        #[arg(long, default_value_t = 30)]
        max_turns: u32,
        /// Skip AI calls — produces a deterministic check with no quality review.
        #[arg(long)]
        no_ai: bool,
        /// After AI verification, automatically fix all findings via a zag agent.
        #[arg(long)]
        fix: bool,
    },
    /// Bring an existing repo into OSS_SPEC.md conformance via a zag agent.
    /// Without flags: edits files in place to remove every §19 violation.
    /// With --create-issues: instead files one GitHub issue per violation
    /// cluster via `gh` (no source files are touched).
    Fix {
        /// Repo to fix. Defaults to the current directory.
        #[arg(long, default_value = ".", conflicts_with = "url")]
        path: PathBuf,
        /// Instead of fixing in place, file one GitHub issue per violation.
        #[arg(long)]
        create_issues: bool,
        /// Cap the agent's iteration budget.
        #[arg(long, default_value_t = 30)]
        max_turns: u32,
        /// Fix (or file issues for) a remote git repo by cloning it first.
        /// Requires `--create-issues` — in-place fixes on an ephemeral clone
        /// would be discarded.
        #[arg(long, value_name = "URL")]
        url: Option<String>,
        /// Use a shallow (--depth 1) clone when `--url` is given.
        #[arg(long, default_value_t = true)]
        shallow: bool,
        /// Assume defaults for every interactive prompt.
        #[arg(long, short = 'y')]
        yes: bool,
        /// Skip AI calls.
        #[arg(long)]
        no_ai: bool,
    },
    /// Clone the public oss-spec repository into a local directory so a coding
    /// agent (or you) can browse OSS_SPEC.md, the templates, and the dogfood
    /// implementation locally as a reference.
    Fetch {
        /// Where to clone into. Defaults to a fresh subdirectory under the
        /// system temp dir; the resolved path is printed on stdout.
        #[arg(long)]
        into: Option<PathBuf>,
        /// Override the upstream repository URL.
        #[arg(
            long,
            default_value = "https://github.com/niclaslindstedt/oss-spec.git"
        )]
        url: String,
        /// Use a shallow (--depth 1) clone.
        #[arg(long, default_value_t = true)]
        shallow: bool,
    },
    /// List CLI commands in stable, machine-readable form (§12.4).
    Commands {
        /// Optional command name for a single-command spec.
        name: Option<String>,
        /// Show example invocations alongside the spec.
        #[arg(long)]
        examples: bool,
    },
    /// Print an embedded docs/ topic (§12.3). No arg lists topics.
    Docs { topic: Option<String> },
    /// Print an embedded manpage (§12.3). No arg lists commands.
    Man { command: Option<String> },
}

/// Top-level dispatcher called from `lib::run`.
pub async fn dispatch(cli: Cli) -> Result<()> {
    if cli.help_agent {
        println!("{}", crate::agent_help::HELP_AGENT);
        return Ok(());
    }
    if cli.debug_agent {
        println!("{}", crate::agent_help::DEBUG_AGENT);
        return Ok(());
    }

    log::debug!("dispatch: command={:?}, debug={}", cli.command, cli.debug);

    match cli.command.clone() {
        Some(Command::Commands { name, examples }) => {
            crate::agent_help::print_commands(name.as_deref(), examples);
            Ok(())
        }
        Some(Command::Docs { topic }) => {
            crate::agent_help::print_docs(topic.as_deref());
            Ok(())
        }
        Some(Command::Man { command }) => {
            crate::agent_help::print_man(command.as_deref());
            Ok(())
        }
        Some(Command::Fetch { into, url, shallow }) => {
            let dest = crate::git::fetch_oss_spec(&url, into.as_deref(), shallow)?;
            println!("{}", dest.display());
            Ok(())
        }
        Some(Command::Validate {
            path,
            url,
            shallow,
            create_issues,
            max_turns,
            no_ai,
            fix,
        }) => {
            let (target, cleanup) = match url {
                Some(u) => (
                    crate::git::clone_repo(&u, None, shallow, "oss-spec-validate")?,
                    true,
                ),
                None => (path, false),
            };
            let validate_result = crate::validate::run(&target);
            let mut report = match validate_result {
                Ok(r) => r,
                Err(e) => {
                    if cleanup {
                        let _ = std::fs::remove_dir_all(&target);
                    }
                    return Err(e);
                }
            };

            // AI quality verification (skip with --no-ai). A zag failure here
            // is fatal: we exit early with the error so the user sees what
            // went wrong (and where the debug log lives) instead of a silent
            // success that masks a broken AI path.
            if !no_ai {
                let file_contents = crate::validate::gather_file_contents(&target);
                if !file_contents.is_empty() {
                    match crate::ai::verify_conformance(&file_contents, &report.violations).await {
                        Ok(findings) => {
                            report.ai_findings = findings;
                        }
                        Err(e) => {
                            if cleanup {
                                let _ = std::fs::remove_dir_all(&target);
                            }
                            return Err(e);
                        }
                    }
                }
            }

            report.print();

            // --fix: hand the full report to the fix agent.
            if fix && (!report.is_clean() || !report.ai_findings.is_empty()) {
                let fix_result = crate::ai::fix_conformance(&target, &report, 30).await;
                if cleanup {
                    let _ = std::fs::remove_dir_all(&target);
                }
                return fix_result;
            }

            if create_issues && !report.is_clean() {
                let ai_result =
                    crate::ai::file_conformance_issues(&target, &report, max_turns).await;
                if cleanup {
                    let _ = std::fs::remove_dir_all(&target);
                }
                return ai_result;
            }

            if cleanup {
                let _ = std::fs::remove_dir_all(&target);
            }
            if !report.is_clean() {
                std::process::exit(1);
            }
            Ok(())
        }
        Some(Command::Fix {
            path,
            create_issues,
            max_turns,
            url,
            shallow,
            yes,
            no_ai: _,
        }) => {
            let (target, cleanup) = match url {
                Some(u) => {
                    if !create_issues {
                        anyhow::bail!(
                            "--url requires --create-issues; in-place fixes on a temp clone would be discarded"
                        );
                    }
                    (
                        crate::git::clone_repo(&u, None, shallow, "oss-spec-fix")?,
                        true,
                    )
                }
                None => (path, false),
            };
            let result = crate::fix::run(&target, create_issues, max_turns, yes).await;
            if cleanup {
                let _ = std::fs::remove_dir_all(&target);
            }
            result
        }
        Some(Command::Init {
            prompt,
            description,
            name,
            opts,
        }) => {
            let target = resolve_target_dir(&opts, name.as_deref())?;
            let manifest = if let Some(prompt) = prompt {
                log::debug!("init prompt flow: prompt={prompt:?}");
                let mut m = crate::interview::from_prompt(&opts, &prompt, name.as_deref()).await?;
                // Without an explicit --name, default to the target dir's name.
                if name.is_none() {
                    if let Some(dir_name) =
                        target.file_name().map(|s| s.to_string_lossy().into_owned())
                    {
                        m.name = dir_name;
                    }
                }
                m
            } else {
                crate::interview::run(&opts, name, description, true).await?
            };
            crate::bootstrap::write(&manifest, &target)?;
            post_bootstrap(&opts, &manifest, &target).await?;
            Ok(())
        }
        None => {
            anyhow::bail!("no subcommand given — try `oss-spec --help`");
        }
    }
}

fn resolve_target_dir(opts: &BootstrapOpts, name: Option<&str>) -> Result<PathBuf> {
    let parent = opts
        .path
        .clone()
        .or_else(|| std::env::current_dir().ok())
        .context("cannot determine target directory")?;
    Ok(match name {
        Some(n) => parent.join(n),
        None => parent,
    })
}

async fn post_bootstrap(
    opts: &BootstrapOpts,
    manifest: &crate::manifest::ProjectManifest,
    target: &std::path::Path,
) -> Result<()> {
    if !opts.no_git {
        crate::git::init_and_commit(target)?;
    }
    if !opts.no_gh {
        crate::git::gh_create(
            target,
            &manifest.github_owner,
            &manifest.name,
            opts.visibility.as_deref().unwrap_or("public"),
            opts.yes,
        )?;
    }
    crate::output::status(&format!(
        "bootstrapped {} at {}",
        manifest.name,
        target.display()
    ));
    Ok(())
}
