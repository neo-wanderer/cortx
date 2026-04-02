# Open-Source Readiness Package — Design Spec

## Overview

Prepare the cortx repository for public open-source release on GitHub (`neo-wanderer/cortx`). This covers licensing, documentation, CI/CD, and contribution guidelines. No code changes to the application itself.

## 1. License Files

**Dual MIT / Apache-2.0**, the Rust ecosystem convention.

- `LICENSE-MIT` — MIT license text, copyright "cortx contributors"
- `LICENSE-APACHE` — Apache License 2.0 full text, copyright "cortx contributors"

Using "cortx contributors" rather than a personal name follows the convention for projects that may accept contributions.

## 2. README.md

Structured sections, adapted from existing CLAUDE.md content:

1. **Header** — project name, one-liner description, badges (CI status, license)
2. **What is cortx?** — schema-driven Rust CLI, PARA + GTD, headless multi-agent Second Brain, Markdown + YAML frontmatter
3. **Install** — `cargo install --git https://github.com/neo-wanderer/cortx.git` and building from source
4. **Quick Start** — `cortx init`, `cortx create`, `cortx query` examples
5. **CLI Reference** — full command table (from CLAUDE.md)
6. **Query Language** — operators, examples (from CLAUDE.md)
7. **Architecture** — brief: hexagonal, schema-driven, parallel reads, file locking
8. **Performance** — benchmark table (from CLAUDE.md)
9. **Contributing** — link to CONTRIBUTING.md
10. **License** — dual MIT/Apache-2.0 with links to both files

No badges beyond CI and license. Keep it clean.

## 3. Cargo.toml Metadata

Add the following fields to `[package]`:

```toml
description = "Schema-driven CLI for managing Second Brain entities as Markdown files with YAML frontmatter"
license = "MIT OR Apache-2.0"
repository = "https://github.com/neo-wanderer/cortx"
authors = ["cortx contributors"]
keywords = ["cli", "second-brain", "markdown", "gtd", "para"]
categories = ["command-line-utilities"]
```

Keep `edition = "2024"` (valid since Rust 1.85). Do not add `publish = false` — the default is fine and doesn't prevent future crates.io publishing.

## 4. .gitignore

Expand the current `/target`-only gitignore:

```gitignore
/target
*.lock
!Cargo.lock
.DS_Store
.idea/
.vscode/
*.swp
*.swo
*~
```

Notes:
- `*.lock` ignores the entity-level `.lock` files used by cortx's file locking, but `!Cargo.lock` keeps the Cargo lockfile tracked (important for reproducible builds of a binary).
- IDE and OS files are standard exclusions.

## 5. GitHub Actions

### 5a. CI Workflow (`.github/workflows/ci.yml`)

**Triggers:** push to `main`, pull requests to `main`.

**Jobs:**
- **check** (runs on `ubuntu-latest`):
  - `cargo fmt --all -- --check`
  - `cargo clippy -- -W clippy::all`
  - `cargo test`

Single job is sufficient for a project this size. Matrix builds across OS/Rust versions are unnecessary at this stage.

Uses `dtolnay/rust-toolchain@stable` for toolchain setup and `Swatinem/rust-cache@v2` for dependency caching.

### 5b. Release Workflow (`.github/workflows/release.yml`)

**Trigger:** push of tags matching `v*`.

**Targets:**
| Target | OS | Runner | Build tool |
|--------|-----|--------|------------|
| `x86_64-unknown-linux-gnu` | Linux x86_64 | `ubuntu-latest` | `cross` |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | `ubuntu-latest` | `cross` |
| `x86_64-apple-darwin` | macOS Intel | `macos-latest` | `cargo` |
| `aarch64-apple-darwin` | macOS Apple Silicon | `macos-latest` | `cargo` |

**Steps per target:**
1. Checkout code
2. Install Rust stable toolchain + target
3. Install `cross` (Linux targets only)
4. Build with `--release`
5. Create tarball: `cortx-{tag}-{target}.tar.gz` containing the binary
6. Upload as release asset via `softprops/action-gh-release`

No Windows target — the project uses Unix file locking semantics. Can be added later if needed.

**Release process for the user:**
```bash
git tag v0.1.0
git push origin v0.1.0
```
GitHub Actions builds binaries and attaches them to the auto-created release.

## 6. CONTRIBUTING.md

Short and practical:

1. **Getting started** — clone, `cargo build`, `cargo test`
2. **Before submitting a PR** — `cargo fmt`, `cargo clippy -- -W clippy::all`, `cargo test` must all pass
3. **What makes a good PR** — focused changes, descriptive commit messages, tests for new behavior
4. **Issues** — link to GitHub issues, encourage filing bugs
5. **License** — contributions are dual-licensed under MIT/Apache-2.0

No CLA, no DCO. Keep the barrier low.

## 7. CHANGELOG.md (auto-generated via git-cliff)

Use [git-cliff](https://git-cliff.org/) to auto-generate CHANGELOG.md from conventional commits on each release.

**Configuration:** `cliff.toml` in repo root. Groups commits by type:
- `feat:` → "Features"
- `fix:` → "Bug Fixes"
- `docs:` → "Documentation"
- `chore:`, `refactor:`, `perf:`, `test:` → "Miscellaneous"

Strips scope prefixes, links to commits. Skips `merge` and `wip` commits.

**Integration with release workflow:** The release workflow runs `git-cliff --latest --strip header` to generate release notes for the GitHub Release, and `git-cliff --output CHANGELOG.md` to regenerate the full changelog. The updated CHANGELOG.md is committed back to `main` automatically.

**Initial run:** The first tag (`v0.1.0`) will generate the initial CHANGELOG.md from the full commit history.

**Release process for the user:**
```bash
git tag v0.1.0
git push origin v0.1.0
# GitHub Actions: builds binaries, generates changelog, creates release
```

No manual changelog maintenance needed. Conventional commits (already in use) are the source of truth.

## Files Created/Modified

| File | Action |
|------|--------|
| `LICENSE-MIT` | Create |
| `LICENSE-APACHE` | Create |
| `README.md` | Create |
| `Cargo.toml` | Modify (add metadata fields) |
| `.gitignore` | Modify (expand) |
| `.github/workflows/ci.yml` | Create |
| `.github/workflows/release.yml` | Create |
| `CONTRIBUTING.md` | Create |
| `cliff.toml` | Create |

Total: 7 new files, 2 modified files. No application code changes. CHANGELOG.md is auto-generated by git-cliff during the release workflow.
