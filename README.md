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
