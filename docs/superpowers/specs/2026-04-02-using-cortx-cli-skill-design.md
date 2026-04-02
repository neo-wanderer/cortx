# Skill Design: using-cortx-cli

## Skill Metadata

- **Name:** `using-cortx-cli`
- **Type:** Reference
- **Description:** `Use when managing Second Brain entities (tasks, projects, people, notes) through the cortx CLI — creating, querying, updating, or editing vault contents`
- **Audience:** AI agents using cortx as an installed CLI tool
- **Location:** `~/.claude/skills/using-cortx-cli/SKILL.md`

## Structure

Three sections: Mental Model, Command Reference, Recipes.

### Section 1: Mental Model (~80 words)

Three concepts:

1. **Vault** — a directory containing entity files organized into type-specific folders (e.g., `1_Projects/tasks/`, `5_People/`). Folder mapping comes from `types.yaml`.

2. **Entity** — a Markdown file with YAML frontmatter (typed fields like `id`, `type`, `status`, `tags`) and a freeform body. The `type` field links the file to its schema definition.

3. **Links** — entities reference each other via `link` fields (e.g., a task's `project` field holds a project ID). These are soft references, not filesystem paths.

### Section 2: Command Reference

Single table covering all 15 commands, grouped by function.

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
| `cortx query '<expr>'` | Filter entities | |
| `cortx meta distinct <field>` | List unique values for a field | `--where '<expr>'` |
| `cortx meta count-by <field>` | Group and count by field | `--where '<expr>'` |

**Note Editing:**

| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `cortx note headings <id>` | List headings in body | |
| `cortx note insert-after-heading <id>` | Insert content after a heading | `--heading`, `--content` |
| `cortx note replace-block <id>` | Replace a named block | `--block-id`, `--content` |
| `cortx note read-lines <id>` | Read line range | `--start`, `--end` |

**Maintenance:**

| Command | Purpose |
|---------|---------|
| `cortx init [path]` | Bootstrap vault structure |
| `cortx doctor validate` | Validate files against schemas |
| `cortx doctor links` | Check for broken wiki links |

**Global flag:** `--vault <path>` (also `CORTX_VAULT` env var, defaults to current dir).

**ID generation:** If `--id` is omitted on `create`, cortx auto-generates `<type>-<YYYYMMDD>-<8char-uuid>`.

### Section 3: Query Language Quick Reference

Compact operator table:

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
| And | `expr and expr` | `status = "open" and due < today` |
| Or | `expr or expr` | `type = "task" or type = "project"` |
| Not | `not expr` | `not status = "done"` |

Notes:
- `today` is a special keyword resolving to the current date
- String values must be quoted with double quotes
- Parentheses can group expressions: `(a or b) and c`
- Array fields (like `tags`) use `contains` to check membership

### Section 4: Recipes

Practical composable patterns grouped by use case.

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
# All people tagged "founder"
cortx query 'type = "person" and tags contains "founder"'

# Notes created this month
cortx query 'type = "note" and created_at >= "2026-04-01"'

# Full-text search across all entities
cortx query 'text ~ "quarterly review"'
```

**Aggregation:**
```bash
# What statuses exist across all tasks?
cortx meta distinct status --where 'type = "task"'

# How many entities per type?
cortx meta count-by type

# Task count by status
cortx meta count-by status --where 'type = "task"'
```

**Note editing workflow:**
```bash
# See what headings exist
cortx note headings task-20260402-abc12345

# Add content under a heading
cortx note insert-after-heading task-20260402-abc12345 \
  --heading "Progress" --content "- Completed initial review"

# Replace a named block
cortx note replace-block task-20260402-abc12345 \
  --block-id summary --content "Updated summary text here"
```

**Common CRUD flow:**
```bash
# Create a task linked to a project
cortx create task --title "Review PR" \
  --set project=proj-website-redesign --set due=2026-04-05 \
  --tags "urgent,review"

# Update its status
cortx update task-20260402-abc12345 --set status=in_progress

# Archive when done
cortx archive task-20260402-abc12345
```

## Design Decisions

1. **Documents full planned CLI** — assumes all 15 commands are implemented. Agents should check `cortx --help` for currently available commands.
2. **Reference skill type** — no discipline enforcement, just documentation for correct CLI usage.
3. **Hybrid structure** — mental model for grounding, command reference for lookup, recipes for practical patterns.
4. **Query language inline** — operator table + examples kept inline since the query language is the most non-obvious part of the CLI.
5. **No type-specific guidance** — the CLI is generic by design. Entity types are defined in `types.yaml`, not hardcoded. The skill reflects this.
6. **Token-conscious** — uses tables and code blocks for density. Reference skills are allowed more space than frequently-loaded skills, but should still be concise. Prioritize the recipes section as the highest-value content.
