# JARVIS — Daily Brief Playbook

## Queries (run in order)

```bash
# 1. Urgent tasks
cortx query 'type = "task" and status = "open" and priority = "urgent"' --sort-by due:asc

# 2. Overdue
cortx query 'type = "task" and status != "done" and status != "archived" and due < today' --sort-by due:asc

# 3. Scheduled for today
cortx query 'type = "task" and status = "open" and scheduled <= today' --sort-by priority:desc

# 4. Inbox (unclarified)
cortx query 'type = "task" and status = "inbox"'

# 5. Active goals — check review staleness
# review_frequency = number of days (7=weekly, 14=biweekly, 30=monthly)
# A goal needs review when: (today - last_reviewed) > review_frequency
cortx query 'type = "goal" and status = "active"' --sort-by end_date:asc
```

## Output Format

```
## Daily Brief — [date]

### Urgent ([N])
- [task title] — due [date]

### Overdue ([N])
- [task title] — [N] days late

### Today ([N])
- [task title] ([context], [state], [duration]min)

### Inbox ([N] items need clarifying)
- [task title]

### Goals needing review ([N])
- [goal title] — last reviewed [N] days ago

### Upcoming deadlines
- [goal title] — due in [N] days, [N] open tasks
```

If all sections are empty, say: "All clear — nothing urgent, overdue, or scheduled today."
