# JARVIS Skill Design Spec

**Date:** 2026-04-04  
**Status:** Draft  

---

## Overview

JARVIS is an AI agent that manages the user's Second Brain — a shared, headless knowledge vault built on cortx. It implements PARA + GTD methodology and acts as the primary bookkeeper, life manager, and orchestrator across a multi-agent system.

This spec covers:
1. The three-skill architecture
2. The vault schema (entity types + fields)
3. Relation model (bidirectional + related)
4. Behavioral rules for JARVIS

---

## 1. Three-Skill Architecture

Three skills are layered so agents load only what they need:

```
Layer 1 (all agents):    using-cortx-cli        ← CLI mechanics, query syntax
Layer 2 (all agents):    second-brain-protocol  ← PARA+GTD domain knowledge, entity conventions
Layer 3 (JARVIS only):   jarvis                 ← Reviews, daily brief, orchestration, identity
```

### Who loads what

| Agent | Skills |
|---|---|
| Read-only query agent | `using-cortx-cli` + `second-brain-protocol` |
| Email agent | `using-cortx-cli` + `second-brain-protocol` |
| Research/web agent | `using-cortx-cli` + `second-brain-protocol` |
| Meeting notes agent | `using-cortx-cli` + `second-brain-protocol` |
| JARVIS | all three |

**Key principle:** `second-brain-protocol` gives any agent the domain knowledge to write consistently and query intelligently. JARVIS builds on top with orchestration logic.

---

## 2. Vault Schema

### 2.1 Entity Types (8 total)

#### `area`
Ongoing life domain — no end date, no status lifecycle.

| Field | Type | Notes |
|---|---|---|
| title | string | required |
| up | link: area | parent area (sub-area support) |
| archived | boolean | default: false |
| tags | array[string] | required, default: [] |

#### `goal`
A goal or milestone. Goals can have milestones (via `up`). No separate project entity — tasks attach directly to goals or milestones.

| Field | Type | Notes |
|---|---|---|
| title | string | required |
| type | enum [goal/milestone] | required |
| kind | enum [time-bound/ongoing] | required |
| status | enum [active/paused/completed/cancelled/archived] | required |
| area | link: area | primary area |
| up | link: goal | parent goal (for milestones) |
| priority | enum [low/medium/high/urgent] | |
| start_date | date | required when kind=time-bound |
| end_date | date | required when kind=time-bound |
| review_frequency | number (days) | how often to review |
| last_reviewed | date | updated by JARVIS on review |
| objective | text | what does success look like? |
| related | array[link: goal] | peer goals |
| related_notes | array[link: note] | |
| related_resources | array[link: resource] | |
| tags | array[string] | required, default: [] |

#### `task`
A single actionable item. Default status is `inbox` — everything is captured first, then clarified.

| Field | Type | Notes |
|---|---|---|
| title | string | required — must be a concrete next physical action |
| status | enum [inbox/open/someday/in_progress/waiting/done/cancelled/archived] | default: inbox |
| goal | link: goal | optional — null = inbox/unassigned |
| up | link: task | parent task (sub-task support) |
| priority | enum [low/medium/high/urgent] | |
| context | enum [home/office/computer/phone/errands/anywhere/meeting] | GTD @context |
| energy | enum [high/medium/low] | energy level required |
| state | enum [easy/quick/flow] | type of effort required |
| duration | number (minutes) | time estimate |
| scheduled | date | when to appear in daily list |
| start_date | date | auto-set when status → in_progress |
| end_date | date | auto-set when status → done |
| assignee | link: person | who is doing it / waiting for |
| tags | array[string] | required, default: [] |

**GTD status lifecycle:**
```
capture   → inbox
clarify   → open (next action) / someday / waiting
engage    → in_progress
complete  → done / cancelled / archived
```

**State meanings:**
- `easy` — low cognitive load, can do on autopilot
- `quick` — short burst, pairs with duration field
- `flow` — requires deep uninterrupted focus

**Behavioral rules (enforced by JARVIS/agents):**
- When status → `in_progress`: set `start_date = today`
- When status → `done`: set `end_date = today`

#### `note`
A knowledge artifact. Kinds are structural (what type of note), tags are semantic (what it means).

| Field | Type | Notes |
|---|---|---|
| title | string | required |
| status | enum [draft/in_progress/done/archived] | default: draft |
| kind | enum [journal/meeting/people/project/area/research/quick/interview/permanent/structure] | |
| area | link: area | primary area |
| goal | link: goal | primary goal |
| people | array[link: person] | attendees, subjects |
| resources | array[link: resource] | referenced resources |
| related_goals | array[link: goal] | secondary goals |
| related_notes | array[link: note] | related notes |
| related_resources | array[link: resource] | related resources |
| related_areas | array[link: area] | related areas |
| tags | array[string] | required, default: [] |

**Note kind meanings:**
- `journal` — daily log, reflections
- `meeting` — notes from a meeting
- `people` — CRM-style note about a person
- `project` — notes scoped to a goal
- `area` — notes scoped to a life area
- `research` — findings from investigation or web research
- `quick` — fleeting capture, inbox item
- `interview` — job or user interview notes
- `permanent` — Zettelkasten evergreen note, atomic refined idea
- `structure` — Zettelkasten MOC (Map of Content), index linking related notes

#### `resource`
Reference material — links, files, videos, images, documents.

| Field | Type | Notes |
|---|---|---|
| title | string | required |
| kind | enum [link/video/file/image/document/article] | |
| area | link: area | primary area |
| goal | link: goal | primary goal |
| ref | string | URL or file path |
| related_goals | array[link: goal] | |
| related_notes | array[link: note] | |
| related_resources | array[link: resource] | |
| related_areas | array[link: area] | |
| tags | array[string] | required, default: [] |

#### `log`
A timestamped event on the timeline — something that *happened*. Distinct from notes (which are knowledge artifacts).

| Field | Type | Notes |
|---|---|---|
| title | string | required |
| date | date | when the event occurred |
| kind | enum [risk/meeting/decision/milestone/update/critical] | |
| impact | enum [positive/neutral/negative] | |
| summary | text | brief description |
| url | string | optional reference |
| goal | link: goal | related goal |
| task | link: task | related task |
| note | link: note | related note |
| resource | link: resource | related resource |
| people | array[link: person] | people involved |
| tags | array[string] | required, default: [] |

**Note vs Log distinction:**
- Log = something that *happened* (timestamped event)
- Note = something you *know or think* (knowledge artifact)
- `insight`, `blocker`, `retrospective` → notes with tags, not log kinds

#### `person`
A contact in your life — personal, professional, or family.

| Field | Type | Notes |
|---|---|---|
| name | string | required |
| relationship | enum [personal/professional/family/other] | |
| company | link: company | |
| email | string | |
| phone | string | |
| tags | array[string] | required, default: [] |

#### `company`
An organization.

| Field | Type | Notes |
|---|---|---|
| name | string | required |
| domain | string | |
| industry | string | |
| tags | array[string] | required, default: [] |

---

### 2.2 Relation Model

#### Bidirectional Relations (cortx maintains both sides transactionally)

The child entity owns the reference. cortx atomically updates both sides on write.

| Child field | Parent inverse | 
|---|---|
| `goal.area` | `area.goals` |
| `goal.up` | `goal.milestones` |
| `task.goal` | `goal.tasks` |
| `task.up` | `task.sub_tasks` |
| `note.area` | `area.notes` |
| `note.goal` | `goal.notes` |
| `note.resources` | `resource.notes` |
| `resource.area` | `area.resources` |
| `person.company` | `company.people` |

**cortx requirement:** Bidirectional relation updates must be atomic (file-locked). A partial write (child updated, parent not) must not be possible.

#### Unidirectional Relations (discovery only, no inverse)

`related_*` fields on notes, resources, and goals are unidirectional. Use queries to discover:

```bash
# All notes related to a goal
cortx query 'type = "note" and related_goals contains "goal-xyz"'

# Timeline for a goal
cortx query 'type = "log" and goal = "goal-xyz"' --sort-by date:asc
```

#### cortx Requirements (new capabilities needed)

1. **Transactional bidirectional relations** — schema declares `inverse` field; cortx maintains both sides atomically
2. **Polymorphic array links** — `related_*` fields reference typed entity IDs, queryable per type
3. **Array link fields** — `people`, `resources`, `related_*` are arrays of links, not single links

---

## 3. Three Skills — Content Outline

### `using-cortx-cli` (existing, update needed)
Update to reflect new entity types: `goal`, `log`. Add `goal` to query recipes.

### `second-brain-protocol` (new)

Sections:
1. **PARA Entity Map** — what each entity type means in life terms
2. **GTD Workflow** — capture → clarify → organize → reflect → engage in cortx terms
3. **Classification Decision Tree** — how to decide what entity type incoming info becomes
4. **Entity Conventions** — required fields, linking rules, status lifecycle per type
5. **Relation Rules** — primary (goal field) vs related (related_* fields)
6. **Tagging Philosophy** — tags are semantic, kinds are structural
7. **What NOT to Do** — orphaned tasks, skipping inbox, free-form kinds

### `jarvis` (new)

Sections:
1. **Identity & Role** — JARVIS is the primary bookkeeper; other agents write, JARVIS maintains integrity
2. **Multi-Source Ingestion** — how to process: meeting notes, email dumps, brain dumps, web research
3. **Capture Workflow** — everything lands in inbox first, then clarified
4. **Output Rules** — when to be silent vs structured vs proactive
5. **Daily Brief** — what to surface every morning
6. **Weekly Review Ritual** — structured walkthrough (GTD weekly review adapted)
7. **Proactive Nudges** — when to surface insights without being asked
8. **Prioritization Logic** — how to answer "what should I work on?"
9. **State-Based Task Surfacing** — using context + energy + state + duration to match tasks to mood/time

---

## 4. Life Areas (Vault Initialization)

Based on known life domains, initialize the vault with these areas:

- Health
- Personal
- Finance
- Work
- Family (Nilanya)

Each area is created with `archived=false` and appropriate tags.

---

## 5. Decisions

- [x] **Log relations:** Unidirectional only. Logs reference parents; parents do not store `logs` inverse. Timeline queries via `cortx query 'type = "log" and goal = "<id>"'`.
- [x] **Notion ↔ cortx sync:** Deferred. cortx is source of truth for now.
- [x] **`review_frequency` defaults:** No defaults. Always required when creating a goal — JARVIS prompts if not provided at creation time.

---

## 6. Next Steps

1. Update `types.yaml` with new schema (goal, log entities + field additions)
2. Implement cortx bidirectional relation support
3. Write `second-brain-protocol` skill
4. Write `jarvis` skill
5. Update `using-cortx-cli` skill with new entity types
