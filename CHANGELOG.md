# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This file is **auto-generated from conventional commits at release time** —
do not edit manually.

## [0.3.0] - 2026-04-18

### Added

- Add standalone sync-oss-spec skill to generated projects (#36)
- Align prompts with spec and enforce §19.4 output module (#33)
- Add §10.5 local/CI toolchain parity and validator rule (#34)
- Add sync-oss-spec conformance skill (#27)

### Documentation

- Drift sweep — sync manpages, docs, readme, website, agents (#26)

## [0.2.0] - 2026-04-16

### Added

- Require Rust 1.88 minimum and add local/CI parity guidance (#20)
- Add AI quality verification and --fix flag (#18)
- Add §21 agent skills with maintenance umbrella (#17)

### Fixed

- Exit non-zero when AI verification fails (#21)

## [0.1.5] - 2026-04-15

### Added

- Version OSS_SPEC.md and ship it with generated projects (#16)
- Store debug logs in ~/.oss-spec/logs/ with per-run timestamps
- Add debug logging throughout and show log path on error
- Add §20 test organization with conformance validation (#14)

### Fixed

- Clear init spinner instead of printing redundant "done"
- Improve mobile friendliness (#13)

## [0.1.4] - 2026-04-13

### Added

- Add React landing page, improve commit skill, add reference impl guidance (#12)

## [0.1.3] - 2026-04-13

### Fixed

- Use trusted publishing for crates.io publish

## [0.1.2] - 2026-04-13

### Added

- Enforce pinned toolchain minimum versions (§10.3) (#11)
- Add --url and --create-issues to check and fix (#9)

### Fixed

- Create AGENTS.md symlinks on Windows (#10)
- Switch crates.io publish to trusted publishing
- Defer spinner until after agent initialization

### Documentation

- Mandate trusted publishing and explicit workflow permissions (#8)
- Add crates.io, release, and pages badges to README

## [0.1.1] - 2026-04-13

- No notable changes

## [Unreleased]

