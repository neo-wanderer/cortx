# Cerebrum

> OpenWolf's learning memory. Updated automatically as the AI learns from interactions.
> Do not edit manually unless correcting an error.
> Last updated: 2026-04-04

## User Preferences

<!-- How the user likes things done. Code style, tools, patterns, communication. -->

## Key Learnings

- **Project:** cortx
- **Description:** [![CI](https://github.com/neo-wanderer/cortx/actions/workflows/ci.yml/badge.svg)](https://github.com/neo-wanderer/cortx/actions/workflows/ci.yml)

## Do-Not-Repeat

<!-- Mistakes made and corrected. Each entry prevents the same mistake recurring. -->
<!-- Format: [YYYY-MM-DD] Description of what went wrong and what to do instead. -->

## Decision Log

<!-- Significant technical decisions with rationale. Why X was chosen over Y. -->

### [2026-04-04] Bidirectional relation atomicity: two-file lock sequence (Option A)

**Decision:** When writing a bidirectional link, acquire locks on both files before writing either (lock ordering: lower ID first to prevent deadlocks). Write child → write parent → release both locks.

**Tradeoffs accepted:**
- A crash between the two writes leaves a dangling reference. Acceptable because `cortx doctor links` exists as the repair path, and crashes are rare in a local CLI tool.
- No WAL (write-ahead log) — avoids real infrastructure complexity (replay logic, startup recovery) that isn't justified for this use case.
- No shadow-file + atomic rename — doesn't protect against a crash between child write and parent rename, so it offers no meaningful improvement over the simpler lock approach.

**Why not WAL:** Crash recovery gap is a known, accepted risk. The repair tool is the safety net.

### [2026-04-04] Human-readable filenames — title slug, fail on collision

**Decision:** Filenames are derived from the entity title as a slug (e.g., `buy-groceries.md`). No type prefix (folder encodes the type). No UUID suffix. On collision (file already exists), fail with a clear error — do not auto-append suffix.

**Date prefix:** Not built into the format. Callers (agents or humans) who want date context pass `--id 2026-04-04-buy-groceries` explicitly.

**Why no UUID suffix:** Forces intentional naming. Collisions are a signal to pick a more specific title, not something to silently work around.

### [2026-04-04] ID not stored in frontmatter — derived from filename at load time

**Decision:** Remove `id` from YAML frontmatter entirely. `Entity.id` is populated by `markdown.rs` from the filename stem when the file is loaded. Link references in other files continue to use the filename stem as the ID string.

**Why:** Clean Obsidian-compatible frontmatter (no synthetic fields). The markdown files can live directly inside an Obsidian vault. Eliminates the risk of filename/frontmatter ID divergence.

**Impact:** `entity.rs` `Entity::new()` needs an explicit `id` param (not derived from frontmatter). `markdown.rs` `read_entity()` passes `path.file_stem()` as the ID. `create.rs` stops inserting `id` into frontmatter. `types.yaml` drops `id` field from all types.

### [2026-04-04] Relationship schema: inline on owning field (Option A)

**Decision:** Bidirectionality, inverse field name, and polymorphic targets are declared inline on the owning field in `types.yaml`, not in a separate top-level `relations` block.

**Cardinality inference rules (must be documented):**
- `type: link` → inverse inferred as `array[link]` (many-to-one)
- `type: "array[link]"` → inverse inferred as `array[link]` (many-to-many)
- `type: link` + `inverse_one: true` → inverse is `link` (one-to-one)

**Polymorphic bidirectional:** `ref` becomes a map of `{ entity_type: { inverse: field_name } }` when multiple target types are needed.
