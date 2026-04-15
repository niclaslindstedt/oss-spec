//! §12 — CLI discoverability contract.
//!
//! `--help-agent`, `--debug-agent`, `commands [<name>] [--examples]`,
//! `docs [<topic>]`, and `man [<command>]`. All output is plain text on stdout
//! with no ANSI escapes, suitable for prompt injection.

use include_dir::{Dir, include_dir};

pub static EMBEDDED_DOCS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/docs");
pub static EMBEDDED_MAN: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/man");

pub const HELP_AGENT: &str = include_str!("../docs/agent/help-agent.txt");
pub const DEBUG_AGENT: &str = include_str!("../docs/agent/debug-agent.txt");

/// Stable, machine-parseable command index.
const COMMANDS_TABLE: &[(&str, &str)] = &[
    (
        "default",
        "oss-spec <prompt>                             # interpret + bootstrap via zag",
    ),
    (
        "new",
        "oss-spec new <name> [--lang ..] [--kind ..]    # explicit bootstrap",
    ),
    (
        "init",
        "oss-spec init                                  # bootstrap into current dir",
    ),
    (
        "check",
        "oss-spec check [--path .] [--url <URL>] [--create-issues]    # validate local or remote repo",
    ),
    (
        "fix",
        "oss-spec fix   [--path .] [--url <URL>] [--create-issues]    # fix or file issues for §19 violations",
    ),
    (
        "fetch",
        "oss-spec fetch [--into <dir>]                  # clone the public oss-spec repo locally",
    ),
    (
        "commands",
        "oss-spec commands [<name>] [--examples]        # this discovery surface",
    ),
    (
        "docs",
        "oss-spec docs [<topic>]                        # embedded docs/ topics",
    ),
    (
        "man",
        "oss-spec man [<command>]                       # embedded manpages",
    ),
];

const COMMAND_SPECS: &[(&str, &str)] = &[
    (
        "default",
        "oss-spec <PROMPT>\n\
         \n\
         Freeform prompt mode. The string is sent to the zag library, which\n\
         returns a structured manifest (name, language, kind, license, why\n\
         bullets). After confirmation, the bootstrap engine writes a full\n\
         OSS_SPEC.md-compliant repo.\n\
         \n\
         Common flags: --no-ai --no-git --no-gh --yes --path <dir>\n\
                       --name --lang --kind --license --visibility\n",
    ),
    (
        "new",
        "oss-spec new <NAME> [-d <DESCRIPTION>]\n\
         \n\
         Explicit bootstrap. NAME becomes both the directory name and the\n\
         project name. Skips AI prompt interpretation; still uses AI to draft\n\
         README 'Why?' bullets unless --no-ai is set.\n\
         \n\
         Flags: --lang rust|python|node|go|generic\n\
                --kind lib|cli|service\n\
                --license MIT|Apache-2.0|MPL-2.0\n\
                --visibility public|private\n\
                --no-ai --no-git --no-gh --yes --path <dir>\n",
    ),
    (
        "init",
        "oss-spec init [-d <DESCRIPTION>]\n\
         \n\
         Bootstrap into the current working directory. Existing files are\n\
         overwritten. Use this to convert an in-progress repo into an\n\
         OSS_SPEC.md-compliant one.\n",
    ),
    (
        "check",
        "oss-spec check [--path .] [--url <URL>] [--shallow] [--no-ai] [--create-issues] [--max-turns N] [--fix]\n\
         \n\
         Walks the target repo and reports every §19 checklist item that is\n\
         missing or malformed. Exits 1 on any violation, 0 if clean.\n\
         \n\
         With --url: clones the given git URL into a temp directory first,\n\
         validates the clone, then removes it. --path and --url are mutually\n\
         exclusive. Shallow clones (--depth 1) are the default.\n\
         \n\
         With --create-issues: after reporting, dispatches the same zag\n\
         agent as `fix --create-issues` to file one well-scoped GitHub\n\
         issue per violation cluster via `gh issue create`. When combined\n\
         with --url, the clone's `origin` remote is the source repo, so\n\
         issues land on the real upstream.\n",
    ),
    (
        "fix",
        "oss-spec fix [--path .] [--url <URL>] [--shallow] [--create-issues] [--max-turns N] [--yes] [--no-ai]\n\
         \n\
         Runs `check` against the target repo, then dispatches a zag-driven\n\
         agent to bring it into conformance.\n\
         \n\
         Without --create-issues: the agent edits files in place to remove\n\
         every §19 violation. It may create files, edit files, create\n\
         symlinks, and run shell commands inside the target directory.\n\
         \n\
         With --create-issues: the agent files one well-scoped GitHub issue\n\
         per violation cluster via `gh issue create` and does not modify\n\
         any source files.\n\
         \n\
         With --url: clones the given git URL into a temp directory first\n\
         and runs against the clone, then removes it. Requires\n\
         --create-issues, since an in-place fix on an ephemeral clone would\n\
         be discarded. --path and --url are mutually exclusive.\n\
         \n\
         The agent's prompts live in prompts/fix-conformance/ and\n\
         prompts/file-conformance-issues/ (versioned per the prompts\n\
         convention in OSS_SPEC.md). Use --max-turns to cap iterations\n\
         (default 30). Use --yes to skip the confirmation prompt.\n",
    ),
    (
        "fetch",
        "oss-spec fetch [--into <DIR>] [--url <URL>] [--shallow]\n\
         \n\
         Clones the public oss-spec repository into a local directory so an\n\
         AI coding agent (or you) can browse OSS_SPEC.md, the embedded\n\
         template tree, and the dogfood implementation as a reference. Prints\n\
         the resolved path on stdout. Defaults to a unique temp directory and\n\
         a shallow clone.\n",
    ),
    (
        "commands",
        "oss-spec commands [<NAME>] [--examples]\n\
         \n\
         Without args: prints the stable command index (one per line).\n\
         With NAME:    prints the spec for that command.\n\
         With --examples: prints a realistic invocation for each command.\n",
    ),
    (
        "docs",
        "oss-spec docs [<TOPIC>]\n\
         \n\
         Without args: lists embedded docs/ topics.\n\
         With TOPIC:   prints docs/<TOPIC>.md to stdout.\n",
    ),
    (
        "man",
        "oss-spec man [<COMMAND>]\n\
         \n\
         Without args: lists embedded man/<command>.md files.\n\
         With COMMAND: prints man/<COMMAND>.md to stdout.\n",
    ),
];

const EXAMPLES: &[(&str, &str)] = &[
    (
        "default",
        "oss-spec \"create a python cli for finding stock buys\"",
    ),
    (
        "new",
        "oss-spec new my-tool --lang rust --kind cli --license MIT --no-ai --yes",
    ),
    ("init", "cd existing-repo && oss-spec init --no-ai --yes"),
    (
        "check",
        "oss-spec check --url https://github.com/niclaslindstedt/oss-spec.git",
    ),
    (
        "fix",
        "oss-spec fix --url https://github.com/niclaslindstedt/oss-spec.git --create-issues --yes",
    ),
    ("fetch", "oss-spec fetch --into /tmp/oss-spec-ref"),
    ("commands", "oss-spec commands --examples"),
    ("docs", "oss-spec docs getting-started"),
    ("man", "oss-spec man oss-spec"),
];

pub fn print_commands(name: Option<&str>, examples: bool) {
    if let Some(n) = name {
        if let Some((_, spec)) = COMMAND_SPECS.iter().find(|(k, _)| *k == n) {
            print!("{spec}");
            if examples {
                if let Some((_, ex)) = EXAMPLES.iter().find(|(k, _)| *k == n) {
                    println!("\nexample:\n  $ {ex}");
                }
            }
        } else {
            eprintln!("unknown command: {n}");
            std::process::exit(2);
        }
        return;
    }
    if examples {
        for (k, ex) in EXAMPLES {
            println!("{k:<10} $ {ex}");
        }
        return;
    }
    for (_, line) in COMMANDS_TABLE {
        println!("{line}");
    }
}

pub fn print_docs(topic: Option<&str>) {
    let docs = &EMBEDDED_DOCS;
    match topic {
        None => list_md(docs, "topics"),
        Some(t) => print_md(docs, t),
    }
}

pub fn print_man(command: Option<&str>) {
    let man = &EMBEDDED_MAN;
    match command {
        None => list_md(man, "manpages"),
        Some(c) => print_md(man, c),
    }
}

fn list_md(dir: &Dir<'_>, label: &str) {
    println!("available {label}:");
    let mut names: Vec<&str> = Vec::new();
    collect_md(dir, &mut names);
    names.sort();
    for n in names {
        println!("  {n}");
    }
}

fn collect_md<'a>(dir: &'a Dir<'_>, out: &mut Vec<&'a str>) {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                if let Some(name) = f.path().file_stem().and_then(|s| s.to_str())
                    && f.path().extension().and_then(|s| s.to_str()) == Some("md")
                {
                    out.push(name);
                }
            }
            include_dir::DirEntry::Dir(d) => collect_md(d, out),
        }
    }
}

fn print_md(dir: &Dir<'_>, name: &str) {
    let target = format!("{name}.md");
    if let Some(file) = find_file(dir, &target) {
        if let Ok(s) = std::str::from_utf8(file.contents()) {
            print!("{s}");
            return;
        }
    }
    eprintln!("not found: {target}");
    std::process::exit(2);
}

fn find_file<'a>(dir: &'a Dir<'_>, name: &str) -> Option<&'a include_dir::File<'a>> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                if f.path().file_name().and_then(|s| s.to_str()) == Some(name) {
                    return Some(f);
                }
            }
            include_dir::DirEntry::Dir(d) => {
                if let Some(f) = find_file(d, name) {
                    return Some(f);
                }
            }
        }
    }
    None
}
