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

**Entity:** A Markdown file with YAML frontmatter (typed fields: `type`, `status`, `tags`, etc.) and a freeform body. The `type` field links the file to its schema in `types.yaml`. The entity ID is derived from the filename stem — it is **not** stored in frontmatter.

**ID format:** Auto-generated as a slug derived from `--title` or `--name` (e.g., `"Buy groceries"` → `buy-groceries`). Unicode is transliterated to ASCII, lowercased, non-alphanumeric runs replaced with hyphens. Override with `--id`. If a slug collides with an existing file, the create command fails — use `--id` to specify a unique name.

**Entity types (Second Brain vault):**

> **Schema dependency:** `goal` and `log` are new types requiring an updated `types.yaml`. The `inbox` task status and fields like `state`, `context`, `energy`, `goal` also require the updated schema. Run `cortx schema types` to verify what's available before using these types.

| Type | Folder | Key fields |
|---|---|---|
| area | 2_Areas/ | title, up, archived |
| goal | 1_Goals/ | title, type_val [goal/milestone], kind [time-bound/ongoing], status, area, priority |
| task | 1_Goals/tasks/ | title, status (default: open — use `--set status=inbox` to capture), goal, priority, state [easy/quick/flow] |
| note | 3_Resources/notes/ | title, kind, status, area, goal |
| resource | 3_Resources/ | title, kind, ref, area, goal |
| log | 4_Logs/ | title, date, kind, impact, goal |
| person | 5_People/ | name, relationship, company |
| company | 5_Companies/ | name, domain, industry |

**Links:** Entities reference each other via `link` fields (e.g., `goal: q2-planning`). The value is the ID (filename stem) of the referenced entity. Bidirectional link fields automatically update the inverse field on the referenced entity when a create or update is written.

**Multi-vault config:** Named vaults are stored in `~/.cortx/config.toml`. Register a vault with `cortx init <path> --name <name>`. Select it with `--vault-name <name>`. The first registered vault becomes the default automatically.

**Vault-specific types:** Each vault has its own `types.yaml`. Edit it to add, remove, or modify entity types for that vault. New type folders are auto-created on first write.

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
| `cortx schema validate` | Check `types.yaml` ref integrity and relation consistency | |

**Maintenance:**

| Command | Purpose |
|---------|---------|
| `cortx init [path] [--name <name>]` | Bootstrap vault and optionally register it |
| `cortx doctor validate` | Validate all entity files against schemas |
| `cortx doctor links [--fix]` | Check bidirectional relation consistency; `--fix` auto-repairs missing inverses |

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
cortx query 'type = "task" and status = "open"' --sort-by due

# Sort descending
cortx query 'type = "task"' --sort-by due:desc

# Multi-field sort
cortx query 'type = "task" and status = "open"' --sort-by priority:asc,due:desc

# Quoted field names with spaces
cortx query 'type = "task"' --sort-by '"Due By":desc'
```

**Null/missing values always sort to the end**, regardless of ascending or descending order.

## Recipes

**Filter by type and status:**
```bash
# Overdue (any entity with a due date field)
cortx query 'type = "task" and status != "done" and due < today'

# Scheduled for today or earlier
cortx query 'type = "task" and status = "open" and scheduled <= today'

# Entities linked to a specific parent (use slug ID)
cortx query 'type = "task" and goal = "q2-planning"'
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
cortx meta distinct status --where 'type = "task"'

# Entity count by type
cortx meta count-by type

# Entity count grouped by status
cortx meta count-by status --where 'type = "task"'
```

**Note editing:**
```bash
# List headings in an entity's body
cortx note headings review-q2-goals

# Insert content after a heading (heading text only, no markdown prefix)
cortx note insert-after-heading review-q2-goals \
  --heading "Progress" --content "- Completed initial review"

# Replace a named block
cortx note replace-block review-q2-goals \
  --block-id summary --content "Updated summary text"
```

**Schema introspection (for agents discovering vault types):**
```bash
# List all types in the vault
cortx schema types
cortx schema types --format json

# Inspect a specific type's fields
cortx schema show task
cortx schema show task --format json

# Validate types.yaml for ref integrity and relation consistency
cortx schema validate
```

**CRUD flow:**
```bash
# Create an entity — ID is auto-generated as slug from title
cortx create task --title "Review PR" \
  --set goal=q2-planning --set due=2026-04-05 \
  --tags "urgent,review"
# Creates: 1_Goals/tasks/review-pr.md with ID: review-pr

# Override ID when you need a date prefix or disambiguation
cortx create note --title "Meeting Notes" --id 2026-04-05-acme-kickoff

# Update a field
cortx update review-pr --set status=in_progress

# Archive when done
cortx archive review-pr
```

**Goal management:**
```bash
# Create a time-bound goal
cortx create goal --title "Launch v2.0" \
  --set type_val=goal --set kind=time-bound --set status=active \
  --set area=product \
  --set start_date=2026-04-01 --set end_date=2026-06-30 \
  --set priority=high
# Creates: 1_Goals/launch-v2-0.md with ID: launch-v2-0

# Create a milestone under a goal
cortx create goal --title "Complete backend API" \
  --set type_val=milestone --set kind=time-bound \
  --set up=launch-v2-0 \
  --set start_date=2026-04-01 --set end_date=2026-04-30

# All active goals
cortx query 'type = "goal" and status = "active"' --sort-by end_date:asc

# All milestones for a goal
cortx query 'type = "goal" and up = "launch-v2-0"'

# Tasks for a goal
cortx query 'type = "task" and goal = "launch-v2-0"' --sort-by priority:desc
```

**Task inbox and state-based filtering:**
```bash
# Capture to inbox (must be explicit — default status is "open")
cortx create task --title "Call John about budget" --set status=inbox

# View inbox (unclarified tasks)
cortx query 'type = "task" and status = "inbox"'

# Clarify: move to open with GTD fields
cortx update call-john-about-budget \
  --set status=open --set context=computer --set state=flow --set priority=high

# Quick wins (short tasks)
cortx query 'type = "task" and status = "open" and state = "quick"' --sort-by duration:asc

# Deep focus tasks
cortx query 'type = "task" and status = "open" and state = "flow"' --sort-by priority:desc

# Low energy tasks at home
cortx query 'type = "task" and status = "open" and state = "easy" and energy = "low" and context = "home"'
```

**Log / timeline recipes:**
```bash
# Record a decision
cortx create log --title "Decided to migrate to Rust" \
  --set kind=decision --set date=2026-04-04 --set impact=positive \
  --set goal=launch-v2-0

# Record a risk
cortx create log --title "Key engineer may leave Q2" \
  --set kind=risk --set date=2026-04-04 --set impact=negative \
  --set goal=launch-v2-0

# Timeline for a goal (chronological)
cortx query 'type = "log" and goal = "launch-v2-0"' --sort-by date:asc

# All decisions across vault
cortx query 'type = "log" and kind = "decision"' --sort-by date:desc
```

**Relation maintenance:**
```bash
# Check for broken or missing bidirectional inverses
cortx doctor links

# Auto-repair missing inverses
cortx doctor links --fix
```
