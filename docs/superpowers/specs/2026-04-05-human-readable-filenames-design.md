# Human-Readable Filenames & Wikilink Relations — Design Spec

**Date:** 2026-04-05
**Status:** Approved, ready for plan

## Problem

cortx stores entities as `slug.md` files (`buy-groceries.md`) with link fields referencing slugs (`project: buy-groceries`). This is machine-friendly but creates two pain points:

1. **Obsidian interop is broken.** Obsidian only renders `[[...]]` as clickable links. Bare slug strings show up as plain text — no graph view, no backlinks, no rename propagation.
2. **Human readability suffers.** Users and AI agents think in titles ("Buy Groceries"), but every reference goes through a slug transformation that loses the natural name.

The project is pre-production. There are no external users to migrate. A clean cut-over is acceptable.

## Goal

One name per entity. The human-readable title is the entity's identity — it names the file, forms the wikilink target, and is what users type in CLI commands and queries. Link fields in frontmatter are stored as Obsidian-compatible `"[[Title]]"` strings so the vault works natively as an Obsidian vault without any configuration.

## Non-Goals

- Backward compatibility with slug-based vaults. No migration command.
- Changing folder-per-type layout. `types.yaml` schema format is unchanged.
- Changing query language surface. Users still write bare values like `project = "Website Redesign"`.
- Moving archived entities to an `archive/` folder. Out of scope.

## Design

### Core model

One identifier per entity, derived deterministically from the title:

```
title:    "Meeting: Q2 Review"     (frontmatter, raw)
id:       "Meeting Q2 Review"      (filesystem-safe, derived)
filename: "Meeting Q2 Review.md"
wikilink: "[[Meeting Q2 Review]]"
CLI arg:  cortx show "Meeting Q2 Review"
```

**Sanitization rule** (`id = sanitize(title)`):
- Replace each of `/ \ : * ? " < > |` and ASCII control chars with a single space
- Replace trailing dots (Windows constraint)
- Collapse consecutive whitespace to single spaces
- Trim leading/trailing whitespace
- Preserve uppercase, unicode, punctuation that's filesystem-safe (`.`, `-`, `_`, `'`, `(`, `)`, `!`, `,`, etc.)

**Global uniqueness:** no two entities in the vault may share an `id`. Check is case-insensitive across all type folders (for cross-OS portability). On collision at create time, cortx errors and the user retries with a different title.

**Entity identity in code:**
- `Entity::id` = filename stem without `.md`; the sanitized, filename-safe form
- `Entity::title` = raw frontmatter `title` value
- Invariant: `sanitize(entity.title) == entity.id`. Doctor checks for drift.
- The `id` field is never stored in frontmatter. It is always derived from the filename.

### Storage format

Link-typed fields in frontmatter are wrapped as wikilink strings:

```yaml
---
title: Buy Groceries
status: open
project: "[[Website Redesign]]"
related:
  - "[[Weekly Review]]"
  - "[[Meal Planning]]"
assignee: "[[Alice Chen]]"
---
```

- `FieldType::Link` → scalar YAML string `"[[Target Title]]"`
- `FieldType::ArrayLink` → YAML sequence of such strings
- Empty arrays stay `[]`, missing/null fields stay absent
- Non-link fields (dates, enums, `ArrayString`, etc.) are unchanged

**Quoting:** always emit with double quotes. Obsidian accepts both forms, but `serde_yaml` would mis-parse unquoted `[[...]]` as a nested sequence. Deterministic output, one canonical form.

**Body wikilinks:** freeform. Users may write `[[Any Title]]` anywhere in note bodies. cortx treats body wikilinks as first-class references for the rename cascade but does not validate them against schema `ref:` targets (schema constraints apply only to frontmatter link fields).

### Read path (unwrapping)

Unwrapping is schema-driven and happens once during entity load. The reader consults the `TypeRegistry` to know which fields are `Link`/`ArrayLink`. For those fields only:

```
"[[Buy Groceries]]"         →  Value::String("Buy Groceries")
["[[A]]", "[[B]]"]          →  Value::Array([String("A"), String("B")])
```

Non-link string fields that happen to contain literal `[[...]]` are left alone (e.g., a `notes` field with `"see [[foo]] for details"`).

**Location:** inside `Entity::load` (or equivalent in `src/storage/markdown.rs`), after raw YAML parsing but before the entity is returned. Downstream code — query engine, evaluator, sort, doctor, display — operates on bare titles and is unaware of wikilink syntax.

**Edge cases handled by the unwrapper:**
- Malformed wrapper (`"[[Buy Groceries"` missing close): entity fails to load with a parse error; doctor reports
- Empty wrapper (`"[[]]"`): treated as null for that field; doctor flags
- Whitespace inside brackets: trimmed (`"[[  Buy Groceries  ]]"` → `"Buy Groceries"`)
- Piped form (`"[[buy-groceries|Buy Groceries]]"`): rejected at load time; cortx never emits piped links, and doctor flags any found

### Write path (wrapping + validation)

User input is always bare titles. cortx wraps on write.

Pipeline for each link-typed field assignment:

```
Input:     project="Website Redesign"
Validate:  look up "Website Redesign" in allowed target folders (from schema ref:)
           → found? continue.
           → not found? reject with clear error.
Wrap:      Value::String("Website Redesign") → YAML string "[[Website Redesign]]"
Emit:      frontmatter field `project: "[[Website Redesign]]"`
```

**Validation error example:**
```
$ cortx update "Buy Groceries" --set project="Webiste Redesign"
error: no project found with title "Webiste Redesign"
hint: did you mean "Website Redesign"? (closest match)
```

**Escape hatch:** `--no-validate-links` flag bypasses the target-existence check. Off by default. Reserved for future bulk-import scenarios.

**Polymorphic link targets** (schema `ref: [goal, task]`): step 1 tries each allowed target folder in order; first match wins; zero matches → reject. Under global uniqueness, at most one match is possible anyway.

**Atomic writes:** existing `FileLock` RAII pattern continues to protect per-file writes. No change to locking strategy.

### CLI changes

**Commands that reference an existing entity take the title as positional arg:**

```bash
cortx show "Buy Groceries"
cortx update "Buy Groceries" --set status=done --set due=2026-04-10
cortx archive "Buy Groceries"
cortx delete "Buy Groceries" --force
cortx rename "Buy Groceries" "Weekly Groceries"
cortx note headings "Morning Thoughts"
cortx note insert-after-heading "Morning Thoughts" --heading "Ideas" --content "..."
cortx note read-lines "Morning Thoughts" --start 1 --end 20
```

**Resolution semantics:**
- Case-insensitive match on lookup (matches the uniqueness-check invariant)
- Scan all type folders; return unique match
- Errors: `"no entity found with title X"` (zero matches) or `"multiple entities match X — vault invariant violated, run cortx doctor filenames"` (defensive; should be impossible)

**`create` keeps `--type` (unchanged behavior):**
```bash
cortx create task --title "Buy Groceries" --set project="Website Redesign"
```
- Computes `id = sanitize(title)`, verifies no global collision, writes `tasks/Buy Groceries.md`
- Link-typed `--set` values go through the write-path validation

**`update --set title=...` is rejected** with:
```
error: use 'cortx rename' to change an entity's title
```
Renaming has cross-file consequences that `update` is not allowed to perform.

**Query language surface is unchanged** — users still write bare values:
```bash
cortx query 'project = "Website Redesign" and status != "done"'
cortx query 'related contains "Weekly Review"'
```
The read-path unwrapping makes this transparent to queries.

### Rename cascade

`cortx rename <old-title> <new-title>` is a new top-level command. It is the only way to change an entity's title.

**Pipeline:**

1. Resolve `<old-title>` → `old_id` → `old_path`
2. Compute `new_id = sanitize(new-title)`
3. Collision check: `new_id` must not exist elsewhere in vault (case-insensitive scan). On collision → error.
4. Plan the rewrite:
   - File rename: `tasks/Buy Groceries.md` → `tasks/Weekly Groceries.md`
   - Update `title` field inside the renamed file's frontmatter
   - Scan every entity file in the vault. For each, inspect all link-typed frontmatter fields (from schema): replace `"[[Buy Groceries]]"` with `"[[Weekly Groceries]]"`.
   - Scan every `.md` file in the vault for body wikilinks `[[Buy Groceries]]` (case-sensitive literal match; the old id is the exact filename stem). Replace with `[[Weekly Groceries]]`.
5. Execute all edits. On any error, roll back (see below).

**Transactionality:**
- Before modifying anything, copy each file that will be modified into a temporary directory keyed on the rename session id
- Apply edits in-place
- On success: delete the temp backups
- On any per-file error: restore all modified files from backups, surface the error

**Flags:**
- `--dry-run`: prints the full plan (file rename + every back-ref site), no writes
- `--skip-body`: rewrite frontmatter back-refs only; leave body wikilinks alone (power-user escape hatch for cases where body mentions must not propagate)

**Output:**
```
$ cortx rename "Buy Groceries" "Weekly Groceries"
renamed: tasks/Buy Groceries.md → tasks/Weekly Groceries.md
updated 3 back-references:
  projects/Household.md (related)
  notes/Meal Planning.md (body wikilink)
  tasks/Order Delivery.md (depends_on)
```

**Performance:** O(vault size) scan per rename. At 5k files the existing benchmarks show ~70ms for full-vault read; rename adds the write phase for modified files only. Acceptable for interactive use. A `rename_bench` will be added to validate the shape.

### Doctor checks

`cortx doctor` gains new subcommand `filenames` and new checks. Existing link checks adapt.

**New checks (under `cortx doctor filenames`):**

1. **Filename/title drift** — for every entity, verify `filename_stem == sanitize(entity.title)`. Catches manual filesystem edits or interrupted renames. `--fix` renames the file to match (simple single-file rename; does NOT trigger cascade — use `cortx rename` for that).

2. **Case-insensitive collision** — scan all filenames, flag any two that differ only in case. Cannot auto-fix (which title is canonical?); user picks a winner and manually renames.

3. **Wikilink format** — every value in a `Link`/`ArrayLink` field must match the canonical `"[[...]]"` form. Flag:
   - Bare strings (`project: website-redesign`)
   - Piped forms (`"[[slug|Display]]"`)
   - Malformed wrappers (unclosed, empty)
   `--fix` attempts to wrap bare strings whose value resolves to an existing entity title; rejects piped forms and malformed wrappers.

4. **Body wikilink integrity** (optional, `--check-bodies`) — scan note bodies for `[[...]]` tokens that do not resolve to any vault entity. Reports, no auto-fix. Body wikilinks are freeform and may intentionally point to nonexistent notes.

**Existing checks adapt:**

5. **Dangling links** (`doctor links`) — unchanged command; now operates on unwrapped titles (since read-path unwrapping runs before doctor sees values). Reports frontmatter link fields that don't resolve.

6. **Bidirectional inverse consistency** (`doctor links`) — unchanged command. `--fix` writes the wrapped form when inserting new back-refs into inverse fields.

**New CLI surface:**
```bash
cortx doctor validate                  # schema validation (unchanged)
cortx doctor links [--fix]             # dangling + bidirectional (adapted)
cortx doctor filenames [--fix] [--check-bodies]  # NEW
```

## Implementation Surface

Files most likely touched (not exhaustive; for orientation):

- `src/slug.rs` — add `sanitize_title()`; keep `slug_from_title()` or retire it
- `src/entity.rs` — read-path unwrapping hook; title/id invariant
- `src/storage/markdown.rs` — write-path wrapping, collision check, lookup-by-title
- `src/cli/create.rs` — collision check, link-target validation on `--set`
- `src/cli/update.rs` — reject `--set title=...`, link-target validation
- `src/cli/show.rs`, `archive.rs`, `delete.rs`, `note.rs` — accept title positional arg
- `src/cli/rename.rs` — NEW, full cascade
- `src/cli/doctor.rs` — new `filenames` subcommand, adapt link checks
- `src/value.rs` — wikilink-aware `to_yaml` / `from_yaml` for link-typed fields (or handled one level up in `Entity::load`)
- `src/query/evaluator.rs` — no change expected; operates on unwrapped values
- `tests/common/mod.rs` — `TestVault` helpers for title-based lookup
- `tests/*` — substantial adaptation of existing integration tests (slug → title)

## Testing Strategy

**Unit tests (new):**
- `slug::sanitize_title` — every illegal char class, whitespace collapsing, unicode preservation, empty/whitespace-only, trailing dots, idempotence (`sanitize(sanitize(x)) == sanitize(x)`)
- Wikilink wrap/unwrap round-trip for `Value::String` and `Value::Array` under each `FieldType` variant
- Malformed wikilink detection (unclosed, empty, piped)
- Case-insensitive collision detection

**Integration tests (new, in `tests/`):**
- `rename_test.rs` — full cascade happy path; rollback on mid-transaction failure; `--dry-run`; `--skip-body`; collision rejection; renaming touches both frontmatter back-refs and body wikilinks
- `wikilink_read_write_test.rs` — create entity with link fields; load it back; verify unwrap produces bare titles; verify write-path validation rejects dangling refs; verify `--no-validate-links` bypasses
- `cli_integration_test.rs` additions — `show/update/archive/delete` by title with spaces, case-insensitive resolution, `update --set title=...` rejection

**Integration tests (adapted):**
- Existing `cli_integration_test.rs` tests are slug-based. Rewrite assertions to expect title-based filenames and wrapped link values.
- `doctor` tests: add cases for each new check category.
- `storage_test.rs`, `schema_test.rs`, `frontmatter_test.rs`: adapt any slug-dependent assertions.

**Benchmarks:**
- Add `rename_bench` to `benches/query_bench.rs` at 100/500/5000 files to validate O(vault) rename cost.

**Out of scope:**
- Doctests for rename cascade (setup too heavy; use integration tests with `TestVault`)

## Documentation / Skill Sweep

The following require updates to match the new model. These are discovered during implementation, not part of the core feature, but listed here so the plan captures them:

- `CLAUDE.md` — "Non-Obvious Patterns" section: ID format, default field values, sanitization rules
- `README.md` — any example commands using slug IDs
- `skills/second-brain-protocol/SKILL.md` — ID format section (line 210–212), all query/CRUD examples, Links description (line 214), Relation Rules examples (lines 166–178), Recipes section CRUD flow (lines 324–396). Every example currently shows slug IDs like `q2-planning`, `launch-v2-0`, `review-pr` — all must become title form (`"Q2 Planning"`, `"Launch v2.0"`, `"Review PR"`) with wikilinks where stored.
- `skills/using-cortx-cli/SKILL.md` — same sweep as second-brain-protocol
- `skills/jarvis/*.md` — playbooks reference entity IDs; one-line update that entities are referenced by title
- Skill Maintenance section in `CLAUDE.md` already lists this requirement

## Risks & Open Questions

**Risk: case-insensitive filesystem surprises.** On macOS default (HFS+/APFS case-insensitive), two files with titles differing only in case cannot both exist. The uniqueness check (case-insensitive on create + rename) prevents this, but a vault created on Linux and then opened on macOS could surface existing collisions. Doctor's filename collision check handles this.

**Risk: rollback on multi-file rename failures.** The transactional rename relies on copying modified files to a temp dir before edits. If cortx crashes mid-rollback (extremely unlikely), the user is left with a partially-updated vault. A stale-lock / in-progress marker file in `.cortx/rename-in-progress` could help, but is deferred until we see a real failure.

**Risk: unicode normalization.** Two visually identical titles may have different UTF-8 byte sequences (NFC vs NFD). macOS HFS+ stores NFD; most editors produce NFC. cortx should NFC-normalize titles before sanitization so that `Café` and `Café` (different composition) compare equal. Doctor filename check should flag if filesystem uses NFD despite cortx writing NFC.

**Open question (deferred):** should `cortx rename` offer `--rename-body` as the opt-in (and skip by default), rather than `--skip-body` as the opt-out? The current default (rewrite bodies) matches Obsidian's behavior but carries more risk. Can be revisited after first real use.

## Decision Log

- **Option B (filesystem-safe title as ID) over C (separate slug + title filename)**: avoids having two parallel reference systems (Obsidian wikilinks vs cortx slug refs).
- **Wrapped wikilinks in frontmatter over bare strings**: the user's explicit requirement — only `[[...]]` renders as clickable in Obsidian.
- **Reject at write time, not defer to doctor**: consistent with collision policy ("throw an error, user retries"); instant feedback for AI agents.
- **Global uniqueness over per-folder**: keeps Obsidian wikilink resolution unambiguous without path-qualified links.
- **Error on title collision, no auto-suffix**: user-driven clarity over machine-generated disambiguation.
- **Separate `rename` command over allowing `update --set title=...`**: renames are cross-file transactional operations; surfacing the cost in the verb communicates the weight to users and agents.
- **No migration tool**: pre-production; one-time reset is acceptable.
