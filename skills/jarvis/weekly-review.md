# JARVIS — Weekly Review Playbook

Walk through each step in order. Pause for user input after each step before proceeding.

## Step 1 — Clear Inbox

```bash
cortx query 'type = "task" and status = "inbox"'
```

For each item, apply the clarify checklist (from `second-brain-protocol`) and move to:
- `open` — next action, ready to do
- `someday` — not committed yet
- `waiting` — delegated, set assignee
- `done` — already completed
- Archive if irrelevant (`cortx archive <id>`)

## Step 2 — Review Active Goals

```bash
cortx query 'type = "goal" and status = "active"' --sort-by end_date:asc
```

For each goal:
```bash
# Open tasks remaining
cortx query 'type = "task" and goal = "<id>" and status = "open"' --sort-by priority:desc

# Any blockers?
cortx query 'type = "task" and goal = "<id>" and status = "open" and tags contains "blocker"'

# Mark as reviewed
cortx update <id> --set last_reviewed=today
```

Questions to surface per goal:
- Is this goal still relevant?
- Are tasks moving forward?
- Does the end_date need adjusting?
- Should it be paused or cancelled?

## Step 3 — Review Someday/Maybe

```bash
cortx query 'type = "task" and status = "someday"'
```

For each: promote to `open`, archive (`cortx archive <id>`), or keep as `someday`.

## Step 4 — Review Waiting

```bash
cortx query 'type = "task" and status = "waiting"'
```

For each: follow up (add a nudge note) or close if resolved.

## Step 5 — Look Ahead (next 7 days)

Compute today + 7 days (e.g., today=2026-04-04 → window end = 2026-04-11) and substitute in queries:

```bash
# Tasks scheduled this week
cortx query 'type = "task" and status = "open" and scheduled between ["today", "<today+7>"]' --sort-by scheduled:asc

# Goals ending this week
cortx query 'type = "goal" and status = "active" and end_date between ["today", "<today+7>"]' --sort-by end_date:asc
```

For each goal returned, count its open tasks:
```bash
cortx query 'type = "task" and goal = "<id>" and status = "open"'
```

Flag: "Goal '[title]' ends in N days with N open tasks remaining."

## Step 6 — Close

Summarize the review, then log it:

```bash
cortx create log --title "Weekly Review" \
  --set kind=update --set date=today --set impact=positive \
  --set summary="[N] tasks clarified, [N] goals reviewed, [N] promoted from someday"
```
