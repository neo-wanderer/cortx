# JARVIS — Proactive Nudges Playbook

Check these after every write operation. Surface any that trigger.

## Nudge Checks

```bash
# Goal with no open tasks
cortx query 'type = "goal" and status = "active"'
# → for each, check: cortx query 'type = "task" and goal = "<id>" and status = "open"'
# → if empty: "Goal '[title]' has no open tasks — add a next action?"

# Task in_progress too long (> 7 days since start_date)
cortx query 'type = "task" and status = "in_progress"'
# → check start_date manually; if > 7 days ago: "Task '[title]' has been in progress for N days — still relevant?"

# Goal end_date approaching (within 7 days)
# Compute today + 7 days (e.g., 2026-04-04 → 2026-04-11) and substitute:
cortx query 'type = "goal" and status = "active" and end_date between ["today", "<today+7>"]' --sort-by end_date:asc
# → for each result: count open tasks; surface: "Goal '[title]' is due in N days with N open tasks remaining"

# Goal overdue for review
# review_frequency is a number of days (7 = weekly, 14 = biweekly, 30 = monthly)
# Compare: last_reviewed + review_frequency days < today → overdue
cortx query 'type = "goal" and status = "active"'
# → for each: if (today - last_reviewed) > review_frequency → "Goal '[title]' hasn't been reviewed in N days"

# Large inbox
cortx query 'type = "task" and status = "inbox"'
# → if count > 10: "You have N unprocessed inbox items"

# Nothing scheduled today
cortx query 'type = "task" and status = "open" and scheduled <= today'
# → if empty: "Nothing scheduled for today — want me to suggest tasks?"
```

## Nudge Format

Surface nudges as a brief block after the main response:

```
---
**Heads up:**
- Goal 'Launch v2.0' is due in 3 days with 5 open tasks remaining
- Task 'Review competitor analysis' has been in progress for 9 days
```

**Relevance filter:** Run all checks, but only surface nudges connected to the write that just happened:

| Write type | Surface these nudge checks |
|---|---|
| Created/updated a task | goal-no-open-tasks, large-inbox, nothing-scheduled |
| Moved task to `in_progress` | task-in-progress-too-long (on start) |
| Completed a task | goal-no-open-tasks, goal-end-date-approaching |
| Created/updated a goal | goal-end-date-approaching, goal-overdue-for-review |
| Captured a batch (brain dump, meeting notes) | large-inbox, nothing-scheduled |

Don't repeat nudges surfaced in the immediately preceding response.
