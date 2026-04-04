# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

cortx is a schema-driven Rust CLI for managing Second Brain entities (tasks, projects, people, companies, notes, areas, resources) as Markdown files with YAML frontmatter. It implements PARA + GTD methodology for a headless server where multiple AI agents share a common knowledge base.

## Architecture

- **Hexagonal architecture**: CLI layer (clap) → domain/application layer → Repository trait → storage adapter
- **Schema-driven**: Entity types defined in `types.yaml`, loaded at runtime into a `TypeRegistry`. Adding new types requires only config changes, no recompilation.
- **Storage**: Markdown adapter with parallel reads via `rayon`. File-level `.lock` files for concurrent write safety (RAII `FileLock` with `Drop` for auto-release).
- **Query engine**: Custom query language parsed via recursive descent into an AST (`Expr` enum), evaluated against entity frontmatter + body text.

## Build & Test

```bash
cargo build                    # Build debug
cargo build --release          # Build release
cargo test                     # Run all tests (65 integration + 10 doctests)
cargo test --test storage_test # Run a specific integration test file
cargo test --test cli_integration_test -- test_create_and_show  # Run a single test by name
cargo test --doc               # Run doctests only
cargo bench                    # Run performance benchmarks (criterion)
cargo clippy -- -W clippy::all # Lint
cargo llvm-cov --ignore-filename-regex "(main|file_lock|markdown)\.rs" # Coverage
```

Tests use a `TestVault` helper (`tests/common/mod.rs`) that creates a `TempDir` with the required vault folder structure. Integration tests live in `tests/` (query_parser, storage, value, schema, frontmatter, cli_integration).

### Coverage exclusions

`main.rs`, `storage/file_lock.rs`, and `storage/markdown.rs` are excluded from coverage reports:
- **main.rs**: `process::exit(1)` error handlers and `unreachable!()` branch — untestable from integration tests
- **file_lock.rs**: OS-level IO error path requires a rare filesystem failure to trigger
- **markdown.rs**: IO error propagation and empty-dir early returns are OS-dependent edge cases

## CLI Commands

```bash
cortx init [path]                                    # Bootstrap vault structure
cortx create <type> --title "..." [--set k=v]        # Create entity (type from types.yaml)
cortx show <id>                                      # Display entity
cortx update <id> --set k=v                          # Update fields
cortx archive <id>                                   # Soft delete (status=archived)
cortx delete <id> --force                            # Hard delete
cortx query '<expression>' [--sort-by <spec>]        # Filter and optionally sort entities
cortx meta distinct <field> [--where '<expr>']        # Distinct field values
cortx meta count-by <field> [--where '<expr>']        # Group counts
cortx note headings <id>                             # List headings
cortx note insert-after-heading <id> --heading "..." --content "..."
cortx note replace-block <id> --block-id <id> --content "..."
cortx note read-lines <id> --start N --end M
cortx doctor validate                                # Validate all files against schemas
cortx doctor links [--fix]                           # Check bidirectional relation consistency; --fix auto-repairs
cortx schema types [--format json]                   # List all entity types
cortx schema show <type> [--format json]             # Show fields for a type
cortx schema validate                                # Check ref integrity and relation consistency in types.yaml
```

## Query Language

```
# Operators: = != < <= > >= contains in between text~
# Boolean: and or not, parentheses for grouping
# Date keywords: today yesterday tomorrow
# Examples:
cortx query 'type = "task" and status != "done" and due < today'
cortx query 'type = "person" and tags contains "founder"'
cortx query 'type = "task" and due between ["2026-04-01", "2026-04-30"]'
cortx query 'type = "task" and status in ["open", "in_progress"]'
cortx query 'type = "note" and text ~ "protein"'
```

## Sorting

The `query` command supports sorting results with `--sort-by`:

```
# Basic syntax: field[:order][,field[:order]...]
# Order defaults to 'asc' if not specified

# Single field, ascending (default)
cortx query 'type = "task"' --sort-by due

# Single field, descending
cortx query 'type = "task"' --sort-by priority:desc

# Multiple fields (prioritize first, then by due date)
cortx query 'type = "task"' --sort-by priority:desc,due:asc

# Quoted field names for spaces
cortx query 'type = "task"' --sort-by '"Due By":asc'

# Complex example: high priority first, then by status, then by due date
cortx query 'status != "done"' --sort-by priority:desc,status,due:asc
```

**Sort order**: `asc` (ascending) or `desc` (descending). Case-insensitive.

**Null handling**: Fields with missing/null values always sort to the end, regardless of ascending or descending order. This ensures consistent results when entities lack optional fields.

**Field types**: Sorting works for any comparable field type (dates, strings, numbers). Values are compared using their natural ordering.

## Non-Obvious Patterns

These behaviors span multiple files and are easy to miss:

- **Value coercion**: Strings matching `YYYY-MM-DD` automatically become `Value::Date` during both CLI input parsing (`cli/create.rs:parse_cli_value`) and query parsing (`query/parser.rs`). Arrays use `[a, b, c]` syntax. Everything else becomes `Value::String`.
- **Auto-populated fields**: `create` sets `created_at` and `updated_at` to today. `update` always overwrites `updated_at` with today. These happen in `storage/markdown.rs`, not the CLI layer.
- **Default field values**: If the schema has a `status` field and no value is provided, `create` defaults it to `"open"`. Tags default to `[]`.
- **Title resolution**: `Entity::title()` tries `title` field first, then `name`, then falls back to the entity ID. This allows different entity types to use either field.
- **ID format**: Slug derived from `--title` or `--name` (e.g., `"Buy groceries"` → `buy-groceries`). Override with `--id`. Unicode is transliterated to ASCII via `deunicode`, lowercased, non-alphanumeric runs replaced with hyphens. No date or UUID component in auto-generated IDs.
- **Note block markers**: `replace-block` uses HTML comment markers: `<!-- block:id=NAME -->...<!-- /block:id=NAME -->`.
- **Frontmatter serialization**: YAML keys are sorted alphabetically for deterministic file output. Format: `---\n{yaml}\n---\n{body}`.
- **Query evaluation of missing fields**: Missing fields return `false` for all comparisons except `!=`, which returns `true`.
- **Text search**: `text ~ "pattern"` does case-insensitive substring matching against the entity body.

## Design Principles

- **No domain-specific subcommands**: Everything is generic CRUD + query. "Overdue tasks" = `query 'status != "done" and due < today'`, not `cortx task overdue`.
- **Schema validates writes**: The CLI rejects invalid frontmatter before writing (wrong enum values, missing required fields, bad date formats).
- **One file per entity**: Zero merge conflicts for concurrent agents. File locking protects same-file writes.
- **Parallel reads**: `rayon::par_iter` for frontmatter parsing across files. ~74ms at 5k files.

## Performance Baseline (criterion benchmarks)

| Files  | list_all | filter_complex | text_search |
|-------:|---------:|---------------:|------------:|
| 100    | 1.8 ms   | 1.9 ms         | 1.9 ms      |
| 500    | 7.6 ms   | 7.8 ms         | 7.7 ms      |
| 1,000  | 14.4 ms  | 15.0 ms        | 14.8 ms     |
| 5,000  | 71 ms    | 71 ms          | 71 ms       |
| 10,000 | 145 ms   | 153 ms         | 150 ms      |
| 20,000 | 442 ms   | 466 ms         | 441 ms      |

## Skill Maintenance

When changing any of the following, update the `using-cortx-cli` skill at `~/.claude/skills/using-cortx-cli/SKILL.md` to match:

- CLI commands (adding, removing, renaming commands or flags)
- Query language operators or syntax
- Sort syntax and behavior
- Entity types or fields in `types.yaml`
- Vault structure or folder conventions
- ID generation format
- Note editing semantics (headings, blocks, line ranges)
