#!/usr/bin/env bash
# scripts/validate.sh — language-agnostic OSS_SPEC.md conformance checker.
#
# Use this when you cannot install the `oss-spec` Rust binary (sandboxed
# agent sessions, CI runners without cargo, etc.). The script is a 1:1
# mirror of the deterministic, non-AI checks performed by the Rust
# validator under src/validate/. AI quality checks are emitted at the end
# as a manual checklist for the calling agent.
#
# IMPORTANT FOR MAINTAINERS:
#   This script MUST stay in lockstep with src/validate/ (structural.rs,
#   content.rs, toolchain.rs, agent_skills.rs). Whenever you change a §19
#   rule on either side, mirror the change here. The self-conformance test
#   in tests/self_conformance.rs only exercises the Rust path; drift
#   between the two implementations is not caught by CI, so reviews must
#   verify parity by hand.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/niclaslindstedt/oss-spec/main/scripts/validate.sh | bash -s -- [<path>]
#   ./scripts/validate.sh [<path>]
#
#   <path> defaults to the current working directory.
#
# Exit codes:
#   0  — no structural violations (manual checklist may still apply).
#   1  — one or more structural violations were reported.
#   2  — the script could not run (bad path, missing tools).

set -euo pipefail

SPEC_URL="https://raw.githubusercontent.com/niclaslindstedt/oss-spec/main/OSS_SPEC.md"
SPEC_VERSION="2.6.0"

# The agent-prompt body lives at prompts/validate-sh-agent/<v>.md per §13.5.
# Bump this URL whenever a new version is added to that directory; the
# `update-prompts` skill is responsible for keeping the bash script and
# the prompt file in lockstep.
PROMPT_URL="https://raw.githubusercontent.com/niclaslindstedt/oss-spec/main/prompts/validate-sh-agent/1_0_0.md"
PROMPT_VERSION="1.0.0"

# ---------------------------------------------------------------------------
# Output helpers
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    C_ERR=$'\033[31m'; C_OK=$'\033[32m'; C_WARN=$'\033[33m'
    C_INFO=$'\033[36m'; C_RST=$'\033[0m'
else
    C_ERR=''; C_OK=''; C_WARN=''; C_INFO=''; C_RST=''
fi

info()   { printf '%s\n' "${C_INFO}$*${C_RST}"; }
ok()     { printf '%s\n' "${C_OK}$*${C_RST}"; }
warn()   { printf '%s\n' "${C_WARN}$*${C_RST}" >&2; }
err()    { printf '%s\n' "${C_ERR}$*${C_RST}" >&2; }
header() { printf '\n%s\n' "${C_INFO}== $* ==${C_RST}"; }

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
TARGET="${1:-.}"
if [ ! -d "$TARGET" ]; then
    err "validate.sh: '$TARGET' is not a directory"
    exit 2
fi
TARGET="$(cd "$TARGET" && pwd)"

# ---------------------------------------------------------------------------
# Spec download — always pulled fresh into the repo root and overwritten.
#
# We never trust a pre-existing OSS_SPEC.md on disk: the spec evolves and
# a stale local copy will silently mask new mandates. Either we get the
# current upstream version or we treat the spec as unavailable. Writing
# to the repo root (rather than a temp file) lets the calling agent run
# `git diff OSS_SPEC.md` to see exactly what changed since the project
# was last brought into conformance.
# ---------------------------------------------------------------------------
SPEC_PATH="$TARGET/OSS_SPEC.md"
SPEC_PREEXISTING=0
SPEC_DOWNLOADED=0
[ -f "$SPEC_PATH" ] && SPEC_PREEXISTING=1

if command -v curl >/dev/null 2>&1; then
    if curl -fsSL "$SPEC_URL" -o "$SPEC_PATH" 2>/dev/null; then
        SPEC_DOWNLOADED=1
    fi
elif command -v wget >/dev/null 2>&1; then
    if wget -q "$SPEC_URL" -O "$SPEC_PATH" 2>/dev/null; then
        SPEC_DOWNLOADED=1
    fi
fi

# A stale local OSS_SPEC.md is worse than no spec — the agent would silently
# review against last month's mandates. If the fresh download failed, drop
# the pointer entirely so the prompt below makes the agent fetch it manually.
if [ "$SPEC_DOWNLOADED" -eq 0 ]; then
    SPEC_PATH=""
fi

# Detect whether the freshly-downloaded copy actually differs from what was
# previously committed. We only nag the agent to `git diff -- OSS_SPEC.md`
# when there is a real diff to look at.
#
# Values:
#   yes      — pre-existing tracked file, upstream differs from HEAD
#   no       — pre-existing tracked file, upstream matches HEAD (no-op overwrite)
#   unknown  — not in a git repo, or file not tracked yet, or git unavailable
#   fresh    — no pre-existing copy on disk
SPEC_CHANGED="unknown"
if [ "$SPEC_DOWNLOADED" -eq 1 ]; then
    if [ "$SPEC_PREEXISTING" -eq 0 ]; then
        SPEC_CHANGED="fresh"
    elif command -v git >/dev/null 2>&1 \
         && git -C "$TARGET" rev-parse --git-dir >/dev/null 2>&1 \
         && git -C "$TARGET" ls-files --error-unmatch OSS_SPEC.md >/dev/null 2>&1; then
        if git -C "$TARGET" diff --quiet HEAD -- OSS_SPEC.md 2>/dev/null; then
            SPEC_CHANGED="no"
        else
            SPEC_CHANGED="yes"
        fi
    fi
fi

# Agent-prompt download (best-effort). Stored in a temp file rather than the
# target repo because, unlike OSS_SPEC.md, this is not an artifact the agent
# is expected to commit or diff against. The body lives at
# prompts/validate-sh-agent/<v>.md per §13.5; this script renders the
# `## User` section with placeholders substituted at run time.
PROMPT_FILE=""
# Prefer a local copy when running against a checkout that already ships the
# prompt (the oss-spec repo itself, or a project that vendors the prompts/
# tree). Pick the highest-versioned <major>_<minor>_<patch>.md so the
# loader stays consistent with §13.5.
_local_prompt_dir="$TARGET/prompts/validate-sh-agent"
if [ -d "$_local_prompt_dir" ]; then
    _local_prompt="$(find "$_local_prompt_dir" -maxdepth 1 -type f -name '[0-9]*_[0-9]*_[0-9]*.md' \
                       2>/dev/null | sort -V | tail -n 1)"
    if [ -n "$_local_prompt" ] && [ -s "$_local_prompt" ]; then
        PROMPT_FILE="$_local_prompt"
    fi
fi
if [ -z "$PROMPT_FILE" ]; then
    _prompt_tmp="${TMPDIR:-/tmp}/_oss_spec_validate_agent_prompt.md"
    if command -v curl >/dev/null 2>&1; then
        if curl -fsSL "$PROMPT_URL" -o "$_prompt_tmp" 2>/dev/null; then
            PROMPT_FILE="$_prompt_tmp"
        fi
    elif command -v wget >/dev/null 2>&1; then
        if wget -q "$PROMPT_URL" -O "$_prompt_tmp" 2>/dev/null; then
            PROMPT_FILE="$_prompt_tmp"
        fi
    fi
fi

# ---------------------------------------------------------------------------
# Violation buffer
# ---------------------------------------------------------------------------
VIOLATIONS=()
add_violation() {
    # $1 = spec section (e.g. "§19"), $2... = message
    local sec="$1"; shift
    VIOLATIONS+=("[$sec] $*")
}

# ---------------------------------------------------------------------------
# Version comparison helpers (mirrors src/validate/toolchain.rs)
# ---------------------------------------------------------------------------
is_floating_specifier() {
    case "$1" in
        stable|latest|lts|"lts/*"|"*") return 0 ;;
        *) return 1 ;;
    esac
}

# version_ge "1.88.0" "1.85" → 0 (true); shorter sides are zero-padded.
version_ge() {
    local a="$1" b="$2"
    if ! [[ "$a" =~ ^[0-9]+(\.[0-9]+)*$ ]] || ! [[ "$b" =~ ^[0-9]+(\.[0-9]+)*$ ]]; then
        return 2
    fi
    local highest
    highest="$(printf '%s\n%s\n' "$a" "$b" | sort -V | tail -n 1)"
    [ "$a" = "$highest" ]
}

versions_same_major_minor() {
    local a_mm b_mm
    a_mm="$(printf '%s' "$1" | awk -F. '{printf "%s.%s", $1, $2}')"
    b_mm="$(printf '%s' "$2" | awk -F. '{printf "%s.%s", $1, $2}')"
    [ "$a_mm" = "$b_mm" ]
}

# ---------------------------------------------------------------------------
# Structural checks (mirrors src/validate/structural.rs)
# ---------------------------------------------------------------------------
check_required_files() {
    # (file, section)
    local entries=(
        "LICENSE|§2"
        "README.md|§3"
        "CONTRIBUTING.md|§4"
        "CODE_OF_CONDUCT.md|§5"
        "SECURITY.md|§6"
        "AGENTS.md|§7"
        "CHANGELOG.md|§8.4"
        ".gitignore|§19"
        ".editorconfig|§19"
        "Makefile|§9"
    )
    local e f sec
    for e in "${entries[@]}"; do
        f="${e%%|*}"; sec="${e##*|}"
        if [ ! -e "$TARGET/$f" ]; then
            add_violation "$sec" "missing $f"
        fi
    done
}

check_agents_symlinks() {
    local links=(
        "CLAUDE.md"
        ".cursorrules"
        ".windsurfrules"
        "GEMINI.md"
        ".github/copilot-instructions.md"
    )
    local l
    for l in "${links[@]}"; do
        if [ ! -L "$TARGET/$l" ]; then
            add_violation "§7.1" "$l must be a symlink to AGENTS.md"
        fi
    done
}

check_required_dirs() {
    local entries=(
        ".github/workflows|§10.1"
        ".github/ISSUE_TEMPLATE|§15"
        "docs|§11.1"
        "prompts|§13.5"
        "scripts|§10.3"
    )
    local e d sec
    for e in "${entries[@]}"; do
        d="${e%%|*}"; sec="${e##*|}"
        if [ ! -d "$TARGET/$d" ]; then
            add_violation "$sec" "missing directory $d"
        fi
    done
}

check_prompts_versioned() {
    local root="$TARGET/prompts"
    [ -d "$root" ] || return 0
    local sub name has_versioned f base
    while IFS= read -r -d '' sub; do
        [ -d "$sub" ] || continue
        name="$(basename "$sub")"
        has_versioned=0
        for f in "$sub"/*.md; do
            [ -f "$f" ] || continue
            base="$(basename "$f" .md)"
            if [[ "$base" =~ ^[0-9]+_[0-9]+_[0-9]+$ ]]; then
                has_versioned=1
                break
            fi
        done
        if [ "$has_versioned" -eq 0 ]; then
            add_violation "§13.5" "prompts/$name/ has no versioned <major>_<minor>_<patch>.md file"
        fi
    done < <(find "$root" -mindepth 1 -maxdepth 1 -type d -print0 2>/dev/null)
}

check_workflows() {
    local required=(ci.yml version-bump.yml release.yml pages.yml)
    local w
    for w in "${required[@]}"; do
        if [ ! -e "$TARGET/.github/workflows/$w" ]; then
            add_violation "§10" "missing .github/workflows/$w"
        fi
    done
}

check_issue_pr_templates() {
    local files=(
        ".github/PULL_REQUEST_TEMPLATE.md"
        ".github/ISSUE_TEMPLATE/bug_report.md"
        ".github/ISSUE_TEMPLATE/feature_request.md"
        ".github/ISSUE_TEMPLATE/config.yml"
        ".github/dependabot.yml"
    )
    local f
    for f in "${files[@]}"; do
        if [ ! -e "$TARGET/$f" ]; then
            add_violation "§15" "missing $f"
        fi
    done
}

check_test_naming() {
    local d="$TARGET/tests"
    [ -d "$d" ] || return 0
    local f stem
    while IFS= read -r -d '' f; do
        stem="$(basename "$f")"; stem="${stem%.*}"
        case "$stem" in
            *_test|*_tests|*Test|*Tests) ;;
            *) add_violation "§20.2" \
                "tests/$(basename "$f"): file stem '$stem' does not end with _test, _tests, Test, or Tests" ;;
        esac
    done < <(find "$d" -mindepth 1 -maxdepth 1 -type f -print0 2>/dev/null)
}

# ---------------------------------------------------------------------------
# Content checks (mirrors src/validate/content.rs)
# ---------------------------------------------------------------------------
check_output_module() {
    # §19.4 Central output module (only when src/ or lib/ exists).
    if [ ! -d "$TARGET/src" ] && [ ! -d "$TARGET/lib" ]; then
        return 0
    fi
    local file_candidates=(
        src/output.rs src/output.ts src/output.js src/output.mjs
        src/output.py src/output.go src/output.rb src/output.java
        src/output.kt src/output.swift src/output.cs
        lib/output.ts lib/output.js lib/output.mjs lib/output.py
    )
    local c
    for c in "${file_candidates[@]}"; do
        [ -f "$TARGET/$c" ] && return 0
    done
    local dir_candidates=(src/output lib/output internal/output)
    for c in "${dir_candidates[@]}"; do
        [ -d "$TARGET/$c" ] && return 0
    done
    add_violation "§19.4" \
        "missing central output module (expected e.g. src/output.rs with semantic helpers status/warn/info/header/error; see §19.4)"
}

check_no_inline_tests() {
    # §20 No inline #[cfg(test)] mod blocks in src/**/*.rs.
    local d="$TARGET/src"
    [ -d "$d" ] || return 0
    local f
    while IFS= read -r -d '' f; do
        if has_inline_test_attribute "$f"; then
            local rel="${f#"$TARGET"/}"
            add_violation "§20" \
                "$rel: contains inline test block; move tests to a separate file in tests/"
        fi
    done < <(find "$d" -type f -name '*.rs' -print0 2>/dev/null)
}

# Returns 0 (true) if the Rust source contains an inline `#[cfg(test)] mod x { ... }`.
# Lines that are comments or that use `mod tests;` (semicolon, external file) are ignored.
has_inline_test_attribute() {
    awk '
        /^[[:space:]]*\/\// { next }
        /^[[:space:]]*\/\*/ { next }
        /^[[:space:]]*\*/   { next }
        {
            line = $0
            sub(/^[[:space:]]+/, "", line)
            if (line ~ /^#\[cfg\(test\)\]/) {
                rest = line; sub(/^#\[cfg\(test\)\]/, "", rest); sub(/^[[:space:]]+/, "", rest)
                if (rest ~ /^mod[[:space:]][^;]*\{/) { print "INLINE"; exit }
                pending = 1; next
            }
            if (pending) {
                if (line == "" || line ~ /^#\[/) next
                if (line ~ /^mod[[:space:]][^;]*\{/) { print "INLINE"; exit }
                pending = 0
            }
        }
    ' "$1" | grep -q INLINE
}

check_source_file_size() {
    # §20.5 Source file size limit, with `oss-spec:allow-large-file: <reason>` opt-out.
    local roots=(src lib)
    local skip_dirs=(tests target node_modules .git .agent .claude dist build __pycache__ .venv venv)
    local prune=()
    local d_name
    for d_name in "${skip_dirs[@]}"; do
        prune+=( -name "$d_name" -o )
    done
    prune+=( -name __nope__ )

    local r d
    for r in "${roots[@]}"; do
        d="$TARGET/$r"
        [ -d "$d" ] || continue
        local f
        while IFS= read -r -d '' f; do
            walk_size_check "$f"
        done < <(find "$d" \( "${prune[@]}" \) -prune \
                       -o -type f \( -name '*.rs' -o -name '*.py' -o -name '*.ts' -o \
                                     -name '*.tsx' -o -name '*.js' -o -name '*.jsx' -o \
                                     -name '*.go' -o -name '*.java' -o -name '*.kt' -o \
                                     -name '*.cs' -o -name '*.swift' \) -print0 2>/dev/null)
    done
}

walk_size_check() {
    local f="$1"
    local stem; stem="$(basename "$f")"; stem="${stem%.*}"
    case "$stem" in
        *_test|*_tests|*Test|*Tests) return 0 ;;
    esac
    local lines; lines="$(wc -l < "$f" | tr -d ' ')"
    [ "$lines" -le 1000 ] && return 0
    if head -n 20 "$f" | grep -Eq 'oss-spec:allow-large-file:[[:space:]]*[^[:space:]]'; then
        return 0
    fi
    local rel="${f#"$TARGET"/}"
    add_violation "§20.5" \
        "$rel: $lines lines exceeds 1000-line limit; split the file or add an \`oss-spec:allow-large-file: <reason>\` marker in the first 20 lines"
}

check_website_seo() {
    # §11.3 SEO scaffolding (Open Graph, Twitter Card, JSON-LD, sitemap.xml, robots.txt).
    local has_website=0
    local seen_og=0 seen_tw=0 seen_jsonld=0 seen_sitemap=0 seen_robots=0
    local skip_dirs=(node_modules target dist build .git .agent .claude __pycache__ .venv venv)
    local prune=()
    local d_name
    for d_name in "${skip_dirs[@]}"; do
        prune+=( -name "$d_name" -o )
    done
    prune+=( -name __nope__ )

    local f
    while IFS= read -r -d '' f; do
        local base; base="$(basename "$f")"
        case "$base" in
            index.html|index.htm|index.html.tmpl) has_website=1 ;;
        esac
        if [[ "$f" == */.github/workflows/*.yml || "$f" == */.github/workflows/*.yaml ]]; then
            grep -q 'actions/deploy-pages' "$f" 2>/dev/null && has_website=1
        fi
        case "$base" in
            *.html|*.htm|*.js|*.ts|*.mjs|*.cjs|*.jsx|*.tsx|*.vue|*.svelte|*.tmpl)
                grep -q 'og:image' "$f" 2>/dev/null && seen_og=1
                grep -q 'twitter:card' "$f" 2>/dev/null && seen_tw=1
                grep -q 'application/ld+json' "$f" 2>/dev/null && seen_jsonld=1
                grep -q 'sitemap.xml' "$f" 2>/dev/null && seen_sitemap=1
                grep -q 'robots.txt' "$f" 2>/dev/null && seen_robots=1
                ;;
        esac
    done < <(find "$TARGET" \( "${prune[@]}" \) -prune \
                   -o -type f -print0 2>/dev/null)

    [ "$has_website" -eq 0 ] && return 0
    local missing=()
    [ "$seen_og" -eq 0 ]      && missing+=("Open Graph (og:image)")
    [ "$seen_tw" -eq 0 ]      && missing+=("Twitter Card (twitter:card)")
    [ "$seen_jsonld" -eq 0 ]  && missing+=("JSON-LD (application/ld+json)")
    [ "$seen_sitemap" -eq 0 ] && missing+=("sitemap.xml")
    [ "$seen_robots" -eq 0 ]  && missing+=("robots.txt")
    if [ "${#missing[@]}" -gt 0 ]; then
        local IFS=', '
        add_violation "§11.3" \
            "project has a website but its SEO scaffolding is incomplete; missing: ${missing[*]}"
    fi
}

# ---------------------------------------------------------------------------
# Toolchain checks (mirrors src/validate/toolchain.rs)
# ---------------------------------------------------------------------------
# Spec-defined minimums (§10.3). Keep aligned with MIN_TOOLCHAIN_VERSIONS in
# src/validate/toolchain.rs.
MIN_RUST="1.88.0"
MIN_PYTHON="3.12"
MIN_NODE="24"
MIN_GO="1.22"

check_workflow_toolchain_versions() {
    local file="$1" content="$2" lang spec
    while IFS= read -r line; do
        if [[ "$line" == *"dtolnay/rust-toolchain@"* ]]; then
            spec="${line#*dtolnay/rust-toolchain@}"
            spec="${spec%%[[:space:]#]*}"
            evaluate_toolchain "Rust" "$spec" "$file" "$MIN_RUST"
        fi
    done < <(printf '%s\n' "$content")

    eval_setup() {
        local needle="$1" lang="$2" key="$3" min="$4"
        local lines=()
        while IFS= read -r ln; do lines+=("$ln"); done < <(printf '%s\n' "$content")
        local i n=${#lines[@]}
        for (( i=0; i<n; i++ )); do
            if [[ "${lines[$i]}" == *"$needle"* ]]; then
                local j end=$((i+1+6))
                [ "$end" -gt "$n" ] && end="$n"
                for (( j=i+1; j<end; j++ )); do
                    local trimmed="${lines[$j]#"${lines[$j]%%[![:space:]]*}"}"
                    if [[ "$trimmed" == "$key:"* ]]; then
                        local val="${trimmed#*:}"
                        val="${val#"${val%%[![:space:]]*}"}"
                        val="${val%\"}"; val="${val#\"}"
                        val="${val%\'}"; val="${val#\'}"
                        evaluate_toolchain "$lang" "$val" "$file" "$min"
                        break
                    fi
                done
            fi
        done
    }
    eval_setup "actions/setup-python" "Python" "python-version" "$MIN_PYTHON"
    eval_setup "actions/setup-node"   "Node"   "node-version"   "$MIN_NODE"
    eval_setup "actions/setup-go"     "Go"     "go-version"     "$MIN_GO"
}

evaluate_toolchain() {
    local lang="$1" spec="$2" file="$3" min="$4"
    if is_floating_specifier "$spec"; then
        add_violation "§10.3" \
            ".github/workflows/$file: $lang toolchain uses floating specifier '$spec'; pin to >= $min"
        return
    fi
    if version_ge "$spec" "$min"; then
        return
    fi
    local rc=$?
    if [ "$rc" -eq 2 ]; then
        add_violation "§10.3" \
            ".github/workflows/$file: $lang toolchain has unparseable version '$spec'; pin to >= $min"
    else
        add_violation "§10.3" \
            ".github/workflows/$file: $lang toolchain pinned to $spec; minimum is $min"
    fi
}

check_ci_toolchains() {
    local f
    for f in ci.yml release.yml; do
        local p="$TARGET/.github/workflows/$f"
        [ -f "$p" ] || continue
        local content; content="$(cat "$p")"
        check_workflow_toolchain_versions "$f" "$content"
    done
}

# Extract `<key>: "<spec>"` (or unquoted) on the next 6 lines after a needle.
find_setup_version() {
    local content="$1" needle="$2" key="$3"
    awk -v needle="$needle" -v key="$key" '
        index($0, needle) > 0 { found=NR }
        found && NR > found && NR <= found+6 {
            line=$0; sub(/^[[:space:]]+/, "", line)
            if (index(line, key ":") == 1) {
                v=substr(line, length(key)+2); sub(/^[[:space:]]+/, "", v)
                gsub(/^["'\'']|["'\'']$/, "", v)
                print v; exit
            }
        }
    ' <<< "$content"
}

find_rust_ci_version() {
    awk '
        { idx=index($0, "dtolnay/rust-toolchain@")
          if (idx > 0) { rest=substr($0, idx+length("dtolnay/rust-toolchain@"))
                          sub(/[[:space:]#].*$/, "", rest); print rest; exit } }
    ' <<< "$1"
}

parse_rust_channel() {
    awk '
        /^[[:space:]]*\[/   { in_tc = ($0 ~ /^\[toolchain\]/) ; next }
        /^[[:space:]]*#/    { next }
        in_tc && /^[[:space:]]*channel[[:space:]]*=/ {
            sub(/^[[:space:]]*channel[[:space:]]*=[[:space:]]*/, "")
            sub(/[[:space:]]*#.*$/, "")
            gsub(/^["'\'']|["'\'']$/, "")
            print; exit
        }
    ' "$1"
}

parse_go_toolchain() {
    awk '
        /^[[:space:]]*toolchain[[:space:]]/ {
            sub(/^[[:space:]]*toolchain[[:space:]]+/, "")
            tok=$1; sub(/^go/, "", tok); print tok; exit
        }
    ' "$1"
}

check_local_pins() {
    local ci_yml=""
    [ -f "$TARGET/.github/workflows/ci.yml" ] && ci_yml="$(cat "$TARGET/.github/workflows/ci.yml")"

    if [ -f "$TARGET/Cargo.toml" ];          then check_rust_pin   "$ci_yml"; fi
    if [ -f "$TARGET/pyproject.toml" ] || [ -f "$TARGET/setup.py" ]; then
        check_python_pin "$ci_yml"
    fi
    if [ -f "$TARGET/package.json" ];        then check_node_pin   "$ci_yml"; fi
    if [ -f "$TARGET/go.mod" ];              then check_go_pin     "$ci_yml"; fi
}

check_rust_pin() {
    local ci_yml="$1"
    local pin="$TARGET/rust-toolchain.toml"
    if [ ! -f "$pin" ]; then
        add_violation "§10.5" "missing rust-toolchain.toml; pin the Rust channel at repo root (see §10.5)"
        return
    fi
    local channel; channel="$(parse_rust_channel "$pin")"
    if [ -z "$channel" ]; then
        add_violation "§10.5" "rust-toolchain.toml must declare [toolchain] channel = \"<x.y.z>\" (see §10.5)"
        return
    fi
    if is_floating_specifier "$channel"; then
        add_violation "§10.5" "rust-toolchain.toml channel '$channel' is a floating specifier; pin to an exact version (see §10.5)"
        return
    fi
    if [ -n "$ci_yml" ]; then
        local ci_ver; ci_ver="$(find_rust_ci_version "$ci_yml")"
        if [ -n "$ci_ver" ] && [ "$ci_ver" != "$channel" ]; then
            add_violation "§10.5" \
                "rust-toolchain.toml channel '$channel' does not match ci.yml dtolnay/rust-toolchain@$ci_ver (see §10.5)"
        fi
    fi
}

check_python_pin() {
    local ci_yml="$1"
    local pin="$TARGET/.python-version"
    if [ ! -f "$pin" ]; then
        add_violation "§10.5" "missing .python-version; pin the Python version at repo root (see §10.5)"
        return
    fi
    local pinned; pinned="$(tr -d '[:space:]' < "$pin")"
    if [ -z "$pinned" ]; then
        add_violation "§10.5" ".python-version is empty; pin a concrete version like '3.12' (see §10.5)"
        return
    fi
    if is_floating_specifier "$pinned"; then
        add_violation "§10.5" ".python-version '$pinned' is a floating specifier; pin to an exact version (see §10.5)"
        return
    fi
    if [ -n "$ci_yml" ]; then
        local ci_ver; ci_ver="$(find_setup_version "$ci_yml" "actions/setup-python" "python-version")"
        if [ -n "$ci_ver" ] && ! versions_same_major_minor "$pinned" "$ci_ver"; then
            add_violation "§10.5" \
                ".python-version '$pinned' does not match ci.yml python-version '$ci_ver' (see §10.5)"
        fi
    fi
}

check_node_pin() {
    local ci_yml="$1"
    local pin="$TARGET/.nvmrc"
    if [ ! -f "$pin" ]; then
        add_violation "§10.5" "missing .nvmrc; pin the Node version at repo root (see §10.5)"
        return
    fi
    local pinned; pinned="$(tr -d '[:space:]' < "$pin")"
    pinned="${pinned#v}"
    if [ -z "$pinned" ]; then
        add_violation "§10.5" ".nvmrc is empty; pin a concrete major version like '24' (see §10.5)"
        return
    fi
    if is_floating_specifier "$pinned"; then
        add_violation "§10.5" ".nvmrc '$pinned' is a floating specifier; pin to a concrete version (see §10.5)"
        return
    fi
    if [ -n "$ci_yml" ]; then
        local ci_ver; ci_ver="$(find_setup_version "$ci_yml" "actions/setup-node" "node-version")"
        if [ -n "$ci_ver" ]; then
            local ci_major="${ci_ver%%.*}"
            local pin_major="${pinned%%.*}"
            if [ "$ci_major" != "$pin_major" ]; then
                add_violation "§10.5" \
                    ".nvmrc '$pinned' does not match ci.yml node-version '$ci_ver' (see §10.5)"
            fi
        fi
    fi
}

check_go_pin() {
    local ci_yml="$1"
    local pin="$TARGET/go.mod"
    [ -f "$pin" ] || return 0
    local toolchain; toolchain="$(parse_go_toolchain "$pin")"
    if [ -z "$toolchain" ]; then
        add_violation "§10.5" \
            "go.mod must contain a 'toolchain goX.Y.Z' directive pinning the Go toolchain (see §10.5)"
        return
    fi
    if is_floating_specifier "$toolchain"; then
        add_violation "§10.5" \
            "go.mod toolchain '$toolchain' is a floating specifier; pin to an exact version (see §10.5)"
        return
    fi
    if [ -n "$ci_yml" ]; then
        local ci_ver; ci_ver="$(find_setup_version "$ci_yml" "actions/setup-go" "go-version")"
        if [ -n "$ci_ver" ] && ! versions_same_major_minor "$toolchain" "$ci_ver"; then
            add_violation "§10.5" \
                "go.mod toolchain '$toolchain' does not match ci.yml go-version '$ci_ver' (see §10.5)"
        fi
    fi
}

# ---------------------------------------------------------------------------
# Agent skills (mirrors src/validate/agent_skills.rs)
# ---------------------------------------------------------------------------
is_kebab_case() {
    [[ "$1" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]
}

extract_yaml_key() {
    # $1 = file, $2 = key. Returns 0 if a top-level `<key>:` line exists.
    local f="$1" k="$2"
    awk -v k="$k" '
        BEGIN { in_fm=0; opened=0 }
        NR==1 && /^---$/ { in_fm=1; opened=1; next }
        in_fm && /^---$/ { exit }
        in_fm {
            if (substr($0,1,1) == " " || substr($0,1,1) == "\t" || substr($0,1,1) == "#") next
            if (index($0, k ":") == 1) { found=1; exit }
        }
        END { exit (found?0:1) }
    ' "$f"
}

has_front_matter() {
    head -n 1 "$1" 2>/dev/null | grep -q '^---$'
}

check_agent_skills() {
    local skills_root="$TARGET/.agent/skills"
    if [ ! -d "$skills_root" ]; then
        add_violation "§21.2" "missing directory .agent/skills (see §21 Agent skills)"
        return
    fi

    # .claude/skills must be a symlink whose target ends with .agent/skills.
    local claude="$TARGET/.claude/skills"
    local link_ok=0
    if [ -L "$claude" ]; then
        local tgt; tgt="$(readlink "$claude")"
        tgt="${tgt%/}"
        case "$tgt" in
            *".agent/skills") link_ok=1 ;;
        esac
    fi
    if [ "$link_ok" -eq 0 ]; then
        add_violation "§21.2" ".claude/skills must be a symlink to ../.agent/skills"
    fi

    # Validate every skill subdirectory.
    local present=()
    local sub name
    while IFS= read -r -d '' sub; do
        [ -d "$sub" ] || continue
        name="$(basename "$sub")"
        present+=("$name")
        validate_skill_dir "$sub" "$name"
    done < <(find "$skills_root" -mindepth 1 -maxdepth 1 -type d -print0 2>/dev/null)

    # Required skills.
    local required=("maintenance|always")
    [ -f "$TARGET/README.md" ] && required+=("update-readme|README.md")
    [ -d "$TARGET/docs" ]      && required+=("update-docs|docs/")
    [ -d "$TARGET/man" ]       && required+=("update-manpages|man/")
    [ -d "$TARGET/website" ]   && required+=("update-website|website/")

    local r skill artifact found p
    for r in "${required[@]}"; do
        skill="${r%%|*}"; artifact="${r##*|}"
        found=0
        for p in "${present[@]:-}"; do
            [ "$p" = "$skill" ] && found=1 && break
        done
        if [ "$found" -eq 0 ]; then
            local sec="§21.5"
            [ "$skill" = "maintenance" ] && sec="§21.6"
            local reason
            if [ "$artifact" = "always" ]; then
                reason="always required"
            else
                reason="required because $artifact is present"
            fi
            add_violation "$sec" "missing maintenance skill .agent/skills/$skill/SKILL.md ($reason)"
        fi
    done
}

validate_skill_dir() {
    local dir="$1" name="$2"
    if ! is_kebab_case "$name"; then
        add_violation "§21.5" \
            ".agent/skills/$name: skill name must be kebab-case (lowercase letters, digits, hyphens)"
    fi
    local skill_md="$dir/SKILL.md"
    local last_updated="$dir/.last-updated"
    if [ ! -f "$skill_md" ]; then
        add_violation "§21.3" ".agent/skills/$name: missing SKILL.md"
        return
    fi
    if [ ! -f "$last_updated" ]; then
        add_violation "§21.4" ".agent/skills/$name: missing .last-updated tracking file (see §21.4)"
    fi
    if ! has_front_matter "$skill_md"; then
        add_violation "§21.3" \
            ".agent/skills/$name/SKILL.md: missing YAML front matter with \`name\` and \`description\`"
        return
    fi
    if ! extract_yaml_key "$skill_md" "name"; then
        add_violation "§21.3" ".agent/skills/$name/SKILL.md: front matter missing \`name\` field"
    fi
    if ! extract_yaml_key "$skill_md" "description"; then
        add_violation "§21.3" ".agent/skills/$name/SKILL.md: front matter missing \`description\` field"
    fi
}

# ---------------------------------------------------------------------------
# Agent prompt rendering
#
# The qualitative checklist + final imperative both live in
# `prompts/validate-sh-agent/<v>.md` (§13.5). At runtime we download the
# latest version, strip YAML front matter, extract the `## User` section,
# and substitute the two `{{ jinja }} ` placeholders the script supplies:
#
#   {{ spec_ref }}    — single-line path/URL where OSS_SPEC.md can be read
#   {{ diff_block }}  — multi-line block (or empty string) telling the agent
#                       to `git diff -- OSS_SPEC.md` when the upstream copy
#                       differs from the project's committed version
#
# When the prompt download fails we print a tight fallback that points the
# agent at the upstream URL — a stale or missing prompt is worse than a
# clear pointer.
# ---------------------------------------------------------------------------
print_agent_prompt() {
    local spec_ref="${SPEC_PATH:-$SPEC_URL}"
    local diff_block=""
    if [ "$SPEC_CHANGED" = "yes" ]; then
        diff_block=$'  1a. Run `git diff -- OSS_SPEC.md` first — this script just replaced\n      OSS_SPEC.md with a newer upstream copy that differs from the\n      version committed in this repo. The diff is the short-list of\n      mandates the project may have drifted away from since the last\n      conformance pass. Treat it as the spec\'s changelog.\n'
    fi

    if [ -z "$PROMPT_FILE" ] || [ ! -s "$PROMPT_FILE" ]; then
        printf '\n'
        warn "Could not fetch the agent review prompt from $PROMPT_URL"
        warn "(no curl/wget, or network failure)."
        warn "Read it manually at the URL above before acting on this script's"
        warn "structural-violations report — the prompt enumerates every spec"
        warn "section the deterministic checks above could not verify."
        return
    fi

    local user_body
    user_body="$(extract_user_section "$PROMPT_FILE")"
    if [ -z "$user_body" ]; then
        printf '\n'
        warn "Downloaded $PROMPT_URL but could not parse a \`## User\` section."
        warn "Read the file manually at the URL above; it is the qualitative"
        warn "review checklist and the final agent prompt for this script's output."
        return
    fi

    # Replace placeholders. Bash parameter expansion handles multi-line
    # replacement values fine; we deliberately use the same `{{ name }}`
    # syntax as the project's other prompts so the templates remain
    # interchangeable.
    user_body="${user_body//\{\{ spec_ref \}\}/$spec_ref}"
    user_body="${user_body//\{\{ diff_block \}\}/$diff_block}"

    printf '\n%s\n' "$user_body"
}

# Read a §13.5 prompt file and return everything between `## User` and the
# next `## ` H2 (or EOF). YAML front matter is stripped.
extract_user_section() {
    awk '
        BEGIN { in_fm = 0; opened_fm = 0; in_user = 0 }
        # Strip YAML front matter (one leading "---" / closing "---" pair).
        NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; opened_fm = 1; next }
        opened_fm && in_fm && /^---[[:space:]]*$/ { in_fm = 0; next }
        in_fm { next }
        # Track section state.
        /^##[[:space:]]+User[[:space:]]*$/ { in_user = 1; next }
        /^##[[:space:]]/                   { if (in_user) in_user = 0 }
        in_user                             { print }
    ' "$1"
}

# ---------------------------------------------------------------------------
# Run all checks and report
# ---------------------------------------------------------------------------
header "oss-spec validate.sh — checking $TARGET"
info "spec source:   $SPEC_URL (script pinned against version: $SPEC_VERSION)"
info "agent prompt:  $PROMPT_URL (script pinned against version: $PROMPT_VERSION)"
case "$SPEC_CHANGED" in
    yes)
        warn "OSS_SPEC.md was REPLACED at $SPEC_PATH; the upstream copy differs from"
        warn "the version committed in this repo. Run \`git diff -- OSS_SPEC.md\` to see"
        warn "exactly what mandates changed and treat that diff as the spec's changelog"
        warn "when prioritising fixes."
        ;;
    no)
        ok "OSS_SPEC.md is up to date — the upstream copy matches the committed version."
        ;;
    fresh)
        info "OSS_SPEC.md downloaded fresh to $SPEC_PATH (no pre-existing copy)."
        ;;
    unknown)
        if [ "$SPEC_DOWNLOADED" -eq 1 ]; then
            info "OSS_SPEC.md at $SPEC_PATH was overwritten with the latest upstream copy."
            info "(Not in a tracked git repo, so cannot diff — compare against your"
            info "project's last-known-good copy manually if needed.)"
        else
            warn "Could not fetch $SPEC_URL (no curl/wget, or network failure). The spec"
            warn "evolves, so this script will not validate against any pre-existing local"
            warn "OSS_SPEC.md — fetch the current upstream copy out-of-band before acting"
            warn "on the agent prompt below."
        fi
        ;;
esac

check_required_files
check_agents_symlinks
check_required_dirs
check_prompts_versioned
check_workflows
check_issue_pr_templates
check_test_naming
check_output_module
check_no_inline_tests
check_source_file_size
check_website_seo
check_ci_toolchains
check_local_pins
check_agent_skills

header "Structural violations"
if [ "${#VIOLATIONS[@]}" -eq 0 ]; then
    ok "  none — all deterministic checks pass."
else
    err "  ${#VIOLATIONS[@]} violation(s):"
    i=1
    for v in "${VIOLATIONS[@]}"; do
        printf '  %2d. %s\n' "$i" "$v" >&2
        i=$((i+1))
    done
fi

print_agent_prompt

if [ "${#VIOLATIONS[@]}" -ne 0 ]; then
    exit 1
fi
exit 0
