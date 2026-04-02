# using-cortx-cli Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a reference skill that teaches AI agents how to use the cortx CLI to manage Second Brain entities (tasks, projects, people, notes, etc.) in a vault.

**Architecture:** TDD for skills — RED (baseline test with subagent, no skill) -> GREEN (write minimal skill) -> REFACTOR (close loopholes). The skill itself is a single SKILL.md with three sections: Mental Model, Command Reference, Recipes.

**Skill Type:** Reference (API docs / command reference)

**Location:** `~/.claude/skills/using-cortx-cli/SKILL.md`

---

## File Structure

```
~/.claude/skills/
  using-cortx-cli/
    SKILL.md    # Self-contained reference skill
```

Single file. All content inline — no supporting files needed. The command tables, query reference, and recipes are all compact enough to stay in one document.

---

## Phase: RED — Baseline Testing

### Task 1: Run baseline scenario without skill

**Purpose:** Verify that without the skill, an agent struggles or makes mistakes when asked to use cortx CLI commands. Document exact failure patterns.

- [ ] **Step 1: Create test scenario prompt**

Write the following prompt and dispatch it to a subagent (using the Agent tool) WITHOUT the skill installed. The subagent should have access to the cortx project directory but NOT the skill.

```
You are working in a vault managed by the cortx CLI tool. The vault is at /tmp/test-vault.

Complete these tasks using cortx CLI commands:

1. Create a new task called "Review Q2 budget" assigned to project "proj-finance" with tags "urgent" and "finance", due 2026-04-15
2. Find all open tasks that are overdue
3. Find all people tagged "founder"
4. Show how many tasks exist per status
5. Add a "Progress" note under the "Updates" heading of task task-20260402-abc12345
6. Check the vault for any schema violations
```

- [ ] **Step 2: Dispatch the subagent and record baseline behavior**

Run the subagent with the prompt above. Record verbatim:
- Which commands did the agent get right?
- Which commands did it get wrong (wrong flags, wrong syntax, invented commands)?
- Did it know about the query language syntax?
- Did it try type-specific subcommands (e.g., `cortx task create` instead of `cortx create task`)?
- Did it know about `--set` for key-value fields?
- Did it know about `cortx note` subcommands?

- [ ] **Step 3: Document failure patterns**

Create a temporary file `docs/superpowers/specs/baseline-failures.md` listing each failure pattern observed. Example patterns to watch for:
- Inventing `cortx task` subcommands instead of generic `cortx create task`
- Wrong query syntax (SQL-like `WHERE` clauses, wrong string quoting)
- Not knowing `--set` flag for arbitrary fields
- Not knowing `cortx note` subcommands exist
- Not knowing `cortx meta` aggregation commands
- Using `cortx delete` without `--force`

---

## Phase: GREEN — Write Minimal Skill

### Task 2: Create skill directory and write SKILL.md

**Files:**
- Create: `~/.claude/skills/using-cortx-cli/SKILL.md`

- [ ] **Step 1: Create the skill directory**

```bash
mkdir -p ~/.claude/skills/using-cortx-cli
```

- [ ] **Step 2: Write SKILL.md**

Create `~/.claude/skills/using-cortx-cli/SKILL.md` with this exact content:

```markdown
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
| `cortx query '<expr>'` | Filter entities | |
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

`today` resolves to current date. Strings must be double-quoted. Parentheses group expressions.

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
cortx note headings task-20260402-abc12345
cortx note insert-after-heading task-20260402-abc12345 \
  --heading "Progress" --content "- Completed initial review"
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
```

- [ ] **Step 3: Verify the file was written correctly**

```bash
cat ~/.claude/skills/using-cortx-cli/SKILL.md | head -5
```

Expected: Shows the YAML frontmatter starting with `---`.

- [ ] **Step 4: Commit the skill**

```bash
cd ~/.claude/skills/using-cortx-cli && git init && git add SKILL.md && git commit -m "feat: add using-cortx-cli reference skill"
```

Note: If `~/.claude/skills` is not a git repo, just ensure the file is saved. The commit step is optional depending on user's skills directory setup.

---

### Task 3: Run GREEN test — verify skill fixes baseline failures

- [ ] **Step 1: Re-run the same baseline scenario WITH the skill installed**

Dispatch the same subagent with the same prompt from Task 1, Step 1. This time the skill should be loaded automatically since it lives in `~/.claude/skills/`.

- [ ] **Step 2: Compare results to baseline**

Check each failure pattern from `docs/superpowers/specs/baseline-failures.md`:
- Did the agent now use `cortx create task` instead of invented subcommands?
- Did it use correct query syntax with double-quoted strings?
- Did it use `--set` for arbitrary fields?
- Did it know about `cortx note` and `cortx meta` commands?
- Did it use `--force` with `cortx delete`?

- [ ] **Step 3: Document remaining failures**

If all baseline failures are fixed, move to Task 4. If new failures emerged, document them in `docs/superpowers/specs/baseline-failures.md` under a "GREEN phase — remaining issues" heading.

---

## Phase: REFACTOR — Close Loopholes

### Task 4: Address remaining failures and refine skill

- [ ] **Step 1: Review remaining failures from Task 3**

Read `docs/superpowers/specs/baseline-failures.md` for any issues that persisted through the GREEN phase.

- [ ] **Step 2: Update SKILL.md to address remaining failures**

For each remaining failure, add targeted content to the skill:
- If agents confused command structure, add a "Common Mistakes" note near the command reference
- If query syntax was wrong, add more examples to the query table
- If agents missed flags, bold the critical ones in the table

- [ ] **Step 3: Re-run test scenario**

Dispatch the same subagent with the same prompt. Verify all failures are now fixed.

- [ ] **Step 4: Clean up baseline-failures.md**

If all tests pass, delete `docs/superpowers/specs/baseline-failures.md` — it was a working document.

```bash
rm docs/superpowers/specs/baseline-failures.md
```

- [ ] **Step 5: Final commit**

```bash
cd ~/.claude/skills/using-cortx-cli && git add SKILL.md && git commit -m "refactor: address test feedback in using-cortx-cli skill"
```

---

## Summary

| Phase | Task | What it delivers |
|-------|------|-----------------|
| **RED** | Task 1 | Baseline failures documented — proves skill is needed |
| **GREEN** | Task 2 | SKILL.md written with mental model, command ref, query ref, recipes |
| **GREEN** | Task 3 | Verification that skill fixes baseline failures |
| **REFACTOR** | Task 4 | Remaining loopholes closed, skill finalized |
