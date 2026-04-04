# JARVIS — Prioritization Playbook

## When User Asks "What Should I Work On?"

Run in order, stop at first non-empty result:

```bash
# 1. Urgent
cortx query 'type = "task" and status = "open" and priority = "urgent"'

# 2. Overdue
cortx query 'type = "task" and status = "open" and due < today' --sort-by due:asc

# 3. Scheduled today
cortx query 'type = "task" and status = "open" and scheduled <= today'

# 4. Open by priority + due date
cortx query 'type = "task" and status = "open"' --sort-by priority:desc,due:asc
```

Present top 3-5 tasks with context, state, and duration so the user can pick.

## State-Based Surfacing

Match tasks to the user's current mental mode:

| User says | Query filter |
|---|---|
| "I have 5 minutes" | `state = "quick" and duration <= 5` |
| "I'm tired" / "easy tasks" | `state = "easy" and energy = "low"` |
| "I have solid 30 mins" / "focus time" | `state = "flow"` |
| "Quick wins" | `state = "quick"` sort by `duration:asc` |
| "I'm at my computer" | `context = "computer"` |
| "I'm heading out" / "on the go" | `context in ["errands", "phone"]` |
| "In a meeting" | `context = "meeting"` |

Always combine with `status = "open"` and priority sort:

```bash
# 30 mins deep focus
cortx query 'type = "task" and status = "open" and state = "flow"' --sort-by priority:desc,due:asc

# Quick wins
cortx query 'type = "task" and status = "open" and state = "quick"' --sort-by duration:asc

# Low energy at home
cortx query 'type = "task" and status = "open" and state = "easy" and energy = "low" and context = "home"' --sort-by priority:desc
```

## Multi-Axis Matching

For the best recommendation, combine all available signals:

```bash
# Example: user is at computer, has 45 mins, high energy
cortx query 'type = "task" and status = "open" and context = "computer" and state = "flow" and energy = "high"' \
  --sort-by priority:desc,due:asc
```

If the combined query returns nothing, relax constraints one at a time (drop `context`, then `energy`, then `state`) until results appear.
