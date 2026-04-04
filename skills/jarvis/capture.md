# JARVIS — Capture & Clarify Playbook

## Capture Workflow

Everything lands in inbox first. Never skip this step.

```
1. CREATE   → cortx create task --title "..." --set status=inbox  (must be explicit — default is "open")
2. CLARIFY  → Run clarify checklist from second-brain-protocol on each inbox item
3. ORGANIZE → Set goal, context, energy, state, priority; move status to open/someday/waiting
4. CONFIRM  → Report what was captured and any open questions
```

## Clarify in Batch

When processing multiple inbox items (e.g. after a brain dump or morning triage):

```bash
# Get all inbox items
cortx query 'type = "task" and status = "inbox"' --sort-by created_at:asc
```

For each item, apply the clarify checklist from `second-brain-protocol`, then:

```bash
# Move to open with context
cortx update <id> --set status=open --set context=computer --set state=flow --set priority=high

# Move to someday
cortx update <id> --set status=someday

# Move to waiting
cortx update <id> --set status=waiting --set assignee=<person-id>

# Link to existing goal
cortx update <id> --set goal=<goal-id>
```

## Creating Goals During Clarify

If an inbox item reveals a series of actions with no existing goal:

```bash
# 1. Create the goal first
cortx create goal --title "Migrate auth service" \
  --set type_val=goal --set kind=time-bound --set status=active \
  --set area=<area-id> --set start_date=2026-04-07 --set end_date=2026-05-30 \
  --set priority=high

# 2. Link the task to the new goal
cortx update <task-id> --set goal=<new-goal-id> --set status=open
```
