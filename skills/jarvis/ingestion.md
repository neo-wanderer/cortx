# JARVIS — Input Ingestion Playbook

## Source Classification

Every input arrives from a source. Classify before acting:

| Source | Default action |
|---|---|
| Direct conversation | Capture to task (inbox) or note (kind=quick) |
| Meeting notes pushed | Create note (kind=meeting) + extract action items to inbox |
| Email dump | Extract actionable items → tasks (inbox); reference → notes |
| Web research summary | Create note (kind=research) or resource, link to relevant goal |
| Brain dump (unstructured) | Process line by line — see below |
| Calendar event occurred | Create log (kind=meeting) with attendees and summary |

## Brain Dump Processing

Process each line independently using the classification decision tree from `second-brain-protocol`:

1. Read the full dump first — identify themes before creating entities
2. Group related items: will they become one goal + tasks, or separate tasks?
3. For each item: classify → create entity → link to existing goal/area if found
4. Report a structured summary when done: N tasks captured, N notes created, N open questions

## Output Rules

- Single routine write → silent confirmation: "Created task: Review Q1 budget"
- Complex input (meeting notes, brain dump) → structured summary:
  ```
  ## Captured from [source]
  - [N] tasks added to inbox
  - [N] notes created
  - [N] resources saved
  **Open questions:** [anything ambiguous that needs user input]
  ```
- Conflict, blocker, or deadline surfaced → proactive structured output regardless of input type
