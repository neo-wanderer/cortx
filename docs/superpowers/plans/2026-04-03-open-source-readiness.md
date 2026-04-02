# Open-Source Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prepare the cortx repository for public open-source release with licensing, documentation, CI/CD, and automated changelog.

**Architecture:** No application code changes. This is purely additive — license files, docs, CI workflows, and config. Each task is independent and produces a committable unit.

**Tech Stack:** GitHub Actions, git-cliff, cross (for cross-compilation), standard open-source files.

**Spec:** `docs/superpowers/specs/2026-04-03-open-source-readiness-design.md`

---

### Task 1: License Files

**Files:**
- Create: `LICENSE-MIT`
- Create: `LICENSE-APACHE`

- [ ] **Step 1: Create LICENSE-MIT**

```
MIT License

Copyright (c) 2026 cortx contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 2: Create LICENSE-APACHE**

Use the standard Apache License 2.0 full text from https://www.apache.org/licenses/LICENSE-2.0.txt with copyright line:

```
Copyright 2026 cortx contributors
```

- [ ] **Step 3: Commit**

```bash
git add LICENSE-MIT LICENSE-APACHE
git commit -m "chore: add dual MIT/Apache-2.0 license files"
```

---

### Task 2: Cargo.toml Metadata

**Files:**
- Modify: `Cargo.toml:1-4` (add metadata fields after `edition`)

- [ ] **Step 1: Add metadata fields to Cargo.toml**

The `[package]` section should become:

```toml
[package]
name = "cortx"
version = "0.1.0"
edition = "2024"
description = "Schema-driven CLI for managing Second Brain entities as Markdown files with YAML frontmatter"
license = "MIT OR Apache-2.0"
repository = "https://github.com/neo-wanderer/cortx"
authors = ["cortx contributors"]
keywords = ["cli", "second-brain", "markdown", "gtd", "para"]
categories = ["command-line-utilities"]
```

Do not change anything below the `[package]` section.

- [ ] **Step 2: Verify it builds**

Run: `cargo build`
Expected: successful build, no warnings about metadata.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add package metadata for open-source release"
```

---

### Task 3: Expand .gitignore

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Replace .gitignore contents**

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

`*.lock` ignores cortx's entity-level `.lock` files. `!Cargo.lock` re-includes the Cargo lockfile for reproducible builds.

- [ ] **Step 2: Commit**

```bash
git add .gitignore
git commit -m "chore: expand .gitignore with IDE, OS, and lock file patterns"
```

---

### Task 4: CI Workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create `.github/workflows/` directory**

```bash
mkdir -p .github/workflows
```

- [ ] **Step 2: Create ci.yml**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Format
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy -- -W clippy::all

      - name: Test
        run: cargo test
```

- [ ] **Step 3: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
Expected: no error (valid YAML).

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add CI workflow for fmt, clippy, and tests"
```

---

### Task 5: Release Workflow with git-cliff

**Files:**
- Create: `.github/workflows/release.yml`
- Create: `cliff.toml`

- [ ] **Step 1: Create cliff.toml**

```toml
[changelog]
header = """
# Changelog

All notable changes to this project will be documented in this file.

"""
body = """
{% if version %}\
    ## [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
{% else %}\
    ## [Unreleased]
{% endif %}\
{% for group, commits in commits | group_by(attribute="group") %}
    ### {{ group | striptags | trim | upper_first }}
    {% for commit in commits %}
        - {{ commit.message | upper_first }}\
    {% endfor %}
{% endfor %}\n
"""
trim = true

[git]
conventional_commits = true
filter_unconventional = true
split_commits = false
commit_parsers = [
    { message = "^feat", group = "Features" },
    { message = "^fix", group = "Bug Fixes" },
    { message = "^doc", group = "Documentation" },
    { message = "^perf", group = "Performance" },
    { message = "^refactor", group = "Miscellaneous" },
    { message = "^chore", group = "Miscellaneous" },
    { message = "^test", group = "Miscellaneous" },
    { message = "^ci", group = "Miscellaneous" },
    { body = ".*security", group = "Security" },
    { message = "^revert", group = "Reverted" },
]
filter_commits = false
tag_pattern = "v[0-9].*"
skip_tags = ""
ignore_tags = ""
topo_order = false
sort_commits = "oldest"
```

- [ ] **Step 2: Create release.yml**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.runner }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            runner: ubuntu-latest
            use_cross: true
          - target: aarch64-unknown-linux-gnu
            runner: ubuntu-latest
            use_cross: true
          - target: x86_64-apple-darwin
            runner: macos-latest
            use_cross: false
          - target: aarch64-apple-darwin
            runner: macos-latest
            use_cross: false
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Install cross
        if: matrix.use_cross
        run: cargo install cross --locked

      - name: Build
        run: |
          if [ "${{ matrix.use_cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi

      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../cortx-${{ github.ref_name }}-${{ matrix.target }}.tar.gz cortx
          cd ../../..

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: cortx-${{ matrix.target }}
          path: cortx-${{ github.ref_name }}-${{ matrix.target }}.tar.gz

  release:
    name: Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install git-cliff
        run: cargo install git-cliff --locked

      - name: Generate release notes
        run: git-cliff --latest --strip header > RELEASE_NOTES.md

      - name: Generate full changelog
        run: git-cliff --output CHANGELOG.md

      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          body_path: RELEASE_NOTES.md
          files: artifacts/*

      - name: Commit changelog
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add CHANGELOG.md
          git commit -m "docs: update changelog for ${{ github.ref_name }}" || true
          git push origin HEAD:main
```

- [ ] **Step 3: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`
Expected: no error.

- [ ] **Step 4: Commit**

```bash
git add cliff.toml .github/workflows/release.yml
git commit -m "ci: add release workflow with cross-platform builds and git-cliff changelog"
```

---

### Task 6: CONTRIBUTING.md

**Files:**
- Create: `CONTRIBUTING.md`

- [ ] **Step 1: Create CONTRIBUTING.md**

```markdown
# Contributing to cortx

Thanks for your interest in contributing!

## Getting Started

```bash
git clone https://github.com/neo-wanderer/cortx.git
cd cortx
cargo build
cargo test
```

## Before Submitting a PR

Please make sure the following all pass:

```bash
cargo fmt --all -- --check
cargo clippy -- -W clippy::all
cargo test
```

## What Makes a Good PR

- **Focused** — one logical change per PR
- **Tested** — new behavior should have tests
- **Descriptive** — clear commit messages using [conventional commits](https://www.conventionalcommits.org/) (e.g., `feat:`, `fix:`, `docs:`)

## Reporting Issues

Found a bug or have a feature request? Please [open an issue](https://github.com/neo-wanderer/cortx/issues).

## License

By contributing, you agree that your contributions will be dual-licensed under the [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE) licenses.
```

- [ ] **Step 2: Commit**

```bash
git add CONTRIBUTING.md
git commit -m "docs: add contributing guidelines"
```

---

### Task 7: README.md

**Files:**
- Create: `README.md`

- [ ] **Step 1: Create README.md**

```markdown
# cortx

[![CI](https://github.com/neo-wanderer/cortx/actions/workflows/ci.yml/badge.svg)](https://github.com/neo-wanderer/cortx/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

A schema-driven Rust CLI for managing Second Brain entities as Markdown files with YAML frontmatter. Implements [PARA](https://fortelabs.com/blog/para/) + [GTD](https://gettingthingsdone.com/) methodology for a headless server where multiple AI agents share a common knowledge base.

## Install

### From source

```bash
cargo install --git https://github.com/neo-wanderer/cortx.git
```

### From release binaries

Download the latest binary for your platform from [Releases](https://github.com/neo-wanderer/cortx/releases).

### Build locally

```bash
git clone https://github.com/neo-wanderer/cortx.git
cd cortx
cargo build --release
# Binary at target/release/cortx
```

## Quick Start

```bash
# Initialize a vault
cortx init my-vault
cd my-vault

# Create entities
cortx create task --title "Review PR" --set "due=2026-04-05" --set "priority=high"
cortx create project --title "Q2 Planning" --set "status=open"
cortx create note --title "Meeting Notes" --set "tags=[meetings, weekly]"

# Query entities
cortx query 'type = "task" and status != "done" and due < today'
cortx query 'type = "note" and text ~ "meeting"'
```

## CLI Reference

```
cortx init [path]                                     # Bootstrap vault structure
cortx create <type> --title "..." [--set k=v]         # Create entity
cortx show <id>                                       # Display entity
cortx update <id> --set k=v                           # Update fields
cortx archive <id>                                    # Soft delete (status=archived)
cortx delete <id> --force                             # Hard delete
cortx query '<expression>'                            # Filter entities
cortx meta distinct <field> [--where '<expr>']        # Distinct field values
cortx meta count-by <field> [--where '<expr>']        # Group counts
cortx note headings <id>                              # List headings
cortx note insert-after-heading <id> --heading "..."  # Insert content
cortx note replace-block <id> --block-id <id> ...     # Replace block
cortx note read-lines <id> --start N --end M          # Read line range
cortx doctor validate                                 # Validate against schemas
cortx doctor links                                    # Check broken wiki links
```

## Query Language

Operators: `=` `!=` `<` `<=` `>` `>=` `contains` `in` `between` `text~`

Boolean: `and` `or` `not`, parentheses for grouping

Date keywords: `today` `yesterday` `tomorrow`

```bash
# Overdue tasks
cortx query 'type = "task" and status != "done" and due < today'

# People tagged as founders
cortx query 'type = "person" and tags contains "founder"'

# Tasks due this month
cortx query 'type = "task" and due between ["2026-04-01", "2026-04-30"]'

# Full-text search
cortx query 'type = "note" and text ~ "protein"'
```

## Architecture

- **Hexagonal architecture**: CLI (clap) -> domain layer -> Repository trait -> storage adapter
- **Schema-driven**: Entity types defined in `types.yaml`, loaded at runtime. Add new types with config changes only.
- **Parallel reads**: `rayon` for concurrent frontmatter parsing across files
- **File locking**: RAII file-level `.lock` files for safe concurrent writes
- **One file per entity**: Markdown + YAML frontmatter, zero merge conflicts for concurrent agents

## Performance

| Files  | list_all | filter | text_search |
|-------:|---------:|-------:|------------:|
| 100    | 1.8 ms   | 1.9 ms | 1.9 ms      |
| 500    | 7.6 ms   | 7.8 ms | 7.7 ms      |
| 1,000  | 14 ms    | 15 ms  | 15 ms       |
| 5,000  | 71 ms    | 71 ms  | 71 ms       |
| 10,000 | 145 ms   | 153 ms | 150 ms      |
| 20,000 | 442 ms   | 466 ms | 441 ms      |

## Entity Types

Defined in `types.yaml`: task, project, area, resource, note, person, company. Each type specifies its fields, types, required/optional, enums, and defaults.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with install, usage, and query reference"
```

---

### Task 8: Final Verification

- [ ] **Step 1: Verify all expected files exist**

Run: `ls -la LICENSE-MIT LICENSE-APACHE README.md CONTRIBUTING.md cliff.toml .gitignore Cargo.toml .github/workflows/ci.yml .github/workflows/release.yml`
Expected: all 9 files listed, no errors.

- [ ] **Step 2: Verify build still works**

Run: `cargo build`
Expected: successful build.

- [ ] **Step 3: Verify tests still pass**

Run: `cargo test`
Expected: all 65 integration tests + 10 doctests pass.

- [ ] **Step 4: Verify clippy passes**

Run: `cargo clippy -- -W clippy::all`
Expected: no warnings.

- [ ] **Step 5: Verify formatting**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues.
