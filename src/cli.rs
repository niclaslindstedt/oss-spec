//! Clap definitions for the `oss-spec` binary.
//!
//! The default invocation is the freeform-prompt form:
//!
//! ```text
//! oss-spec "create a python cli for finding stock buys"
//! ```
//!
//! which routes through `ai::interpret_prompt` → manifest → bootstrap. Explicit
//! subcommands (`new`, `init`, `check`, `commands`, `docs`, `man`) cover the
//! deterministic / power-user paths and the §12 CLI discoverability contract.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "oss-spec",
    version,
    about = "Bootstrap new OSS_SPEC.md-compliant repositories",
    long_about = "oss-spec materializes new open source repositories that follow the conventions in OSS_SPEC.md (LICENSE, README, AGENTS.md + symlinks, CI workflows, docs, examples, website, CLI agent contract). The default form takes a freeform prompt and uses the zag library to interpret it into a project manifest."
)]
pub struct Cli {
    /// Freeform prompt — e.g. "create a python cli for finding stock buys".
    /// When given (and no subcommand), oss-spec interprets it via zag and runs
    /// the bootstrap flow.
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    /// Print a plain-text dump suitable for prompt injection (§12.1).
    #[arg(long, exclusive = true)]
    pub help_agent: bool,

    /// Print a plain-text troubleshooting dump (§12.2).
    #[arg(long, exclusive = true)]
    pub debug_agent: bool,

    /// Enable debug-level output on stdout and verbose file logging.
    #[arg(long, global = true)]
    pub debug: bool,

    /// Skip AI calls — produces a deterministic skeleton from defaults/flags only.
    #[arg(long, global = true)]
    pub no_ai: bool,

    /// Skip `git init` and the first commit.
    #[arg(long, global = true)]
    pub no_git: bool,

    /// Skip `gh repo create`.
    #[arg(long, global = true)]
    pub no_gh: bool,

    /// Assume defaults for every interactive prompt.
    #[arg(long, short = 'y', global = true)]
    pub yes: bool,

    /// Override the parent directory the new repo gets created in.
    #[arg(long, global = true, value_name = "DIR")]
    pub path: Option<PathBuf>,

    /// Override the project name.
    #[arg(long, global = true)]
    pub name: Option<String>,

    /// Override language: rust|python|node|go|generic.
    #[arg(long, global = true)]
    pub lang: Option<String>,

    /// Override kind: lib|cli|service.
    #[arg(long, global = true)]
    pub kind: Option<String>,

    /// Override license: MIT|Apache-2.0|MPL-2.0.
    #[arg(long, global = true)]
    pub license: Option<String>,

    /// gh visibility: public|private. Defaults to public.
    #[arg(long, global = true)]
    pub visibility: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Bootstrap a new project at <name> (or --path/<name>).
    New {
        /// Project name (kebab-case).
        name: String,
        /// Description; if omitted you will be prompted (or AI will infer).
        #[arg(long, short = 'd')]
        description: Option<String>,
    },
    /// Bootstrap into the current directory (must be empty or contain only OSS_SPEC.md).
    Init {
        #[arg(long, short = 'd')]
        description: Option<String>,
    },
    /// Validate an existing repo against the §19 checklist. With `--url`,
    /// clones a remote repo into a temp dir first. With `--create-issues`,
    /// files one GitHub issue per violation after reporting.
    Check {
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
        Some(Command::Check {
            path,
            url,
            shallow,
            create_issues,
            max_turns,
        }) => {
            let (target, cleanup) = match url {
                Some(u) => (
                    crate::git::clone_repo(&u, None, shallow, "oss-spec-check")?,
                    true,
                ),
                None => (path, false),
            };
            let check_result = crate::check::run(&target);
            let report = match check_result {
                Ok(r) => r,
                Err(e) => {
                    if cleanup {
                        let _ = std::fs::remove_dir_all(&target);
                    }
                    return Err(e);
                }
            };
            report.print();

            if create_issues && !report.is_clean() {
                // Same code path as `fix --create-issues`. Issues land on the
                // clone's `origin` remote, which (when cloned via --url) is
                // the real source GitHub repo.
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
            let result = crate::fix::run(&target, create_issues, max_turns, cli.yes).await;
            if cleanup {
                let _ = std::fs::remove_dir_all(&target);
            }
            result
        }
        Some(Command::New { name, description }) => {
            let target = resolve_target_dir(&cli, Some(&name))?;
            let manifest =
                crate::interview::run(&cli, Some(name.clone()), description, false).await?;
            crate::bootstrap::write(&manifest, &target)?;
            post_bootstrap(&cli, &manifest, &target).await?;
            Ok(())
        }
        Some(Command::Init { description }) => {
            let target = std::env::current_dir().context("cannot read current directory")?;
            let manifest = crate::interview::run(&cli, cli.name.clone(), description, true).await?;
            crate::bootstrap::write(&manifest, &target)?;
            post_bootstrap(&cli, &manifest, &target).await?;
            Ok(())
        }
        None => {
            // Default form: positional prompt → AI interpret → bootstrap.
            let prompt = cli
                .prompt
                .clone()
                .context("no prompt and no subcommand — try `oss-spec --help`")?;
            let manifest = crate::interview::from_prompt(&cli, &prompt).await?;
            let target = resolve_target_dir(&cli, Some(&manifest.name))?;
            crate::bootstrap::write(&manifest, &target)?;
            post_bootstrap(&cli, &manifest, &target).await?;
            Ok(())
        }
    }
}

fn resolve_target_dir(cli: &Cli, name: Option<&str>) -> Result<PathBuf> {
    let parent = cli
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
    cli: &Cli,
    manifest: &crate::manifest::ProjectManifest,
    target: &std::path::Path,
) -> Result<()> {
    if !cli.no_git {
        crate::git::init_and_commit(target)?;
    }
    if !cli.no_gh {
        crate::git::gh_create(
            target,
            &manifest.github_owner,
            &manifest.name,
            cli.visibility.as_deref().unwrap_or("public"),
            cli.yes,
        )?;
    }
    crate::output::status(&format!(
        "bootstrapped {} at {}",
        manifest.name,
        target.display()
    ));
    Ok(())
}
