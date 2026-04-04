# Schema Relations & Filename Design Spec

**Date:** 2026-04-04
**Status:** Approved

---

## Overview

This spec covers three related changes to cortx's schema and storage model:

1. **Relationship schema enrichment** — `types.yaml` gains first-class relation fields with cardinality, bidirectionality, and polymorphic target support.
2. **ID removed from frontmatter** — entity identity is derived from the filename at load time; frontmatter stays clean and Obsidian-compatible.
3. **Human-readable filenames** — UUID suffixes replaced by title slugs.

These three changes are coupled: removing ID from frontmatter only makes sense once filenames are stable, human-readable identifiers.

---

## 1. Relationship Schema in `types.yaml`

### 1.1 Single-type relation (existing, extended)

```yaml
task:
  fields:
    goal:
      type: link
      ref: goal
      bidirectional: true
      inverse: tasks        # field name on goal entity
```

### 1.2 Polymorphic relation (new)

When a field can point to multiple entity types, `ref` becomes a map keyed by entity type. Each entry declares the inverse field name on that target type.

```yaml
note:
  fields:
    related:
      type: link
      ref:
        goal: { inverse: related_notes }
        task: { inverse: related_notes }
        area: { inverse: related_notes }
      bidirectional: true
```

Unidirectional polymorphic (no inverse maintenance needed):

```yaml
log:
  fields:
    subject:
      type: link
      ref: [goal, task, note, resource]   # array shorthand, no inverse
```

### 1.3 Cardinality

> **Important distinction:** `ref: [a, b, c]` declares *allowed target types* (polymorphic) — it does not affect cardinality. Cardinality is always controlled by `type: link` (holds one reference) vs `type: "array[link]"` (holds many references).

Cardinality is expressed by the field type on the owning side. The inverse cardinality is inferred.

| Owning side type | Inverse inferred as | Relation pattern |
|---|---|---|
| `link` | `array[link]` | many-to-one (default) |
| `array[link]` | `array[link]` | many-to-many |
| `link` + `inverse_one: true` | `link` | one-to-one |

**Rule:** When the owning side is a single `link`, the parent side will accumulate many references — so its inverse defaults to `array[link]`. Explicit `inverse_one: true` opts into true one-to-one (both sides hold a single reference).

### 1.4 Schema type changes (`src/schema/types.rs`)

`FieldType::Link` is extended:

```rust
pub enum FieldType {
    // ...existing variants...
    Link(LinkDef),
    ArrayLink(LinkDef),
}

pub struct LinkDef {
    /// Single target type, or multiple for polymorphic relations.
    pub targets: LinkTargets,
    pub bidirectional: bool,
    /// For one-to-one: override inverse inference to single link.
    pub inverse_one: bool,
}

pub enum LinkTargets {
    /// ref: goal  (single, non-polymorphic)
    Single { ref_type: String, inverse: Option<String> },
    /// ref: { goal: { inverse: related_notes }, task: { ... } }
    Poly(Vec<PolyTarget>),
}

pub struct PolyTarget {
    pub ref_type: String,
    pub inverse: Option<String>,
}
```

### 1.5 `types.yaml` syntax summary

```yaml
# Single bidirectional many-to-one
goal: { type: link, ref: goal, bidirectional: true, inverse: tasks }

# Single unidirectional
area: { type: link, ref: area }

# Array bidirectional many-to-many
related_goals:
  type: "array[link]"
  ref: goal
  bidirectional: true
  inverse: related_notes

# Polymorphic bidirectional
related:
  type: link
  ref:
    goal: { inverse: related_notes }
    task: { inverse: related_notes }
  bidirectional: true

# Polymorphic unidirectional (array shorthand)
subject:
  type: link
  ref: [goal, task, note]

# One-to-one
emergency_contact:
  type: link
  ref: person
  bidirectional: true
  inverse: emergency_contact_for
  inverse_one: true
```

---

## 2. Atomic Bidirectional Writes

When writing a bidirectional link field, cortx must update two files: the owning entity and the referenced entity (to maintain the inverse array). Both updates must appear to succeed or fail together.

### Strategy: Two-file lock sequence

1. Determine both file paths (owning entity + referenced entity).
2. Acquire `FileLock` on both files. **Lock ordering:** always lock the lexicographically lower filename first to prevent deadlocks between concurrent writers.
3. Write the owning entity file.
4. Read the referenced entity, append/remove the back-reference, write it.
5. Release both locks.

### Crash recovery

A process crash between step 3 and step 4 leaves the inverse field out of sync. This is an accepted risk. `cortx doctor links` is the repair path: it scans all bidirectional field declarations, verifies both sides are consistent, and reports (or optionally repairs) any divergence.

### Non-bidirectional writes

Unidirectional link fields (`bidirectional: false` or absent) write only the owning file. No lock ordering rules apply beyond the existing single-file lock.

---

## 3. ID Removed from Frontmatter

### Motivation

- Frontmatter with an `id` field is a cortx-specific convention that breaks Obsidian compatibility.
- The filename and the frontmatter ID could diverge (e.g., manual rename in Obsidian).
- The entity type is already encoded in the folder; the ID is already encoded in the filename.

### New behavior

- `id` is **not written** to frontmatter on create.
- `id` is **not required** in `types.yaml` field definitions (remove from all `required:` lists and `fields:` sections).
- `Entity.id` is populated from `path.file_stem()` in `markdown.rs` when an entity is loaded.
- `Entity::new()` gains an explicit `id: String` parameter (no longer derived from frontmatter).

### Link references

Link fields in frontmatter store the filename stem of the referenced entity (same as before — the ID format just changes from `task-20260403-a1b2c3d4` to `buy-groceries`). No change to how links are stored or queried.

---

## 4. Human-Readable Filenames

### Format

```
{title-slug}.md
```

Where `title-slug` is the entity title lowercased with spaces replaced by hyphens and non-alphanumeric characters stripped.

Examples:
- `"Buy groceries"` → `buy-groceries.md`
- `"Q2 Planning"` → `q2-planning.md`
- `"Meeting: John @ Acme"` → `meeting-john-acme.md`

**No type prefix.** The folder encodes the type (`1_Projects/tasks/`). A type prefix in the filename is redundant and clutters Obsidian's file explorer.

**No UUID suffix.** Collisions are a signal to choose a more specific title, not something to silently work around.

### Collision handling

If `{slug}.md` already exists in the target folder, the create command fails with:

```
Error: 'buy-groceries.md' already exists in 1_Projects/tasks/.
Use --id to specify a unique name (e.g., --id buy-groceries-2).
```

### Custom IDs and date prefixes

Callers who need date context or disambiguation pass `--id` explicitly:

```bash
cortx create task --title "Buy groceries" --id 2026-04-04-buy-groceries
cortx create note  --title "Meeting notes" --id 2026-04-04-acme-kickoff
```

This keeps the CLI simple and gives agents full control over naming conventions.

---

## 5. Open Questions

- [ ] `cortx doctor links` repair mode: report-only vs. auto-repair flag?
- [ ] Slug generation: should Unicode characters be transliterated or stripped?
- [ ] When renaming a file (title change), should cortx offer a `cortx rename <id> <new-id>` command that updates all back-references atomically?

---

## 6. Affected Files

| File | Change |
|---|---|
| `types.yaml` | Remove `id` field from all types; update `required` lists; add relation metadata to link fields |
| `src/schema/types.rs` | Replace `FieldType::Link` with `Link(LinkDef)` / `ArrayLink(LinkDef)` |
| `src/schema/registry.rs` | Parse new relation syntax from YAML |
| `src/schema/validation.rs` | Validate link fields against new `LinkDef` |
| `src/entity.rs` | `Entity::new()` takes explicit `id` param; remove frontmatter derivation |
| `src/storage/markdown.rs` | Derive ID from `file_stem()`; implement two-file lock for bidirectional writes |
| `src/cli/create.rs` | Generate slug from title; remove `id` insertion into frontmatter; collision error |
| `src/cli/doctor.rs` | Add `links` subcommand validation for bidirectional consistency |
| `tests/` | Update all fixtures and assertions for new ID/filename format |
