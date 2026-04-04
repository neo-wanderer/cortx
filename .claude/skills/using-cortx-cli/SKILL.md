---
name: using-cortx-cli
description: Use when managing Second Brain entities through the cortx CLI — creating, querying, updating, or editing vault contents
---

# Using the cortx CLI

## Overview

cortx is a schema-driven CLI that stores entities as Markdown files with YAML frontmatter. Entity types are defined per-vault in `types.yaml` — all share the same generic commands with no type-specific subcommands.

## Mental Model

**Vault:** A directory with entity files in type-specific folders. Each type's folder is defined in `types.yaml`.

**Vault resolution order (highest to lowest priority):**
1. `--vault <path>` — explicit path
2. `--vault-name <name>` — named vault from `~/.cortx/config.toml`
3. `CORTX_VAULT` env var
4. Default vault from `~/.cortx/config.toml` (if set)
5. Current working directory

**Entity:** A Markdown file with YAML frontmatter (typed fields: `id`, `type`, `status`, `tags`, etc.) and a freeform body. The `type` field links the file to its schema in `types.yaml`.

**Links:** Entities reference each other via `link` fields (e.g., an entity's field holds another entity's ID). Soft references, not filesystem paths.

**Multi-vault config:** Named vaults are stored in `~/.cortx/config.toml`. Register a vault with `cortx init <path> --name <name>`. Select it with `--vault-name <name>`. The first registered vault becomes the default automatically.

**Vault-specific types:** Each vault has its own `types.yaml`. Edit it to add, remove, or modify entity types for that vault. New type folders are auto-created on first write.

**ID format:** Auto-generated as `<type>-<YYYYMMDD>-<8char-uuid>` if `--id` is omitted on create.

## Command Reference

**CRUD:**

| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `cortx create <type> --title "..." [--set k=v]` | Create entity | `--id`, `--name`, `--tags`, `--set` |
| `cortx show <id>` | Display entity | |
| `cortx update <id> --set k=v` | Update fields | `--set` (repeatable) |
| `cortx archive <id>` | Soft delete (status=archived) | |
| `cortx delete <id> --force` | Hard delete | `--force` required |

**Query & Aggregation:**

| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `cortx query '<expr>'` | Filter entities | `--sort-by`, `--format` |
| `cortx meta distinct <field>` | Unique field values | `--where '<expr>'` |
| `cortx meta count-by <field>` | Group counts | `--where '<expr>'` |

**Note Editing:**

| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `cortx note headings <id>` | List headings | |
| `cortx note insert-after-heading <id>` | Insert after heading | `--heading`, `--content` |
| `cortx note replace-block <id>` | Replace named block | `--block-id`, `--content` |
| `cortx note read-lines <id>` | Read line range | `--start`, `--end` |

**Schema Introspection:**

| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `cortx schema types` | List all entity types in the vault | `--format json` |
| `cortx schema show <type>` | Show fields, types, required, defaults for a type | `--format json` |

**Maintenance:**

| Command | Purpose |
|---------|---------|
| `cortx init [path] [--name <name>]` | Bootstrap vault and optionally register it |
| `cortx doctor validate` | Validate against schemas |
| `cortx doctor links` | Check broken wiki links |

## Query Language

**All string values MUST be double-quoted.** Unquoted strings will cause parse errors.

| Operator | Syntax | Example |
|----------|--------|---------|
| Equal | `field = "value"` | `status = "open"` |
| Not equal | `field != "value"` | `status != "done"` |
| Less than | `field < value` | `due < today` |
| Less/equal | `field <= value` | `scheduled <= today` |
| Greater than | `field > value` | `due > "2026-01-01"` |
| Greater/equal | `field >= value` | `created_at >= "2026-03-01"` |
| Contains | `field contains "value"` | `tags contains "urgent"` |
| In | `field in ["a", "b"]` | `status in ["open", "waiting"]` |
| Between | `field between ["start", "end"]` | `due between ["2026-04-01", "2026-04-30"]` |
| Text search | `text ~ "pattern"` | `text ~ "meeting notes"` |
| And / Or / Not | `expr and expr` | `status = "open" and due < today` |

`today` resolves to current date. Parentheses group expressions: `(a or b) and c`.

## Sort Order

Use `--sort-by` on `cortx query` to order results. Format: `field[:order][,field[:order]...]`. Order defaults to `asc`.

```bash
# Sort by a date field ascending (default)
cortx query 'type = "widget" and status = "open"' --sort-by due

# Sort descending
cortx query 'type = "widget"' --sort-by due:desc

# Multi-field sort
cortx query 'type = "widget" and status = "open"' --sort-by priority:asc,due:desc

# Quoted field names with spaces
cortx query 'type = "widget"' --sort-by '"Due By":desc'
```

**Null/missing values always sort to the end**, regardless of ascending or descending order.

## Recipes

**Filter by type and status:**
```bash
# Open entities with no linked parent
cortx query 'type = "widget" and status = "open" and project = null'

# Overdue (any entity with a due date field)
cortx query 'type = "widget" and status != "done" and due < today'

# Scheduled for today or earlier
cortx query 'type = "widget" and status = "open" and scheduled <= today'

# Entities linked to a specific parent
cortx query 'type = "widget" and project = "proj-20260401-abc12345"'
```

**Discovery:**
```bash
# Entities with a specific tag
cortx query 'tags contains "urgent"'

# Entities created this month
cortx query 'created_at >= "2026-04-01"'

# Full-text search across all entity bodies
cortx query 'text ~ "quarterly review"'

# Entities of any type with a specific status
cortx query 'status = "open"'
```

**Sorted queries:**
```bash
# Overdue entities sorted by due date (oldest first)
cortx query 'status != "done" and due < today' --sort-by due:asc

# Entities sorted by priority then due date
cortx query 'status = "open"' --sort-by priority:asc,due:asc
```

**Aggregation:**
```bash
# Distinct values for a field
cortx meta distinct status --where 'type = "widget"'

# Entity count by type
cortx meta count-by type

# Entity count grouped by status
cortx meta count-by status --where 'type = "widget"'
```

**Note editing:**
```bash
# List headings in an entity's body
cortx note headings widget-20260402-abc12345

# Insert content after a heading (heading text only, no markdown prefix)
cortx note insert-after-heading widget-20260402-abc12345 \
  --heading "Progress" --content "- Completed initial review"

# Replace a named block
cortx note replace-block widget-20260402-abc12345 \
  --block-id summary --content "Updated summary text"
```

**Schema introspection (for agents discovering vault types):**
```bash
# List all types in the vault
cortx schema types
cortx schema types --format json

# Inspect a specific type's fields
cortx schema show widget
cortx schema show widget --format json
```

**CRUD flow:**
```bash
# Create an entity with fields
cortx create widget --title "Review PR" \
  --set project=proj-20260401-abc12345 --set due=2026-04-05 \
  --tags "urgent,review"

# Update a field
cortx update widget-20260402-abc12345 --set status=in_progress

# Archive when done
cortx archive widget-20260402-abc12345
```
