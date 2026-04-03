---
name: using-cortx-cli
description: Use when managing Second Brain entities (tasks, projects, people, notes) through the cortx CLI — creating, querying, updating, or editing vault contents
---

# Using the cortx CLI

## Overview

cortx is a schema-driven CLI that stores entities as Markdown files with YAML frontmatter. All entity types (task, project, person, company, note, area, resource) are defined in `types.yaml` and share the same generic commands — no type-specific subcommands.

## Mental Model

**Vault:** A directory with entity files in type-specific folders (e.g., `1_Projects/tasks/`, `5_People/`). Folder mapping comes from `types.yaml`. Set via `--vault <path>`, `CORTX_VAULT` env var, or defaults to current dir.

**Entity:** A Markdown file with YAML frontmatter (typed fields: `id`, `type`, `status`, `tags`, etc.) and a freeform body. The `type` field links the file to its schema.

**Links:** Entities reference each other via `link` fields (e.g., a task's `project` field holds a project ID). Soft references, not filesystem paths.

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

**Maintenance:**

| Command | Purpose |
|---------|---------|
| `cortx init [path]` | Bootstrap vault |
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
# Sort by due date ascending (default)
cortx query 'type = "task" and status = "open"' --sort-by due

# Sort by due date descending
cortx query 'type = "task"' --sort-by due:desc

# Multi-field sort: priority ascending, then due date descending
cortx query 'type = "task" and status = "open"' --sort-by priority:asc,due:desc

# Quoted field names with spaces
cortx query 'type = "task"' --sort-by '"Due By":desc'
```

**Null/missing values always sort to the end**, regardless of ascending or descending order.

## Recipes

**Task management:**
```bash
# Inbox (unassigned open tasks)
cortx query 'type = "task" and status = "open" and project = null'

# Overdue tasks
cortx query 'type = "task" and status != "done" and due < today'

# Today's scheduled work
cortx query 'type = "task" and status = "open" and scheduled <= today'

# Tasks for a specific project
cortx query 'type = "task" and project = "proj-website-redesign"'
```

**Discovery:**
```bash
# People tagged "founder"
cortx query 'type = "person" and tags contains "founder"'

# Notes created this month
cortx query 'type = "note" and created_at >= "2026-04-01"'

# Full-text search
cortx query 'text ~ "quarterly review"'
```

**Sorted queries:**
```bash
# Overdue tasks sorted by due date (oldest first)
cortx query 'type = "task" and status != "done" and due < today' --sort-by due:asc

# Open tasks sorted by priority then due date
cortx query 'type = "task" and status = "open"' --sort-by priority:asc,due:asc
```

**Aggregation:**
```bash
# Distinct statuses for tasks
cortx meta distinct status --where 'type = "task"'

# Entity count by type
cortx meta count-by type

# Task count by status
cortx meta count-by status --where 'type = "task"'
```

**Note editing:**
```bash
# List headings in a note's body
cortx note headings task-20260402-abc12345

# Insert content after a heading (heading text only, no markdown prefix)
cortx note insert-after-heading task-20260402-abc12345 \
  --heading "Progress" --content "- Completed initial review"

# Replace a named block
cortx note replace-block task-20260402-abc12345 \
  --block-id summary --content "Updated summary text"
```

**CRUD flow:**
```bash
# Create task linked to project
cortx create task --title "Review PR" \
  --set project=proj-website-redesign --set due=2026-04-05 \
  --tags "urgent,review"

# Update status
cortx update task-20260402-abc12345 --set status=in_progress

# Archive when done
cortx archive task-20260402-abc12345
```
