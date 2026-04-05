# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Changed (BREAKING)

- Entities are now identified by their human title (filesystem-safe), not a slug. Files are named `Buy Groceries.md` instead of `buy-groceries.md`. Sanitization replaces `/ \ : * ? " < > |` and control chars with spaces, collapses whitespace, strips trailing dots, NFC-normalizes.
- Link-typed frontmatter fields are stored as Obsidian wikilinks (`project: "[[Website Redesign]]"`). cortx wraps on write and unwraps on read; CLI and query language continue to accept bare titles.
- `cortx show`/`update`/`archive`/`delete`/`note` now take a human title as the positional argument. Lookup is case-insensitive.
- `update --set title=...` is rejected — use `cortx rename` instead.
- `cortx create` rejects on case-insensitive title collision across the entire vault.

### Added

- `cortx rename "<old>" "<new>"` command with transactional cascade (file rename, frontmatter back-refs, body wikilinks). Flags: `--dry-run`, `--skip-body`.
- `cortx doctor filenames` subcommand: detects filename/title drift, case-insensitive collisions, wikilink format issues. `--fix` auto-repairs drift and wraps bare link values. `--check-bodies` scans note bodies for unresolved wikilinks.
- `--no-validate-links` flag on `create`/`update` bypasses link-target existence checks (for bulk imports).
- `rename_bench` criterion benchmark at 100/500/5000 entities.

### Removed

- The `to_slug` function and `deunicode` dependency.

## [0.5.0] - 2026-04-04

### Documentation

- Update changelog for v0.4.0

### Features

- JARVIS Second Brain skills and OpenWolf integration (#4)
- Schema relations, slug filenames, schema validate, doctor links --fix (#5)

### Miscellaneous

- Add pre-commit clippy hook and auto-format on edit

## [0.4.0] - 2026-04-04

### Features

- Vault-specific types and schema introspection command (#3)

### Miscellaneous

- Bump version to v0.3.0 and automate version bump in release workflow
- Add release.sh script and simplify release workflow
- Bump version to v0.4.0

## [0.3.0] - 2026-04-03

### Documentation

- Update changelog for v0.2.0

### Features

- Global config and multi-vault support (#2)

## [0.2.0] - 2026-04-03

### Documentation

- Update changelog for v0.1.0

## [0.1.0] - 2026-04-02

### Documentation

- Add inline documentation and doctests for public API
- Add using-cortx-cli skill spec, plan, and CLAUDE.md
- Add open-source readiness design spec
- Update spec with git-cliff for automated changelog generation
- Add open-source readiness implementation plan
- Add contributing guidelines
- Add README with install, usage, and query reference

### Features

- Project setup with dependencies and error types
- Add Value type with date parsing, comparison, and array contains
- Add schema types and TypeRegistry with types.yaml config
- Add frontmatter validation against schema definitions
- Add frontmatter parsing and serialization
- Add Entity struct
- Add Repository trait and MarkdownRepository adapter
- Add file-level locking for safe concurrent writes
- Add query AST types
- Add query language parser
- Add query evaluator for filtering entities
- Add CLI with generic CRUD, query, meta, note editing, and doctor commands
- Add CLI integration tests
- Add init command to bootstrap vault structure
- Add performance benchmarks with criterion
- Extend benchmarks to 10k and 20k files, update baseline

### Miscellaneous

- Fix clippy warnings and verify full test suite
- Expand .gitignore with IDE, OS, and lock file patterns
- Add package metadata for open-source release
- Add dual MIT/Apache-2.0 license files
- Add CI workflow for fmt, clippy, and tests
- Add release workflow with cross-platform builds and git-cliff changelog
- Add code coverage gate at 95% using cargo-llvm-cov
- Boost coverage to 98% with targeted tests and exclusions

### Style

- Run cargo fmt on all source files
- Run cargo fmt on test files

