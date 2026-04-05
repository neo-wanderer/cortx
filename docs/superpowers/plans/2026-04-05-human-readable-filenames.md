# Human-Readable Filenames & Wikilink Relations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Entities are identified by their human title (filesystem-safe). Link fields in frontmatter are stored as Obsidian-compatible `"[[Title]]"` strings. A new `cortx rename` command handles title changes via a transactional cascade across frontmatter back-refs and body wikilinks.

**Architecture:** Sanitization derives a filesystem-safe `id` from `title` (replaces illegal chars with space, collapses whitespace, NFC-normalizes). Link-typed frontmatter fields are read/written through a schema-driven wrap/unwrap layer so the query engine operates on bare titles. CLI accepts titles directly. Rename is a dedicated command that scans the vault and rewrites all references (frontmatter + body) transactionally with copy-first rollback.

**Tech Stack:** Rust 2024, clap 4, serde_yaml 0.9, walkdir 2, rayon 1, deunicode 1, tempfile (dev-dep)

---

## File Structure

**New files:**
- `src/wikilink.rs` — wrap/unwrap helpers for `[[Title]]` strings
- `src/cli/rename.rs` — rename command with cascade + rollback
- `tests/wikilink_test.rs` — unit + integration tests for wrap/unwrap
- `tests/rename_test.rs` — integration tests for rename cascade
- `tests/doctor_filenames_test.rs` — doctor filenames subcommand tests

**Modified files:**
- `src/slug.rs` — add `sanitize_title()`, keep `to_slug()` temporarily for backward tests
- `src/entity.rs` — no change to struct; add helper methods if needed
- `src/storage/markdown.rs` — title-based lookup, collision check, wrap/unwrap integration, link validation
- `src/cli/create.rs` — use `sanitize_title`, collision check, link validation
- `src/cli/update.rs` — reject `title` updates, link validation, `--no-validate-links`
- `src/cli/show.rs`, `archive.rs`, `delete.rs`, `note.rs` — title positional arg semantics
- `src/cli/mod.rs` — register `rename` command, add `filenames` to doctor
- `src/cli/doctor.rs` — new `filenames` subcommand, adapt `links` check to unwrapped values
- `src/lib.rs` — re-export `wikilink` module
- `benches/query_bench.rs` — add `rename_bench`
- `tests/cli_integration_test.rs` — rewrite slug-based assertions
- Existing integration test files — adapt where slug IDs are assumed
- `CLAUDE.md`, `README.md`, skill files — documentation sweep

---

## Task 1: Add `sanitize_title` with NFC normalization

**Goal:** Produce a pure function that turns a raw title into a filesystem-safe `id`: `sanitize("Meeting: Q2/Q3 Review")` → `"Meeting Q2 Q3 Review"`.

**Files:**
- Modify: `src/slug.rs` (add function, keep `to_slug` for now)
- Modify: `Cargo.toml` (add `unicode-normalization` dep)

- [ ] **Step 1: Add unicode-normalization dependency**

Add to `Cargo.toml` under `[dependencies]`:

```toml
unicode-normalization = "0.1"
```

Run: `cargo build`
Expected: compiles with new dep resolved.

- [ ] **Step 2: Write failing tests in `src/slug.rs`**

Append to the existing `#[cfg(test)] mod tests` block in `src/slug.rs`:

```rust
    // --- sanitize_title tests ---

    #[test]
    fn sanitize_preserves_simple_title() {
        assert_eq!(sanitize_title("Buy Groceries"), "Buy Groceries");
    }

    #[test]
    fn sanitize_replaces_illegal_chars_with_space() {
        assert_eq!(sanitize_title("Meeting: Q2/Q3 Review"), "Meeting Q2 Q3 Review");
    }

    #[test]
    fn sanitize_collapses_whitespace() {
        assert_eq!(sanitize_title("Q2    Planning"), "Q2 Planning");
        assert_eq!(sanitize_title("A\tB"), "A B");
    }

    #[test]
    fn sanitize_trims_edges() {
        assert_eq!(sanitize_title("  hello world  "), "hello world");
    }

    #[test]
    fn sanitize_strips_all_illegal() {
        assert_eq!(sanitize_title(r#"\/:*?"<>|"#), "");
    }

    #[test]
    fn sanitize_trailing_dot_removed() {
        assert_eq!(sanitize_title("Note..."), "Note");
        assert_eq!(sanitize_title("Foo."), "Foo");
    }

    #[test]
    fn sanitize_preserves_unicode() {
        assert_eq!(sanitize_title("Café Réunion"), "Café Réunion");
    }

    #[test]
    fn sanitize_preserves_caps_and_punct() {
        assert_eq!(sanitize_title("Don't Forget (Urgent)!"), "Don't Forget (Urgent)!");
    }

    #[test]
    fn sanitize_idempotent() {
        let s = "Meeting: Q2/Q3 Review";
        assert_eq!(sanitize_title(s), sanitize_title(&sanitize_title(s)));
    }

    #[test]
    fn sanitize_nfc_normalizes() {
        // "é" as NFD (e + combining acute) vs NFC (single code point)
        let nfd = "Cafe\u{0301}";
        let nfc = "Café";
        assert_eq!(sanitize_title(nfd), sanitize_title(nfc));
    }

    #[test]
    fn sanitize_control_chars_stripped() {
        assert_eq!(sanitize_title("foo\x00bar\x1fbaz"), "foo bar baz");
    }

    #[test]
    fn sanitize_empty_input() {
        assert_eq!(sanitize_title(""), "");
        assert_eq!(sanitize_title("   "), "");
    }
```

Run: `cargo test --lib slug::tests::sanitize`
Expected: FAIL — `sanitize_title` not defined.

- [ ] **Step 3: Implement `sanitize_title`**

At the top of `src/slug.rs`, add the import:

```rust
use unicode_normalization::UnicodeNormalization;
```

Then append this function (keep the existing `to_slug` and `deunicode` import):

```rust
/// Derive a filesystem-safe `id` from a human title.
///
/// Replaces each of `/ \ : * ? " < > |` and ASCII control chars with a
/// space, strips trailing dots (Windows), collapses runs of whitespace
/// to single spaces, trims edges, and NFC-normalizes the result.
///
/// Preserves uppercase, unicode letters, and filesystem-safe punctuation.
/// This function is idempotent: `sanitize(sanitize(x)) == sanitize(x)`.
///
/// # Examples
///
/// ```
/// use cortx::slug::sanitize_title;
/// assert_eq!(sanitize_title("Buy Groceries"), "Buy Groceries");
/// assert_eq!(sanitize_title("Meeting: Q2/Q3 Review"), "Meeting Q2 Q3 Review");
/// assert_eq!(sanitize_title("  multiple   spaces  "), "multiple spaces");
/// ```
pub fn sanitize_title(title: &str) -> String {
    // NFC-normalize first so composed/decomposed unicode compares equal
    let normalized: String = title.nfc().collect();

    // Replace illegal filesystem chars and control chars with a space
    let mut replaced = String::with_capacity(normalized.len());
    for c in normalized.chars() {
        let illegal = matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            || c.is_control();
        if illegal {
            replaced.push(' ');
        } else {
            replaced.push(c);
        }
    }

    // Collapse whitespace runs, trim edges
    let mut result = String::with_capacity(replaced.len());
    let mut prev_space = true; // suppresses leading whitespace
    for c in replaced.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }
    // Trim trailing whitespace
    while result.ends_with(' ') {
        result.pop();
    }

    // Strip trailing dots (Windows constraint). Do this AFTER whitespace
    // collapsing so "Foo . " → "Foo" via sequential dot/space removal.
    loop {
        if result.ends_with('.') {
            result.pop();
            while result.ends_with(' ') {
                result.pop();
            }
        } else {
            break;
        }
    }

    result
}
```

- [ ] **Step 4: Run tests and verify pass**

Run: `cargo test --lib slug::tests`
Expected: all `sanitize_*` tests PASS; existing `to_slug` tests still PASS.

Run: `cargo test --doc slug`
Expected: doctest in `sanitize_title` passes.

- [ ] **Step 5: Commit**

```bash
git add src/slug.rs Cargo.toml Cargo.lock
git commit -m "feat: add sanitize_title for filesystem-safe entity ids"
```

---

## Task 2: Wikilink wrap/unwrap helpers

**Goal:** Pure helpers that convert `Value::String("Buy Groceries")` ↔ `serde_yaml::Value::String("[[Buy Groceries]]")`, detect malformed/piped forms, and operate on arrays.

**Files:**
- Create: `src/wikilink.rs`
- Modify: `src/lib.rs`
- Create: `tests/wikilink_test.rs`

- [ ] **Step 1: Write failing unit tests in the new module**

Create `src/wikilink.rs` with just the test module:

```rust
//! Wikilink wrapping for link-typed frontmatter fields.
//!
//! Link-typed fields (`FieldType::Link`, `FieldType::ArrayLink`) are stored
//! in YAML frontmatter as wrapped wikilink strings (`"[[Title]]"`) so they
//! render as clickable links in Obsidian. This module is the single seam
//! where wrap and unwrap happen; all downstream code operates on bare titles.

use crate::error::{CortxError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_bare_title() {
        assert_eq!(wrap("Buy Groceries"), "[[Buy Groceries]]");
    }

    #[test]
    fn unwrap_wrapped_title() {
        assert_eq!(unwrap("[[Buy Groceries]]").unwrap(), "Buy Groceries");
    }

    #[test]
    fn unwrap_trims_whitespace_inside() {
        assert_eq!(unwrap("[[  Buy Groceries  ]]").unwrap(), "Buy Groceries");
    }

    #[test]
    fn unwrap_rejects_missing_close() {
        assert!(unwrap("[[Buy Groceries").is_err());
    }

    #[test]
    fn unwrap_rejects_missing_open() {
        assert!(unwrap("Buy Groceries]]").is_err());
    }

    #[test]
    fn unwrap_rejects_empty() {
        assert!(unwrap("[[]]").is_err());
        assert!(unwrap("[[   ]]").is_err());
    }

    #[test]
    fn unwrap_rejects_piped_form() {
        assert!(unwrap("[[slug|Display]]").is_err());
    }

    #[test]
    fn unwrap_rejects_bare_string() {
        assert!(unwrap("bare-string").is_err());
    }

    #[test]
    fn is_wrapped_predicate() {
        assert!(is_wrapped("[[foo]]"));
        assert!(!is_wrapped("foo"));
        assert!(!is_wrapped("[[foo"));
        assert!(!is_wrapped("foo]]"));
    }

    #[test]
    fn round_trip() {
        let title = "Meeting Q2 Review";
        assert_eq!(unwrap(&wrap(title)).unwrap(), title);
    }
}
```

Run: `cargo test --lib wikilink`
Expected: FAIL — functions not defined.

- [ ] **Step 2: Implement the helpers**

Add this above the `#[cfg(test)]` block in `src/wikilink.rs`:

```rust
/// Wrap a bare title in wikilink syntax.
///
/// # Examples
/// ```
/// use cortx::wikilink::wrap;
/// assert_eq!(wrap("Buy Groceries"), "[[Buy Groceries]]");
/// ```
pub fn wrap(title: &str) -> String {
    format!("[[{title}]]")
}

/// Unwrap a wikilink string to a bare title.
///
/// Returns an error for:
/// - missing open/close brackets
/// - empty or whitespace-only content
/// - piped forms (`[[slug|Display]]`)
///
/// Whitespace inside brackets is trimmed.
///
/// # Examples
/// ```
/// use cortx::wikilink::unwrap;
/// assert_eq!(unwrap("[[Buy Groceries]]").unwrap(), "Buy Groceries");
/// assert!(unwrap("[[slug|Display]]").is_err());
/// ```
pub fn unwrap(wrapped: &str) -> Result<String> {
    let s = wrapped.trim();
    if !s.starts_with("[[") || !s.ends_with("]]") {
        return Err(CortxError::Validation(format!(
            "not a wikilink: {wrapped:?} (expected [[Title]])"
        )));
    }
    let inner = &s[2..s.len() - 2];
    if inner.contains('|') {
        return Err(CortxError::Validation(format!(
            "piped wikilinks are not supported: {wrapped:?}"
        )));
    }
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return Err(CortxError::Validation(format!(
            "empty wikilink: {wrapped:?}"
        )));
    }
    Ok(trimmed.to_string())
}

/// Check if a string is a well-formed `[[...]]` wikilink.
pub fn is_wrapped(s: &str) -> bool {
    let t = s.trim();
    t.starts_with("[[") && t.ends_with("]]") && t.len() > 4
}
```

- [ ] **Step 3: Register module in lib.rs**

Add to `src/lib.rs`:

```rust
pub mod wikilink;
```

(Insert alphabetically with existing `pub mod` lines.)

- [ ] **Step 4: Run tests**

Run: `cargo test --lib wikilink`
Expected: all tests PASS.

Run: `cargo test --doc wikilink`
Expected: doctests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/wikilink.rs src/lib.rs
git commit -m "feat: wikilink wrap/unwrap helpers with validation"
```

---

## Task 3: Value-level wikilink helpers for Link/ArrayLink fields

**Goal:** Schema-driven helpers that walk a frontmatter `HashMap` and wrap/unwrap link-typed fields. These are the seam between raw YAML and the rest of cortx.

**Files:**
- Modify: `src/wikilink.rs`
- Modify: `tests/wikilink_test.rs` (create if not exists — integration for schema-aware behavior)

- [ ] **Step 1: Write failing tests for schema-aware wrap/unwrap**

Append to `src/wikilink.rs` tests module:

```rust
    use crate::schema::types::{FieldDefinition, FieldType, LinkDef, LinkTargets, TypeDefinition};
    use crate::value::Value;
    use std::collections::HashMap;

    fn mk_type_def_with_link_fields() -> TypeDefinition {
        let mut fields = HashMap::new();

        let single_link = FieldDefinition {
            field_type: FieldType::Link(LinkDef {
                targets: LinkTargets::Single { ref_type: "project".into(), inverse: None },
                bidirectional: false,
                inverse_one: false,
            }),
            required: false,
            default: None,
        };
        let array_link = FieldDefinition {
            field_type: FieldType::ArrayLink(LinkDef {
                targets: LinkTargets::Single { ref_type: "note".into(), inverse: None },
                bidirectional: false,
                inverse_one: false,
            }),
            required: false,
            default: None,
        };
        let string_field = FieldDefinition {
            field_type: FieldType::String,
            required: false,
            default: None,
        };

        fields.insert("project".into(), single_link);
        fields.insert("related".into(), array_link);
        fields.insert("title".into(), string_field);

        TypeDefinition {
            name: "task".into(),
            folder: "tasks".into(),
            required: vec![],
            fields,
        }
    }

    #[test]
    fn wrap_frontmatter_wraps_link_fields() {
        let mut fm = HashMap::new();
        fm.insert("project".into(), Value::String("Website Redesign".into()));
        fm.insert("related".into(), Value::Array(vec![
            Value::String("Weekly Review".into()),
            Value::String("Meal Planning".into()),
        ]));
        fm.insert("title".into(), Value::String("Buy Groceries".into()));

        let td = mk_type_def_with_link_fields();
        wrap_frontmatter(&mut fm, &td);

        assert_eq!(fm["project"], Value::String("[[Website Redesign]]".into()));
        assert_eq!(fm["related"], Value::Array(vec![
            Value::String("[[Weekly Review]]".into()),
            Value::String("[[Meal Planning]]".into()),
        ]));
        // Non-link string field untouched
        assert_eq!(fm["title"], Value::String("Buy Groceries".into()));
    }

    #[test]
    fn unwrap_frontmatter_unwraps_link_fields() {
        let mut fm = HashMap::new();
        fm.insert("project".into(), Value::String("[[Website Redesign]]".into()));
        fm.insert("related".into(), Value::Array(vec![
            Value::String("[[Weekly Review]]".into()),
        ]));
        fm.insert("title".into(), Value::String("[[Buy Groceries]]".into())); // literal in string field

        let td = mk_type_def_with_link_fields();
        unwrap_frontmatter(&mut fm, &td).unwrap();

        assert_eq!(fm["project"], Value::String("Website Redesign".into()));
        assert_eq!(fm["related"], Value::Array(vec![
            Value::String("Weekly Review".into()),
        ]));
        // Non-link string field untouched — literal `[[...]]` preserved
        assert_eq!(fm["title"], Value::String("[[Buy Groceries]]".into()));
    }

    #[test]
    fn unwrap_frontmatter_errors_on_malformed_link_field() {
        let mut fm = HashMap::new();
        fm.insert("project".into(), Value::String("bare-string-not-wrapped".into()));
        let td = mk_type_def_with_link_fields();
        assert!(unwrap_frontmatter(&mut fm, &td).is_err());
    }

    #[test]
    fn unwrap_frontmatter_tolerates_empty_arrays_and_null() {
        let mut fm = HashMap::new();
        fm.insert("related".into(), Value::Array(vec![]));
        fm.insert("project".into(), Value::Null);
        let td = mk_type_def_with_link_fields();
        unwrap_frontmatter(&mut fm, &td).unwrap();
        assert_eq!(fm["related"], Value::Array(vec![]));
        assert_eq!(fm["project"], Value::Null);
    }
```

Run: `cargo test --lib wikilink`
Expected: FAIL — `wrap_frontmatter`, `unwrap_frontmatter` not defined.

- [ ] **Step 2: Implement schema-aware helpers**

Append to `src/wikilink.rs` (above tests):

```rust
use crate::schema::types::{FieldType, TypeDefinition};
use crate::value::Value;
use std::collections::HashMap;

/// Wrap every link-typed field value in `[[...]]` form.
///
/// Operates in place. Consults the type definition to decide which fields
/// are `Link`/`ArrayLink`. Non-link fields are untouched.
///
/// Assumes link field values are bare titles (strings or arrays of strings).
/// Already-wrapped values are wrapped again — callers should only pass bare
/// titles here.
pub fn wrap_frontmatter(fm: &mut HashMap<String, Value>, type_def: &TypeDefinition) {
    for (field_name, field_def) in &type_def.fields {
        let is_link = matches!(
            field_def.field_type,
            FieldType::Link(_) | FieldType::ArrayLink(_)
        );
        if !is_link {
            continue;
        }
        let Some(value) = fm.get_mut(field_name) else {
            continue;
        };
        match value {
            Value::String(s) if !s.is_empty() => {
                *s = wrap(s);
            }
            Value::Array(items) => {
                for item in items.iter_mut() {
                    if let Value::String(s) = item {
                        if !s.is_empty() {
                            *s = wrap(s);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Unwrap every link-typed field value from `[[...]]` to bare titles.
///
/// Operates in place. Returns an error if any link-typed field contains a
/// malformed value (not `[[Title]]` form, piped, or empty).
///
/// Empty strings, nulls, and empty arrays are tolerated.
pub fn unwrap_frontmatter(
    fm: &mut HashMap<String, Value>,
    type_def: &TypeDefinition,
) -> Result<()> {
    for (field_name, field_def) in &type_def.fields {
        let is_link = matches!(
            field_def.field_type,
            FieldType::Link(_) | FieldType::ArrayLink(_)
        );
        if !is_link {
            continue;
        }
        let Some(value) = fm.get_mut(field_name) else {
            continue;
        };
        match value {
            Value::String(s) if !s.is_empty() => {
                let bare = unwrap(s).map_err(|e| {
                    CortxError::Validation(format!("field '{field_name}': {e}"))
                })?;
                *s = bare;
            }
            Value::Array(items) => {
                for (idx, item) in items.iter_mut().enumerate() {
                    if let Value::String(s) = item {
                        if s.is_empty() {
                            continue;
                        }
                        let bare = unwrap(s).map_err(|e| {
                            CortxError::Validation(format!(
                                "field '{field_name}[{idx}]': {e}"
                            ))
                        })?;
                        *s = bare;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Run tests and verify pass**

Run: `cargo test --lib wikilink`
Expected: all tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/wikilink.rs
git commit -m "feat: schema-driven wrap/unwrap for link-typed frontmatter fields"
```

---

## Task 4: Read-path unwrapping in `MarkdownRepository::read_entity`

**Goal:** When cortx reads any entity file, link-typed fields are unwrapped from `[[Title]]` to bare titles before the entity is returned. Downstream code sees bare titles.

**Files:**
- Modify: `src/storage/markdown.rs`

- [ ] **Step 1: Write failing integration test**

Add this test to `tests/storage_test.rs` (or a new test file if more convenient):

```rust
#[test]
fn read_entity_unwraps_link_fields() {
    use cortx::schema::registry::TypeRegistry;
    use cortx::storage::Repository;
    use cortx::storage::markdown::MarkdownRepository;
    use cortx::value::Value;

    let vault = TestVault::new();
    // Minimal schema: task with a project link field and related array-link field
    let schema_yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project }
      related: { type: array[link], ref: note }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
  note:
    folder: "notes"
    required: [type, title]
    fields:
      type: { const: note }
      title: { type: string }
"#;
    let registry = TypeRegistry::from_yaml_str(schema_yaml).unwrap();

    vault.write_file(
        "tasks/Buy Groceries.md",
        "---\n\
         type: task\n\
         title: Buy Groceries\n\
         project: \"[[Website Redesign]]\"\n\
         related:\n  - \"[[Weekly Review]]\"\n  - \"[[Meal Planning]]\"\n\
         ---\n",
    );

    let repo = MarkdownRepository::new(vault.path().to_path_buf());
    let entity = repo.get_by_id("Buy Groceries", &registry).unwrap();

    assert_eq!(
        entity.frontmatter.get("project"),
        Some(&Value::String("Website Redesign".into()))
    );
    assert_eq!(
        entity.frontmatter.get("related"),
        Some(&Value::Array(vec![
            Value::String("Weekly Review".into()),
            Value::String("Meal Planning".into()),
        ]))
    );
}
```

Run: `cargo test --test storage_test read_entity_unwraps_link_fields`
Expected: FAIL — link fields come back wrapped.

- [ ] **Step 2: Thread registry into `read_entity` / `scan_folder`**

In `src/storage/markdown.rs`, update the signature and implementation of `read_entity`:

```rust
fn read_entity(&self, path: &Path, registry: &TypeRegistry) -> Result<Entity> {
    let content = std::fs::read_to_string(path)?;
    let (mut fm, body) = parse_frontmatter(&content)?;
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Unwrap link-typed fields using the entity's type definition
    if let Some(type_name) = fm.get("type").and_then(|v| v.as_str()) {
        if let Some(type_def) = registry.get(type_name) {
            crate::wikilink::unwrap_frontmatter(&mut fm, type_def)?;
        }
    }

    Ok(Entity::new(id, fm, body).with_path(path.to_path_buf()))
}
```

Update every call site in `markdown.rs`:

- In `scan_folder`, change the signature to `fn scan_folder(&self, folder: &Path, registry: &TypeRegistry) -> Result<Vec<Entity>>` and pass `registry` into the parallel closure.
- In `get_by_id`: `self.read_entity(&path, registry)` (registry is already in scope).
- In `update`: `let mut entity = self.read_entity(&path, registry)?;`
- In `list_by_type`: `self.scan_folder(&folder, registry)`.
- In `apply_bidirectional`: the function reads a file directly with `parse_frontmatter` and doesn't use `read_entity`, so update it to also unwrap. Specifically, after `let (mut ref_fm, ref_body) = parse_frontmatter(&ref_content)?;`, insert:

```rust
if let Some(ref_type_name) = ref_fm.get("type").and_then(|v| v.as_str()) {
    if let Some(ref_type_def) = registry.get(ref_type_name) {
        crate::wikilink::unwrap_frontmatter(&mut ref_fm, ref_type_def)?;
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo build`
Expected: compiles.

Run: `cargo test --test storage_test read_entity_unwraps_link_fields`
Expected: PASS.

- [ ] **Step 4: Check for regressions**

Run: `cargo test`
Expected: some existing tests may fail because they assert on slug-based IDs or raw frontmatter. Note the failures; they will be fixed in later tasks. If tests fail only due to wrapping/unwrapping of data they didn't provide wrapped, investigate. At this point, existing tests should pass because they don't use wrapped link values in fixtures yet.

- [ ] **Step 5: Commit**

```bash
git add src/storage/markdown.rs tests/storage_test.rs
git commit -m "feat: unwrap wikilink-wrapped link fields on read"
```

---

## Task 5: Write-path wrapping before serialization

**Goal:** When cortx writes an entity, link-typed fields are wrapped to `[[Title]]` form in the file. Callers continue to pass bare titles.

**Files:**
- Modify: `src/storage/markdown.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/storage_test.rs`:

```rust
#[test]
fn create_wraps_link_fields_in_file() {
    use cortx::schema::registry::TypeRegistry;
    use cortx::storage::Repository;
    use cortx::storage::markdown::MarkdownRepository;
    use cortx::value::Value;
    use std::collections::HashMap;

    let vault = TestVault::new();
    let schema_yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project }
      related: { type: array[link], ref: note }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
  note:
    folder: "notes"
    required: [type, title]
    fields:
      type: { const: note }
      title: { type: string }
"#;
    let registry = TypeRegistry::from_yaml_str(schema_yaml).unwrap();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    // Create target entities first so link validation (later task) would pass.
    // For now, link fields are wrapped regardless of existence.
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Buy Groceries".into()));
    fm.insert("project".into(), Value::String("Website Redesign".into()));
    fm.insert("related".into(), Value::Array(vec![
        Value::String("Weekly Review".into()),
    ]));
    repo.create("Buy Groceries", fm, "", &registry).unwrap();

    // Read the raw file and verify wikilinks are present
    let raw = vault.read_file("tasks/Buy Groceries.md");
    assert!(raw.contains("project: \"[[Website Redesign]]\""), "got: {raw}");
    assert!(raw.contains("\"[[Weekly Review]]\""), "got: {raw}");
}
```

Run: `cargo test --test storage_test create_wraps_link_fields_in_file`
Expected: FAIL — file contains bare strings, not wrapped.

- [ ] **Step 2: Add wrapping in `create` and `update`**

In `src/storage/markdown.rs`, modify `create`:

```rust
fn create(
    &self,
    id: &str,
    frontmatter: HashMap<String, Value>,
    body: &str,
    registry: &TypeRegistry,
) -> Result<Entity> {
    let type_name = frontmatter
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CortxError::Validation("missing 'type' field".into()))?
        .to_string();

    let type_def = registry
        .get(&type_name)
        .ok_or_else(|| CortxError::Schema(format!("unknown type '{type_name}'")))?;

    validate_frontmatter(&frontmatter, type_def)?;

    let path = self.resolve_path(&type_name, id, registry)?;

    if path.exists() {
        return Err(CortxError::Storage(format!(
            "entity '{id}' already exists at {}",
            path.display()
        )));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Wrap link-typed fields before serialization
    let mut fm_for_write = frontmatter.clone();
    crate::wikilink::wrap_frontmatter(&mut fm_for_write, type_def);

    let content = serialize_entity(&fm_for_write, body);
    let _lock = file_lock::FileLock::acquire(&path)?;
    std::fs::write(&path, content)?;

    // Maintain bidirectional inverse fields (use bare-title values)
    for (field_name, value) in &frontmatter {
        self.apply_bidirectional(id, field_name, value, registry, type_def)?;
    }

    Ok(Entity::new(id.to_string(), frontmatter, body.to_string()).with_path(path))
}
```

And modify `update` around the serialize call:

```rust
    // Wrap link-typed fields before serialization
    let mut fm_for_write = entity.frontmatter.clone();
    if let Some(type_def) = registry.get(&entity.entity_type) {
        crate::wikilink::wrap_frontmatter(&mut fm_for_write, type_def);
    }
    let content = serialize_entity(&fm_for_write, &entity.body);
    std::fs::write(&path, content)?;
```

(Replace the existing `let content = serialize_entity(&entity.frontmatter, &entity.body);` / `std::fs::write` pair.)

Also modify `apply_bidirectional`: after it builds the new inverse array, before writing the file, wrap:

```rust
    // Wrap link-typed fields before serializing the ref file
    crate::wikilink::wrap_frontmatter(&mut ref_fm, &ref_type_def_for_write);
    let updated = serialize_entity(&ref_fm, &ref_body);
    std::fs::write(&ref_path, updated)?;
```

You'll need to obtain `ref_type_def_for_write` by looking it up via `registry.get(&ref_type)` before the write.

- [ ] **Step 3: Run the new test**

Run: `cargo test --test storage_test create_wraps_link_fields_in_file`
Expected: PASS.

- [ ] **Step 4: Full test suite**

Run: `cargo test`
Expected: new test passes; existing tests may fail because they write raw fixture files with bare IDs and then expect to read them. Triaging: any test that writes `project: some-slug` and then queries on `project = "some-slug"` will fail under the new read path (tries to unwrap `some-slug`, fails). These will be addressed in the test-adaptation task (Task 18). For now, record which tests fail and proceed.

- [ ] **Step 5: Commit**

```bash
git add src/storage/markdown.rs tests/storage_test.rs
git commit -m "feat: wrap link-typed fields as wikilinks on write"
```

---

## Task 6: Case-insensitive collision check and title-based lookup

**Goal:** `create` rejects on case-insensitive title collision across the whole vault. Add `find_by_title` method for CLI commands.

**Files:**
- Modify: `src/storage/markdown.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/storage_test.rs`:

```rust
#[test]
fn create_rejects_case_insensitive_collision() {
    use cortx::schema::registry::TypeRegistry;
    use cortx::storage::Repository;
    use cortx::storage::markdown::MarkdownRepository;
    use cortx::value::Value;
    use std::collections::HashMap;

    let vault = TestVault::new();
    let schema_yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
  note:
    folder: "notes"
    required: [type, title]
    fields:
      type: { const: note }
      title: { type: string }
"#;
    let registry = TypeRegistry::from_yaml_str(schema_yaml).unwrap();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    let mut fm1 = HashMap::new();
    fm1.insert("type".into(), Value::String("task".into()));
    fm1.insert("title".into(), Value::String("Buy Groceries".into()));
    repo.create("Buy Groceries", fm1, "", &registry).unwrap();

    // Same id — must collide
    let mut fm2 = HashMap::new();
    fm2.insert("type".into(), Value::String("note".into()));
    fm2.insert("title".into(), Value::String("Buy Groceries".into()));
    let err = repo.create("Buy Groceries", fm2, "", &registry);
    assert!(err.is_err(), "expected collision error");

    // Case-only difference — must still collide (even across types)
    let mut fm3 = HashMap::new();
    fm3.insert("type".into(), Value::String("note".into()));
    fm3.insert("title".into(), Value::String("buy groceries".into()));
    let err = repo.create("buy groceries", fm3, "", &registry);
    assert!(err.is_err(), "expected case-insensitive collision error");
}

#[test]
fn find_by_title_returns_entity_regardless_of_folder() {
    use cortx::schema::registry::TypeRegistry;
    use cortx::storage::Repository;
    use cortx::storage::markdown::MarkdownRepository;
    use cortx::value::Value;
    use std::collections::HashMap;

    let vault = TestVault::new();
    let schema_yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
"#;
    let registry = TypeRegistry::from_yaml_str(schema_yaml).unwrap();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Buy Groceries".into()));
    repo.create("Buy Groceries", fm, "", &registry).unwrap();

    // Exact match
    let entity = repo.get_by_id("Buy Groceries", &registry).unwrap();
    assert_eq!(entity.id, "Buy Groceries");

    // Case-insensitive match also works
    let entity = repo.get_by_id("buy groceries", &registry).unwrap();
    assert_eq!(entity.id, "Buy Groceries");
}
```

Run: `cargo test --test storage_test create_rejects_case_insensitive_collision find_by_title_returns_entity_regardless_of_folder`
Expected: FAIL.

- [ ] **Step 2: Implement collision check in `create`**

In `src/storage/markdown.rs`, add a helper:

```rust
/// Scan every type folder for an existing file whose stem matches `id`
/// case-insensitively. Returns the first match.
fn find_case_insensitive_collision(
    &self,
    id: &str,
    registry: &TypeRegistry,
) -> Option<PathBuf> {
    let lower = id.to_lowercase();
    for type_name in registry.type_names() {
        let Some(type_def) = registry.get(type_name) else { continue };
        let folder = self.vault_path.join(&type_def.folder);
        if !folder.exists() { continue; }
        for entry in WalkDir::new(&folder).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.to_lowercase() == lower {
                    return Some(path.to_path_buf());
                }
            }
        }
    }
    None
}
```

In `create`, replace the existing `if path.exists() { ... }` block with:

```rust
if let Some(existing) = self.find_case_insensitive_collision(id, registry) {
    return Err(CortxError::Storage(format!(
        "entity id '{id}' collides with existing file at {} (case-insensitive match). \
         Choose a different title.",
        existing.display()
    )));
}
```

- [ ] **Step 3: Make `find_file_by_id` case-insensitive**

Update the existing `find_file_by_id` to perform a case-insensitive lookup:

```rust
fn find_file_by_id(&self, id: &str, registry: &TypeRegistry) -> Result<PathBuf> {
    // Try exact match first (fast path)
    for type_name in registry.type_names() {
        if let Some(type_def) = registry.get(type_name) {
            let path = self
                .vault_path
                .join(&type_def.folder)
                .join(format!("{id}.md"));
            if path.exists() {
                return Ok(path);
            }
        }
    }
    // Fall back to case-insensitive scan
    if let Some(path) = self.find_case_insensitive_collision(id, registry) {
        return Ok(path);
    }
    Err(CortxError::NotFound(format!("entity '{id}' not found")))
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test storage_test create_rejects_case_insensitive_collision find_by_title_returns_entity_regardless_of_folder`
Expected: PASS.

Run: `cargo test --lib`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/storage/markdown.rs tests/storage_test.rs
git commit -m "feat: case-insensitive collision check and title-based lookup"
```

---

## Task 7: Update `create` CLI to use `sanitize_title`

**Goal:** `cortx create task --title "Buy Groceries"` derives `id = "Buy Groceries"` (sanitized), writes `tasks/Buy Groceries.md`. `--id` override still works.

**Files:**
- Modify: `src/cli/create.rs`

- [ ] **Step 1: Write failing integration test**

Add to `tests/cli_integration_test.rs` (new test, leave existing slug-based tests for now):

```rust
#[test]
fn create_uses_sanitized_title_as_id() {
    let vault = TestVault::new();
    vault.write_file("types.yaml", r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      status: { type: enum, values: [open, done] }
"#);

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "task", "--title", "Buy Groceries"]);
    cmd.assert().success();

    assert!(vault.file_exists("tasks/Buy Groceries.md"),
            "expected tasks/Buy Groceries.md to exist");
}

#[test]
fn create_sanitizes_illegal_chars_in_title() {
    let vault = TestVault::new();
    vault.write_file("types.yaml", r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      status: { type: enum, values: [open, done] }
"#);

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "task", "--title", "Meeting: Q2/Q3 Review"]);
    cmd.assert().success();

    assert!(vault.file_exists("tasks/Meeting Q2 Q3 Review.md"));
}
```

Run: `cargo test --test cli_integration_test create_uses_sanitized_title_as_id create_sanitizes_illegal_chars_in_title`
Expected: FAIL — file is written with slug name.

- [ ] **Step 2: Replace `to_slug` with `sanitize_title` in create.rs**

In `src/cli/create.rs`, line 3, change:

```rust
use crate::slug::to_slug;
```

to:

```rust
use crate::slug::sanitize_title;
```

In the `run` function, replace:

```rust
    let id = if let Some(explicit) = &args.id {
        explicit.clone()
    } else {
        let base = args
            .title
            .as_deref()
            .or(args.name.as_deref())
            .unwrap_or(&args.entity_type);
        to_slug(base)
    };
```

with:

```rust
    let id = if let Some(explicit) = &args.id {
        sanitize_title(explicit)
    } else {
        let base = args
            .title
            .as_deref()
            .or(args.name.as_deref())
            .unwrap_or(&args.entity_type);
        sanitize_title(base)
    };

    if id.is_empty() {
        return Err(crate::error::CortxError::Validation(
            "title produces empty id after sanitization — provide a title with alphanumeric content".into()
        ));
    }
```

- [ ] **Step 3: Run tests**

Run: `cargo test --test cli_integration_test create_uses_sanitized_title_as_id create_sanitizes_illegal_chars_in_title`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/cli/create.rs tests/cli_integration_test.rs
git commit -m "feat: create uses sanitize_title for human-readable filenames"
```

---

## Task 8: Reject `update --set title=...`

**Goal:** Trying to change an entity's title via `update` returns a clear error directing the user to `cortx rename`.

**Files:**
- Modify: `src/cli/update.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/cli_integration_test.rs`:

```rust
#[test]
fn update_rejects_title_change() {
    let vault = TestVault::new();
    vault.write_file("types.yaml", r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
"#);

    // Create an entity first
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "task", "--title", "Buy Groceries"]);
    cmd.assert().success();

    // Try to update title — should fail
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "update", "Buy Groceries", "--set", "title=Weekly Groceries"]);
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("cortx rename"));
}
```

Run: `cargo test --test cli_integration_test update_rejects_title_change`
Expected: FAIL.

- [ ] **Step 2: Implement rejection**

In `src/cli/update.rs`, inside `run` after parsing updates but before calling `repo.update`:

```rust
pub fn run(args: &UpdateArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());

    let mut updates = HashMap::new();
    for kv in &args.updates {
        if let Some((k, v)) = kv.split_once('=') {
            if k == "title" {
                return Err(crate::error::CortxError::Validation(
                    "use 'cortx rename' to change an entity's title".into(),
                ));
            }
            let value = super::create::parse_cli_value(v);
            updates.insert(k.to_string(), value);
        }
    }

    let entity = repo.update(&args.id, updates, &config.registry)?;
    println!("Updated {}", entity.id);

    Ok(())
}
```

- [ ] **Step 3: Run test**

Run: `cargo test --test cli_integration_test update_rejects_title_change`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/cli/update.rs tests/cli_integration_test.rs
git commit -m "feat: update rejects title changes, directs to rename"
```

---

## Task 9: Link-target validation on write

**Goal:** When `create` or `update` sets a link-typed field (e.g., `project="Website Redesign"`), cortx verifies the target exists. `--no-validate-links` bypasses.

**Files:**
- Modify: `src/storage/markdown.rs`
- Modify: `src/cli/create.rs` (add flag)
- Modify: `src/cli/update.rs` (add flag)

- [ ] **Step 1: Write failing test**

Add to `tests/storage_test.rs`:

```rust
#[test]
fn create_rejects_dangling_link_ref() {
    use cortx::schema::registry::TypeRegistry;
    use cortx::storage::Repository;
    use cortx::storage::markdown::MarkdownRepository;
    use cortx::value::Value;
    use std::collections::HashMap;

    let vault = TestVault::new();
    let schema_yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
"#;
    let registry = TypeRegistry::from_yaml_str(schema_yaml).unwrap();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Buy Groceries".into()));
    fm.insert("project".into(), Value::String("Nonexistent Project".into()));

    // Default behavior: reject
    let err = repo.create("Buy Groceries", fm.clone(), "", &registry);
    assert!(err.is_err(), "expected link validation to reject dangling ref");
    let msg = format!("{}", err.unwrap_err());
    assert!(msg.contains("Nonexistent Project"), "error should mention the missing target: {msg}");
}

#[test]
fn create_accepts_existing_link_ref() {
    use cortx::schema::registry::TypeRegistry;
    use cortx::storage::Repository;
    use cortx::storage::markdown::MarkdownRepository;
    use cortx::value::Value;
    use std::collections::HashMap;

    let vault = TestVault::new();
    let schema_yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
"#;
    let registry = TypeRegistry::from_yaml_str(schema_yaml).unwrap();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    // Create the target project first
    let mut pfm = HashMap::new();
    pfm.insert("type".into(), Value::String("project".into()));
    pfm.insert("title".into(), Value::String("Website Redesign".into()));
    repo.create("Website Redesign", pfm, "", &registry).unwrap();

    // Now create a task that refers to it — should succeed
    let mut tfm = HashMap::new();
    tfm.insert("type".into(), Value::String("task".into()));
    tfm.insert("title".into(), Value::String("Buy Groceries".into()));
    tfm.insert("project".into(), Value::String("Website Redesign".into()));
    repo.create("Buy Groceries", tfm, "", &registry).unwrap();
}
```

Run: `cargo test --test storage_test create_rejects_dangling_link_ref create_accepts_existing_link_ref`
Expected: FAIL (first test should pass by accident only if validation already rejects; it won't currently).

- [ ] **Step 2: Add validation method to `MarkdownRepository`**

In `src/storage/markdown.rs`, add:

```rust
/// Verify every link-typed field in `frontmatter` resolves to an existing
/// entity. Polymorphic links succeed if any allowed target type contains
/// a matching title.
///
/// Returns an error on the first dangling reference.
pub fn validate_link_targets(
    &self,
    frontmatter: &HashMap<String, Value>,
    type_def: &crate::schema::types::TypeDefinition,
    registry: &TypeRegistry,
) -> Result<()> {
    use crate::schema::types::{FieldType, LinkTargets};

    for (field_name, field_def) in &type_def.fields {
        let link_def = match &field_def.field_type {
            FieldType::Link(d) | FieldType::ArrayLink(d) => d,
            _ => continue,
        };

        // Collect referenced titles (bare)
        let refs: Vec<String> = match frontmatter.get(field_name) {
            Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
                .collect(),
            _ => continue,
        };
        if refs.is_empty() { continue; }

        // Collect allowed target folders
        let target_types: Vec<String> = match &link_def.targets {
            LinkTargets::Single { ref_type, .. } => vec![ref_type.clone()],
            LinkTargets::Poly(targets) => targets.iter().map(|t| t.ref_type.clone()).collect(),
        };

        for title in &refs {
            let mut found = false;
            for tt in &target_types {
                if let Some(target_def) = registry.get(tt) {
                    let path = self
                        .vault_path
                        .join(&target_def.folder)
                        .join(format!("{title}.md"));
                    if path.exists() {
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                return Err(CortxError::Validation(format!(
                    "field '{field_name}': no entity found with title '{title}' in allowed target types {target_types:?}"
                )));
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Call validation in `create` and `update`**

Add a configuration flag to the repository or accept it as a parameter. Simplest: add a field.

Modify `MarkdownRepository`:

```rust
pub struct MarkdownRepository {
    vault_path: PathBuf,
    pub validate_links: bool,
}

impl MarkdownRepository {
    pub fn new(vault_path: PathBuf) -> Self {
        MarkdownRepository { vault_path, validate_links: true }
    }

    pub fn with_link_validation(mut self, enabled: bool) -> Self {
        self.validate_links = enabled;
        self
    }
    // ... (rest unchanged)
}
```

In `create`, after `validate_frontmatter(&frontmatter, type_def)?;`:

```rust
    if self.validate_links {
        self.validate_link_targets(&frontmatter, type_def, registry)?;
    }
```

In `update`, after `validate_frontmatter(&entity.frontmatter, type_def)?;` inside the `if let Some(type_def)` block:

```rust
    if self.validate_links {
        self.validate_link_targets(&entity.frontmatter, type_def, registry)?;
    }
```

- [ ] **Step 4: Add `--no-validate-links` flag to CLI**

In `src/cli/create.rs`, add to `CreateArgs`:

```rust
    /// Skip link-target existence validation (for bulk imports)
    #[arg(long)]
    pub no_validate_links: bool,
```

In `run`, change `let repo = MarkdownRepository::new(...)` to:

```rust
    let repo = MarkdownRepository::new(config.vault_path.clone())
        .with_link_validation(!args.no_validate_links);
```

Do the same in `src/cli/update.rs`:

```rust
#[derive(Args)]
pub struct UpdateArgs {
    pub id: String,

    #[arg(long = "set", num_args = 1, required = true)]
    pub updates: Vec<String>,

    /// Skip link-target existence validation (for bulk imports)
    #[arg(long)]
    pub no_validate_links: bool,
}
```

And:

```rust
    let repo = MarkdownRepository::new(config.vault_path.clone())
        .with_link_validation(!args.no_validate_links);
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test storage_test create_rejects_dangling_link_ref create_accepts_existing_link_ref`
Expected: PASS.

Run: `cargo test`
Expected: same baseline as before Task 9 (no new regressions). Some existing tests may fail if they created entities with link fields pointing to nonexistent targets; those need a target entity created first. Fix those inline if encountered.

- [ ] **Step 6: Commit**

```bash
git add src/storage/markdown.rs src/cli/create.rs src/cli/update.rs tests/storage_test.rs
git commit -m "feat: validate link-field targets on write, --no-validate-links escape hatch"
```

---

## Task 10: `rename` command — scaffolding and basic cascade

**Goal:** `cortx rename "Old Title" "New Title"` renames the file, updates its own `title` field, and rewrites frontmatter back-refs across the vault. Body wikilinks are deferred to Task 11.

**Files:**
- Create: `src/cli/rename.rs`
- Modify: `src/cli/mod.rs`
- Create: `tests/rename_test.rs`

- [ ] **Step 1: Register rename command in mod.rs**

In `src/cli/mod.rs`, add near the other `pub mod` declarations:

```rust
pub mod rename;
```

In the `Commands` enum, add:

```rust
    /// Rename an entity (cascades to all back-references)
    Rename(rename::RenameArgs),
```

In `src/main.rs` (check the dispatch), add a matching arm:

```rust
    Commands::Rename(args) => cli::rename::run(&args, &config),
```

Run: `cargo build`
Expected: fails with unresolved `cli::rename`. Good.

- [ ] **Step 2: Write failing integration test**

Create `tests/rename_test.rs`:

```rust
mod common;

use assert_cmd::Command;
use common::TestVault;

fn write_schema(vault: &TestVault) {
    vault.write_file("types.yaml", r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
"#);
}

#[test]
fn rename_updates_filename_and_back_refs() {
    let vault = TestVault::new();
    write_schema(&vault);

    // Create a project
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "project", "--title", "Website Redesign"]);
    cmd.assert().success();

    // Create a task linked to it
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "task", "--title", "Buy Groceries",
              "--set", "project=Website Redesign"]);
    cmd.assert().success();

    // Rename the project
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "rename", "Website Redesign", "Brand Refresh"]);
    cmd.assert().success();

    // Old file gone, new file present
    assert!(!vault.file_exists("projects/Website Redesign.md"));
    assert!(vault.file_exists("projects/Brand Refresh.md"));

    // Task's back-ref updated
    let task_content = vault.read_file("tasks/Buy Groceries.md");
    assert!(task_content.contains("\"[[Brand Refresh]]\""), "got: {task_content}");
    assert!(!task_content.contains("[[Website Redesign]]"));

    // Renamed file's own title field updated
    let project_content = vault.read_file("projects/Brand Refresh.md");
    assert!(project_content.contains("title: Brand Refresh"), "got: {project_content}");
}

#[test]
fn rename_rejects_collision() {
    let vault = TestVault::new();
    write_schema(&vault);

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "project", "--title", "Website Redesign"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "project", "--title", "Brand Refresh"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "rename", "Website Redesign", "Brand Refresh"]);
    cmd.assert().failure()
        .stderr(predicates::str::contains("collides"));
}
```

Run: `cargo test --test rename_test`
Expected: FAIL (cannot build — rename module missing).

- [ ] **Step 3: Implement rename.rs (without body wikilinks, without rollback yet)**

Create `src/cli/rename.rs`:

```rust
use crate::config::Config;
use crate::error::{CortxError, Result};
use crate::frontmatter::{parse_frontmatter, serialize_entity};
use crate::schema::types::{FieldType, TypeDefinition};
use crate::slug::sanitize_title;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::Args;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Args)]
pub struct RenameArgs {
    /// The current title of the entity
    pub old_title: String,

    /// The new title
    pub new_title: String,

    /// Show the plan without applying changes
    #[arg(long)]
    pub dry_run: bool,

    /// Skip rewriting body wikilinks (only update frontmatter back-refs)
    #[arg(long)]
    pub skip_body: bool,
}

pub fn run(args: &RenameArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone())
        .with_link_validation(false); // we're rewriting; not creating new refs

    let old_id = sanitize_title(&args.old_title);
    let new_id = sanitize_title(&args.new_title);

    if new_id.is_empty() {
        return Err(CortxError::Validation(
            "new title sanitizes to empty id — provide alphanumeric content".into(),
        ));
    }
    if old_id == new_id {
        println!("No change: old and new ids are identical after sanitization.");
        return Ok(());
    }

    // Resolve the entity being renamed
    let old_entity = repo.get_by_id(&old_id, &config.registry)?;
    let old_path = old_entity.file_path.clone().ok_or_else(|| {
        CortxError::Storage(format!("entity '{old_id}' has no file path"))
    })?;

    // Case-insensitive collision check for new_id (excluding the file being renamed)
    if collision_exists(&config.vault_path, &new_id, &old_path, &config.registry)? {
        return Err(CortxError::Storage(format!(
            "new id '{new_id}' collides with an existing file (case-insensitive). \
             Choose a different title."
        )));
    }

    // Compute the new path (same folder, new stem)
    let new_path = old_path.with_file_name(format!("{new_id}.md"));

    // Plan: (1) file rename, (2) update this file's title, (3) rewrite back-refs
    let back_ref_sites = find_back_refs(&config.vault_path, &old_id, &old_path, &config.registry)?;

    println!(
        "renamed: {} → {}",
        rel(&old_path, &config.vault_path),
        rel(&new_path, &config.vault_path)
    );
    if !back_ref_sites.is_empty() {
        println!("updated {} back-references:", back_ref_sites.len());
        for site in &back_ref_sites {
            println!("  {} ({})", rel(&site.path, &config.vault_path), site.field);
        }
    }

    if args.dry_run {
        println!("(dry-run, no files written)");
        return Ok(());
    }

    // Execute the plan (Task 11 adds body wikilinks + rollback)
    apply_rename(&old_path, &new_path, &args.new_title)?;
    for site in &back_ref_sites {
        rewrite_frontmatter_back_ref(&site.path, &site.field, &old_id, &new_id, &config.registry)?;
    }

    Ok(())
}

fn rel(path: &std::path::Path, vault: &std::path::Path) -> String {
    path.strip_prefix(vault)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn collision_exists(
    vault: &std::path::Path,
    new_id: &str,
    exclude: &std::path::Path,
    registry: &crate::schema::registry::TypeRegistry,
) -> Result<bool> {
    let lower = new_id.to_lowercase();
    for type_name in registry.type_names() {
        let Some(td) = registry.get(type_name) else { continue };
        let folder = vault.join(&td.folder);
        if !folder.exists() { continue; }
        for entry in WalkDir::new(&folder).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if path == exclude { continue; }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.to_lowercase() == lower {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

struct BackRefSite {
    path: PathBuf,
    field: String,
}

fn find_back_refs(
    vault: &std::path::Path,
    old_id: &str,
    exclude: &std::path::Path,
    registry: &crate::schema::registry::TypeRegistry,
) -> Result<Vec<BackRefSite>> {
    let mut sites = Vec::new();
    for type_name in registry.type_names() {
        let Some(td) = registry.get(type_name) else { continue };
        let folder = vault.join(&td.folder);
        if !folder.exists() { continue; }
        for entry in WalkDir::new(&folder).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if path == exclude { continue; }
            let content = std::fs::read_to_string(path)?;
            let (fm, _) = match parse_frontmatter(&content) {
                Ok(x) => x,
                Err(_) => continue, // skip unparseable files
            };
            let Some(entity_type) = fm.get("type").and_then(|v| v.as_str()) else { continue };
            let Some(entity_td) = registry.get(entity_type) else { continue };

            for (field_name, fd) in &entity_td.fields {
                let is_link = matches!(fd.field_type, FieldType::Link(_) | FieldType::ArrayLink(_));
                if !is_link { continue; }
                let Some(val) = fm.get(field_name) else { continue };
                if link_value_matches(val, old_id) {
                    sites.push(BackRefSite {
                        path: path.to_path_buf(),
                        field: field_name.clone(),
                    });
                    break; // one record per file is enough; rewriter will touch all fields
                }
            }
        }
    }
    Ok(sites)
}

fn link_value_matches(val: &Value, target_id: &str) -> bool {
    // Values here are WRAPPED (raw from file); caller uses parse_frontmatter directly
    match val {
        Value::String(s) => s == &format!("[[{target_id}]]"),
        Value::Array(items) => items.iter().any(|v| {
            matches!(v, Value::String(s) if s == &format!("[[{target_id}]]"))
        }),
        _ => false,
    }
}

fn apply_rename(old_path: &std::path::Path, new_path: &std::path::Path, new_title: &str) -> Result<()> {
    let content = std::fs::read_to_string(old_path)?;
    let (mut fm, body) = parse_frontmatter(&content)?;
    // Update title field
    fm.insert("title".into(), Value::String(new_title.to_string()));
    // Bump updated_at
    fm.insert("updated_at".into(), Value::Date(chrono::Local::now().date_naive()));
    let new_content = serialize_entity(&fm, &body);
    std::fs::write(new_path, new_content)?;
    std::fs::remove_file(old_path)?;
    Ok(())
}

fn rewrite_frontmatter_back_ref(
    path: &std::path::Path,
    _field: &str,
    old_id: &str,
    new_id: &str,
    registry: &crate::schema::registry::TypeRegistry,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let (mut fm, body) = parse_frontmatter(&content)?;
    let Some(entity_type) = fm.get("type").and_then(|v| v.as_str()).map(String::from) else {
        return Ok(());
    };
    let Some(entity_td) = registry.get(&entity_type).cloned() else {
        return Ok(());
    };

    let old_wrapped = format!("[[{old_id}]]");
    let new_wrapped = format!("[[{new_id}]]");

    for (field_name, fd) in &entity_td.fields {
        let is_link = matches!(fd.field_type, FieldType::Link(_) | FieldType::ArrayLink(_));
        if !is_link { continue; }
        let Some(val) = fm.get_mut(field_name) else { continue };
        match val {
            Value::String(s) if *s == old_wrapped => {
                *s = new_wrapped.clone();
            }
            Value::Array(items) => {
                for item in items.iter_mut() {
                    if let Value::String(s) = item {
                        if *s == old_wrapped {
                            *s = new_wrapped.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let new_content = serialize_entity(&fm, &body);
    std::fs::write(path, new_content)?;
    Ok(())
}
```

Note: `TypeDefinition` needs `Clone`. If not already derived, add `#[derive(Clone)]` to `src/schema/types.rs`. (Check first — it's likely already there since other code clones it.)

- [ ] **Step 4: Run tests**

Run: `cargo build`
Expected: compiles.

Run: `cargo test --test rename_test rename_updates_filename_and_back_refs rename_rejects_collision`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cli/rename.rs src/cli/mod.rs src/main.rs tests/rename_test.rs
git commit -m "feat: rename command with frontmatter back-ref cascade"
```

---

## Task 11: Rename — body wikilinks and transactional rollback

**Goal:** Add body-wikilink rewriting and copy-first rollback so a mid-transaction failure doesn't leave a partially-updated vault.

**Files:**
- Modify: `src/cli/rename.rs`

- [ ] **Step 1: Write failing tests**

Append to `tests/rename_test.rs`:

```rust
#[test]
fn rename_rewrites_body_wikilinks() {
    let vault = TestVault::new();
    write_schema(&vault);

    // Create a project
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "project", "--title", "Website Redesign"]);
    cmd.assert().success();

    // Manually append a body wikilink to an unrelated file
    vault.write_file("projects/Website Redesign.md",
        &(vault.read_file("projects/Website Redesign.md") + "\n\nSee [[Website Redesign]] for context.\n"));

    // Rename it
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "rename", "Website Redesign", "Brand Refresh"]);
    cmd.assert().success();

    let content = vault.read_file("projects/Brand Refresh.md");
    assert!(content.contains("See [[Brand Refresh]] for context."), "body not rewritten: {content}");
}

#[test]
fn rename_skip_body_preserves_body_wikilinks() {
    let vault = TestVault::new();
    write_schema(&vault);

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "project", "--title", "Website Redesign"]);
    cmd.assert().success();

    vault.write_file("projects/Website Redesign.md",
        &(vault.read_file("projects/Website Redesign.md") + "\n\nSee [[Website Redesign]] for context.\n"));

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "rename", "Website Redesign", "Brand Refresh", "--skip-body"]);
    cmd.assert().success();

    let content = vault.read_file("projects/Brand Refresh.md");
    assert!(content.contains("See [[Website Redesign]] for context."), "body should be preserved: {content}");
}

#[test]
fn rename_dry_run_writes_nothing() {
    let vault = TestVault::new();
    write_schema(&vault);

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "project", "--title", "Website Redesign"]);
    cmd.assert().success();

    let before = vault.read_file("projects/Website Redesign.md");

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "rename", "Website Redesign", "Brand Refresh", "--dry-run"]);
    cmd.assert().success().stdout(predicates::str::contains("dry-run"));

    assert!(vault.file_exists("projects/Website Redesign.md"));
    assert!(!vault.file_exists("projects/Brand Refresh.md"));
    assert_eq!(vault.read_file("projects/Website Redesign.md"), before);
}
```

Run: `cargo test --test rename_test rename_rewrites_body_wikilinks rename_skip_body_preserves_body_wikilinks rename_dry_run_writes_nothing`
Expected: body tests FAIL, dry-run may pass depending on Task 10 impl.

- [ ] **Step 2: Add body-wikilink rewrite and rollback**

Modify `src/cli/rename.rs`. Add a body-scan function and restructure `run` to copy files first.

Add near the top:

```rust
use std::collections::HashMap;
```

Add functions:

```rust
struct BodyRefSite {
    path: PathBuf,
}

fn find_body_wikilinks(
    vault: &std::path::Path,
    old_id: &str,
    exclude: &std::path::Path,
) -> Result<Vec<BodyRefSite>> {
    let mut sites = Vec::new();
    let token = format!("[[{old_id}]]");
    for entry in WalkDir::new(vault).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if path == exclude { continue; }
        let content = std::fs::read_to_string(path)?;
        // Split frontmatter from body
        let body = match content.find("\n---") {
            Some(close) if content.trim_start().starts_with("---") => {
                let body_start = close + 4;
                &content[body_start..]
            }
            _ => &content[..],
        };
        if body.contains(&token) {
            sites.push(BodyRefSite { path: path.to_path_buf() });
        }
    }
    Ok(sites)
}

fn rewrite_body_wikilinks(path: &std::path::Path, old_id: &str, new_id: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let (fm, body) = parse_frontmatter(&content)?;
    let old_tok = format!("[[{old_id}]]");
    let new_tok = format!("[[{new_id}]]");
    let new_body = body.replace(&old_tok, &new_tok);
    let new_content = serialize_entity(&fm, &new_body);
    std::fs::write(path, new_content)?;
    Ok(())
}
```

Restructure the body of `run` after the plan print, replacing the current apply block:

```rust
    if args.dry_run {
        println!("(dry-run, no files written)");
        return Ok(());
    }

    // Collect body-wikilink sites (skip if --skip-body)
    let body_sites = if args.skip_body {
        Vec::new()
    } else {
        find_body_wikilinks(&config.vault_path, &old_id, &old_path)?
    };

    // --- Transactional apply ---
    // 1. Snapshot every file we're about to touch
    let mut snapshots: HashMap<PathBuf, String> = HashMap::new();
    snapshots.insert(old_path.clone(), std::fs::read_to_string(&old_path)?);
    for site in &back_ref_sites {
        if !snapshots.contains_key(&site.path) {
            snapshots.insert(site.path.clone(), std::fs::read_to_string(&site.path)?);
        }
    }
    for site in &body_sites {
        if !snapshots.contains_key(&site.path) {
            snapshots.insert(site.path.clone(), std::fs::read_to_string(&site.path)?);
        }
    }

    // 2. Apply, rolling back on any failure
    let result: Result<()> = (|| {
        apply_rename(&old_path, &new_path, &args.new_title)?;
        for site in &back_ref_sites {
            rewrite_frontmatter_back_ref(&site.path, &site.field, &old_id, &new_id, &config.registry)?;
        }
        for site in &body_sites {
            rewrite_body_wikilinks(&site.path, &old_id, &new_id)?;
        }
        Ok(())
    })();

    if let Err(e) = result {
        // Rollback: restore every snapshotted file, and restore the renamed file if needed
        if new_path.exists() && !old_path.exists() {
            // apply_rename removed old_path — restore from snapshot
            if let Some(original) = snapshots.get(&old_path) {
                std::fs::write(&old_path, original)?;
            }
            let _ = std::fs::remove_file(&new_path);
        }
        for (path, original) in &snapshots {
            if *path != old_path && path.exists() {
                std::fs::write(path, original)?;
            }
        }
        return Err(e);
    }

    Ok(())
```

Also update body-site count in the plan output just before the dry-run check:

```rust
    if !body_sites_dry.is_empty() {
        println!("body wikilinks to update: {}", body_sites_dry.len());
    }
```

For the dry-run output, compute `body_sites_dry` by calling `find_body_wikilinks` earlier (regardless of `skip_body`, so dry-run shows what would be touched), then filter at apply time. Adjust `run` to compute once before the `dry_run` branch:

```rust
    // Compute plan
    let back_ref_sites = find_back_refs(&config.vault_path, &old_id, &old_path, &config.registry)?;
    let body_sites_plan = if args.skip_body {
        Vec::new()
    } else {
        find_body_wikilinks(&config.vault_path, &old_id, &old_path)?
    };

    println!(
        "renamed: {} → {}",
        rel(&old_path, &config.vault_path),
        rel(&new_path, &config.vault_path)
    );
    if !back_ref_sites.is_empty() {
        println!("updated {} frontmatter back-reference(s):", back_ref_sites.len());
        for site in &back_ref_sites {
            println!("  {} ({})", rel(&site.path, &config.vault_path), site.field);
        }
    }
    if !body_sites_plan.is_empty() {
        println!("updated {} body wikilink site(s):", body_sites_plan.len());
        for site in &body_sites_plan {
            println!("  {}", rel(&site.path, &config.vault_path));
        }
    }

    if args.dry_run {
        println!("(dry-run, no files written)");
        return Ok(());
    }
```

And use `body_sites_plan` where `body_sites` was referenced in the apply block (rename the variable, drop the duplicate `find_body_wikilinks` call inside the apply section).

- [ ] **Step 3: Run tests**

Run: `cargo test --test rename_test`
Expected: all four rename tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/cli/rename.rs tests/rename_test.rs
git commit -m "feat: rename cascades to body wikilinks with transactional rollback"
```

---

## Task 12: Doctor — `filenames` subcommand with drift + collision + wikilink checks

**Goal:** New `cortx doctor filenames` subcommand runs three checks: filename/title drift, case-insensitive collision, wikilink format. `--fix` auto-repairs drift + bare-string wikilink values.

**Files:**
- Modify: `src/cli/doctor.rs`
- Create: `tests/doctor_filenames_test.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/doctor_filenames_test.rs`:

```rust
mod common;

use assert_cmd::Command;
use common::TestVault;

fn write_schema(vault: &TestVault) {
    vault.write_file("types.yaml", r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
"#);
}

#[test]
fn doctor_filenames_detects_drift() {
    let vault = TestVault::new();
    write_schema(&vault);
    // File named "Wrong Name.md" but title says "Correct Name"
    vault.write_file("tasks/Wrong Name.md",
        "---\ntype: task\ntitle: Correct Name\n---\n");

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "doctor", "filenames"]);
    cmd.assert().failure()
        .stdout(predicates::str::contains("DRIFT"));
}

#[test]
fn doctor_filenames_fix_renames_drifted_file() {
    let vault = TestVault::new();
    write_schema(&vault);
    vault.write_file("tasks/Wrong Name.md",
        "---\ntype: task\ntitle: Correct Name\n---\n");

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "doctor", "filenames", "--fix"]);
    cmd.assert().success();

    assert!(!vault.file_exists("tasks/Wrong Name.md"));
    assert!(vault.file_exists("tasks/Correct Name.md"));
}

#[test]
fn doctor_filenames_detects_bare_link_value() {
    let vault = TestVault::new();
    write_schema(&vault);
    // Create a valid project
    vault.write_file("projects/Website Redesign.md",
        "---\ntype: project\ntitle: Website Redesign\n---\n");
    // Create a task with an UNWRAPPED link value (malformed)
    vault.write_file("tasks/Buy Groceries.md",
        "---\ntype: task\ntitle: Buy Groceries\nproject: Website Redesign\n---\n");

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "doctor", "filenames"]);
    cmd.assert().failure()
        .stdout(predicates::str::contains("WIKILINK FORMAT"));
}

#[test]
fn doctor_filenames_fix_wraps_bare_link_value() {
    let vault = TestVault::new();
    write_schema(&vault);
    vault.write_file("projects/Website Redesign.md",
        "---\ntype: project\ntitle: Website Redesign\n---\n");
    vault.write_file("tasks/Buy Groceries.md",
        "---\ntype: task\ntitle: Buy Groceries\nproject: Website Redesign\n---\n");

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "doctor", "filenames", "--fix"]);
    cmd.assert().success();

    let content = vault.read_file("tasks/Buy Groceries.md");
    assert!(content.contains("\"[[Website Redesign]]\""), "got: {content}");
}
```

Run: `cargo test --test doctor_filenames_test`
Expected: FAIL (subcommand doesn't exist).

- [ ] **Step 2: Add `filenames` subcommand in `doctor.rs`**

In `src/cli/doctor.rs`, extend the enum:

```rust
#[derive(Subcommand)]
pub enum DoctorCommands {
    /// Validate all files against schemas
    Validate,
    /// Check bidirectional relation consistency; use --fix to auto-repair missing inverses
    Links {
        #[arg(long, default_value_t = false)]
        fix: bool,
    },
    /// Check filename/title drift, case-insensitive collisions, and wikilink format
    Filenames {
        #[arg(long, default_value_t = false)]
        fix: bool,
        /// Additionally scan note bodies for unresolved [[wikilinks]]
        #[arg(long, default_value_t = false)]
        check_bodies: bool,
    },
}
```

In the `run` match, add:

```rust
        DoctorCommands::Filenames { fix, check_bodies } => {
            run_filenames_check(config, *fix, *check_bodies)?;
        }
```

At the bottom of the file, add:

```rust
fn run_filenames_check(config: &Config, fix: bool, check_bodies: bool) -> Result<()> {
    use crate::frontmatter::{parse_frontmatter, serialize_entity};
    use crate::slug::sanitize_title;
    use crate::wikilink::{is_wrapped, wrap};
    use std::collections::HashMap;
    use walkdir::WalkDir;

    let mut issues = 0;
    let mut fixed = 0;
    let mut seen_ids: HashMap<String, std::path::PathBuf> = HashMap::new();

    for type_name in config.registry.type_names() {
        let Some(td) = config.registry.get(type_name) else { continue };
        let folder = config.vault_path.join(&td.folder);
        if !folder.exists() { continue; }

        for entry in WalkDir::new(&folder).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };

            // Case-insensitive collision check
            let lower = stem.to_lowercase();
            if let Some(prev) = seen_ids.get(&lower) {
                if prev != path {
                    issues += 1;
                    println!(
                        "CASE COLLISION: {} and {} differ only in case",
                        prev.display(), path.display()
                    );
                }
            } else {
                seen_ids.insert(lower, path.to_path_buf());
            }

            // Read file to check drift and wikilink format
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    println!("READ ERROR: {}: {e}", path.display());
                    issues += 1;
                    continue;
                }
            };
            let (mut fm, body) = match parse_frontmatter(&content) {
                Ok(x) => x,
                Err(e) => {
                    println!("PARSE ERROR: {}: {e}", path.display());
                    issues += 1;
                    continue;
                }
            };

            // Drift check: filename stem should equal sanitize(title)
            let title = fm.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if !title.is_empty() {
                let expected_id = sanitize_title(title);
                if expected_id != stem {
                    issues += 1;
                    println!(
                        "DRIFT: {} (stem={stem}, title='{title}', expected stem='{expected_id}')",
                        path.display()
                    );
                    if fix && !expected_id.is_empty() {
                        let new_path = path.with_file_name(format!("{expected_id}.md"));
                        // Don't overwrite an existing file
                        if !new_path.exists() {
                            std::fs::rename(path, &new_path)?;
                            fixed += 1;
                            println!("  FIXED: renamed to {}", new_path.display());
                        } else {
                            println!("  (skip fix: target {} exists)", new_path.display());
                        }
                    }
                }
            }

            // Wikilink format check for link-typed fields
            use crate::schema::types::FieldType;
            let Some(entity_type) = fm.get("type").and_then(|v| v.as_str()).map(String::from) else { continue };
            let Some(entity_td) = config.registry.get(&entity_type).cloned() else { continue };
            let mut file_fixed = false;
            for (field_name, fd) in &entity_td.fields {
                let is_link = matches!(fd.field_type, FieldType::Link(_) | FieldType::ArrayLink(_));
                if !is_link { continue; }
                let Some(val) = fm.get_mut(field_name) else { continue };
                match val {
                    crate::value::Value::String(s) if !s.is_empty() && !is_wrapped(s) => {
                        issues += 1;
                        println!(
                            "WIKILINK FORMAT: {}.{} = {:?} (not wrapped)",
                            path.display(), field_name, s
                        );
                        if fix {
                            *s = wrap(s);
                            file_fixed = true;
                            println!("  FIXED: wrapped");
                        }
                    }
                    crate::value::Value::Array(items) => {
                        for item in items.iter_mut() {
                            if let crate::value::Value::String(s) = item {
                                if !s.is_empty() && !is_wrapped(s) {
                                    issues += 1;
                                    println!(
                                        "WIKILINK FORMAT: {}.{} contains {:?} (not wrapped)",
                                        path.display(), field_name, s
                                    );
                                    if fix {
                                        *s = wrap(s);
                                        file_fixed = true;
                                        println!("  FIXED: wrapped");
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if file_fixed {
                std::fs::write(path, serialize_entity(&fm, &body))?;
                fixed += 1;
            }

            // Body wikilink integrity (--check-bodies)
            if check_bodies {
                for token in extract_body_wikilinks(&body) {
                    let target_path = find_by_id_global(&config.vault_path, &token, &config.registry);
                    if target_path.is_none() {
                        issues += 1;
                        println!(
                            "UNRESOLVED BODY LINK: {} → [[{token}]]",
                            path.display()
                        );
                    }
                }
            }
        }
    }

    if issues == 0 {
        println!("All filenames, titles, and wikilink formats OK.");
    } else if fix {
        println!("\n{issues} issue(s) found, {fixed} fixed.");
    } else {
        println!("\n{issues} issue(s) found. Run with --fix to auto-repair drift and wikilink format.");
        return Err(CortxError::Validation(format!("{issues} filename/wikilink issue(s)")));
    }
    Ok(())
}

fn extract_body_wikilinks(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let inner = &after[..end];
            if !inner.contains('|') && !inner.trim().is_empty() {
                out.push(inner.trim().to_string());
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    out
}

fn find_by_id_global(
    vault: &std::path::Path,
    id: &str,
    registry: &crate::schema::registry::TypeRegistry,
) -> Option<std::path::PathBuf> {
    for type_name in registry.type_names() {
        let Some(td) = registry.get(type_name) else { continue };
        let path = vault.join(&td.folder).join(format!("{id}.md"));
        if path.exists() { return Some(path); }
    }
    None
}
```

- [ ] **Step 3: Run tests**

Run: `cargo build`
Expected: compiles.

Run: `cargo test --test doctor_filenames_test`
Expected: all 4 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/cli/doctor.rs tests/doctor_filenames_test.rs
git commit -m "feat: doctor filenames subcommand with drift + collision + wikilink checks"
```

---

## Task 13: Adapt `doctor links` for unwrapped titles

**Goal:** The existing `doctor links` checks compare `entity.id` (now a sanitized title) against link-field values (now unwrapped by the read path). Verify it still works end-to-end with the new model.

**Files:**
- Modify: `src/cli/doctor.rs` (only if the check needs updates)
- Modify: `tests/cli_integration_test.rs` or new test

- [ ] **Step 1: Write a test exercising doctor links with new format**

Add to `tests/cli_integration_test.rs` (or a more appropriate test file):

```rust
#[test]
fn doctor_links_works_with_wikilink_format() {
    let vault = TestVault::new();
    vault.write_file("types.yaml", r#"
types:
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type: { const: task }
      title: { type: string }
      project: { type: link, ref: project, inverse: tasks }
  project:
    folder: "projects"
    required: [type, title]
    fields:
      type: { const: project }
      title: { type: string }
      tasks: { type: array[link], ref: task, inverse: project }
"#);

    // Create project and task (wikilink-wrapped automatically via create flow)
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "project", "--title", "Website Redesign"]);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "create", "task", "--title", "Buy Groceries",
              "--set", "project=Website Redesign"]);
    cmd.assert().success();

    // doctor links should be green (bidirectional inverse was auto-applied)
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.args(["--vault", vault.path().to_str().unwrap(),
              "doctor", "links"]);
    cmd.assert().success();

    // Verify the project's tasks array is populated with wrapped title
    let proj = vault.read_file("projects/Website Redesign.md");
    assert!(proj.contains("\"[[Buy Groceries]]\""), "got: {proj}");
}
```

Run: `cargo test --test cli_integration_test doctor_links_works_with_wikilink_format`
Expected: should PASS (read-path unwrap + write-path wrap already in place). If it fails, investigate — most likely `apply_bidirectional` needs to also unwrap values when reading the ref file (done in Task 4).

- [ ] **Step 2: If test passes, commit; if it fails, debug**

If the test passes immediately, no changes to `doctor.rs` are needed.

```bash
git add tests/cli_integration_test.rs
git commit -m "test: verify doctor links works with wikilink-wrapped storage"
```

If it fails, trace the root cause (likely: `apply_bidirectional` in Task 5's changes, or the `entity.id` comparison in `doctor.rs` line 129/135 — the comparison is against `entity.id` which under the new model is the sanitized title, so it should work against unwrapped values). Fix inline.

---

## Task 14: Remove legacy `to_slug` and `--name` slug fallback

**Goal:** Nothing in production code should still call `to_slug`. Remove the function (or keep only as a private helper if the CLI still supports a distinct `--name` path that differs from title).

**Files:**
- Modify: `src/slug.rs`
- Modify: `src/cli/create.rs` (already done in Task 7)

- [ ] **Step 1: Search for remaining usages**

Run: (via Grep tool) `to_slug` across the codebase.
Expected: only references in `src/slug.rs` (test module) and possibly test files.

- [ ] **Step 2: Remove `to_slug` and `deunicode`**

If no production callers remain:

In `src/slug.rs`, delete:
- `use deunicode::deunicode;` at the top
- The `to_slug` function
- The `#[test]` functions that test `to_slug` (keep `sanitize_title` tests)

In `Cargo.toml`, remove:
```toml
deunicode = "1"
```

- [ ] **Step 3: Build and test**

Run: `cargo build`
Expected: compiles.

Run: `cargo test --lib slug::tests`
Expected: only `sanitize_*` tests remain, all PASS.

- [ ] **Step 4: Commit**

```bash
git add src/slug.rs Cargo.toml Cargo.lock
git commit -m "refactor: remove legacy to_slug and deunicode dependency"
```

---

## Task 15: Adapt existing integration tests to title-based model

**Goal:** Rewrite every existing test in `tests/*.rs` whose assertions depend on slug-style filenames or slug-style ids to use the new title format. This is a mechanical sweep of surviving slug-style assertions.

**Files:**
- Modify: `tests/cli_integration_test.rs` (main body)
- Modify: `tests/storage_test.rs` (any remaining slug-based cases)
- Modify: `tests/schema_test.rs`, `tests/query_parser_test.rs`, `tests/query_evaluator_test.rs`, `tests/frontmatter_test.rs`, `tests/value_test.rs` (if needed)

- [ ] **Step 1: Identify broken tests**

Run: `cargo test 2>&1 | grep -E "(FAILED|test result)"`
Record the list of failing tests.

- [ ] **Step 2: Update each failing test**

For each failing test:
- If it asserts on a filename like `foo-bar.md`, change to expected title-form (`Foo Bar.md`)
- If it asserts on `entity.id == "foo-bar"`, change to `entity.id == "Foo Bar"`
- If it creates fixtures with bare link values (`project: some-slug`), wrap them or create the target entity first
- If it uses `--id foo-bar`, change to `--id "Foo Bar"` or drop `--id` and let title-derivation handle it
- If it passes slug strings to `cortx show`/`update`, change to the title form

This is mechanical — there's no single code block that covers it. Work through the list until `cargo test` is green.

- [ ] **Step 3: Run full suite**

Run: `cargo test`
Expected: all tests PASS.

Run: `cargo test --doc`
Expected: doctests PASS (may require updating doctests in `entity.rs` that reference slug IDs).

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -W clippy::all`
Expected: no warnings in new/modified files.

- [ ] **Step 5: Commit**

```bash
git add tests/ src/
git commit -m "test: adapt integration tests to title-based entity model"
```

---

## Task 16: Add `rename_bench` benchmark

**Goal:** Benchmark rename-cascade performance at 100/500/5000 entity vaults.

**Files:**
- Modify: `benches/query_bench.rs`

- [ ] **Step 1: Add rename benchmark**

Append to `benches/query_bench.rs`:

```rust
fn rename_bench(c: &mut Criterion) {
    use cortx::cli::rename::RenameArgs;
    // Construct a test vault with N entities where K reference a common project.
    // Then measure the time to rename that project.
    let mut group = c.benchmark_group("rename_cascade");
    for size in [100, 500, 5000] {
        group.bench_function(format!("N={size}"), |b| {
            b.iter_batched(
                || build_vault_with_refs(size), // setup per iteration
                |(vault, config, args)| {
                    cortx::cli::rename::run(&args, &config).unwrap();
                    drop(vault);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}
```

Write `build_vault_with_refs(size)` as a helper that creates `size` tasks all linking to one "Central Project", returns `(TempDir, Config, RenameArgs { old_title: "Central Project", new_title: "Main Project", dry_run: false, skip_body: false })`.

Register it in the `criterion_group!`:

```rust
criterion_group!(benches, list_all, filter_complex, text_search, rename_bench);
```

Note: `rename::RenameArgs` must be `pub` and `run` must be `pub`. Adjust visibility in `src/cli/rename.rs` if needed.

- [ ] **Step 2: Run bench (smoke test, not full run)**

Run: `cargo bench --bench query_bench -- rename_cascade --sample-size 10`
Expected: runs without error, prints timings.

- [ ] **Step 3: Commit**

```bash
git add benches/query_bench.rs src/cli/rename.rs
git commit -m "bench: rename cascade performance at 100/500/5000 entities"
```

---

## Task 17: Documentation sweep — CLAUDE.md and README.md

**Goal:** Update top-level docs to describe the new title-based model.

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: Update CLAUDE.md**

In `CLAUDE.md`, find the "Non-Obvious Patterns" section. Replace the ID-related bullets:

- Replace the "ID format" bullet to describe the new model:
  ```
  - **ID format**: Derived from `--title` via filesystem-safe sanitization (replace `/ \ : * ? " < > |` and control chars with a space, collapse whitespace, strip trailing dots, NFC-normalize). "Meeting: Q2/Q3 Review" → id=`Meeting Q2 Q3 Review`, filename=`Meeting Q2 Q3 Review.md`. Title is globally unique across the vault (case-insensitive).
  ```
- Replace the "Value coercion" bullet — mention that link-typed fields round-trip through wikilink `[[...]]` wrappers automatically
- Add a new bullet under "Design Principles" or "Non-Obvious Patterns":
  ```
  - **Wikilinks in frontmatter**: Link-typed fields are stored as `"[[Title]]"` strings in YAML frontmatter so Obsidian renders them as clickable links. cortx wraps on write and unwraps on read; queries and CLI args use bare titles.
  ```

In the CLI Commands section, add `cortx rename`:

```bash
cortx rename "Old Title" "New Title"     # Rename entity + cascade updates
cortx doctor filenames [--fix]           # Check filename/title drift, case collisions, wikilink format
```

In the Non-Obvious Patterns, replace example IDs in queries with title forms:
- `'project = "buy-groceries"'` → `'project = "Buy Groceries"'`

- [ ] **Step 2: Update README.md**

In `README.md`, find every CLI example using slug-style IDs and rewrite to title form. Specifically:
- `cortx show buy-groceries` → `cortx show "Buy Groceries"`
- `cortx query 'project = "..."'` examples
- Any "Quick Start" walkthrough

Add a short section describing Obsidian interop:

```markdown
## Obsidian Interop

cortx vaults work natively as Obsidian vaults:
- Entity files use human-readable filenames (`Buy Groceries.md`)
- Link fields in frontmatter are stored as `"[[Title]]"` — Obsidian renders these as clickable links
- Graph view, backlinks, and rename propagation work out of the box
```

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: update CLAUDE.md and README.md for title-based entities"
```

---

## Task 18: Skill sweep — second-brain-protocol and using-cortx-cli

**Goal:** Update skill documentation to match the new model. All example IDs should be titles, all relation examples should describe wikilink storage.

**Files:**
- Modify: `skills/second-brain-protocol/SKILL.md`
- Modify: `skills/using-cortx-cli/SKILL.md` (if exists — check)

- [ ] **Step 1: Update second-brain-protocol SKILL.md**

In `skills/second-brain-protocol/SKILL.md`:

1. **ID format section (around line 210-212)** — replace:
   ```
   **ID format:** Auto-generated as a slug derived from `--title` or `--name` (e.g., `"Buy groceries"` → `buy-groceries`). Unicode is transliterated to ASCII, lowercased, non-alphanumeric runs replaced with hyphens. Override with `--id`. If a slug collides with an existing file, the create command fails — use `--id` to specify a unique name.
   ```
   with:
   ```
   **ID format:** Derived from `--title` via filesystem-safe sanitization — illegal chars (`/ \ : * ? " < > |` and control chars) become spaces, whitespace is collapsed, trailing dots are stripped, NFC-normalized. "Buy Groceries" → id=`Buy Groceries`, filename=`Buy Groceries.md`. Titles must be globally unique (case-insensitive) across the vault. Create fails on collision; the user must choose a different title. Override with `--id`, but the value is still sanitized.
   ```

2. **Links section (around line 214)** — replace:
   ```
   **Links:** Entities reference each other via `link` fields (e.g., `goal: q2-planning`). The value is the ID (filename stem) of the referenced entity. Bidirectional link fields automatically update the inverse field on the referenced entity when a create or update is written.
   ```
   with:
   ```
   **Links:** Entities reference each other via `link` fields. Values are stored as Obsidian wikilinks (`goal: "[[Q2 Planning]]"`) so they render as clickable links in Obsidian. cortx wraps on write and unwraps on read — callers always use bare titles. Create and update verify that the target exists before writing (use `--no-validate-links` to bypass). Bidirectional link fields automatically update the inverse field on the referenced entity.
   ```

3. **Querying relations section (around line 166-178)** — replace slug IDs with titles:
   ```bash
   # All tasks for a goal
   cortx query 'type = "task" and goal = "Q2 Planning"'

   # All notes in an area
   cortx query 'type = "note" and area = "Health"'

   # Timeline for a goal
   cortx query 'type = "log" and goal = "Q2 Planning"' --sort-by date:asc

   # All milestones for a goal
   cortx query 'type = "goal" and up = "Launch v2.0"'
   ```

4. **Recipes section (lines 286-397)** — find every example with a slug ID and rewrite:
   - `goal=q2-planning` → `goal="Q2 Planning"`
   - `update review-pr --set ...` → `update "Review PR" --set ...`
   - `create goal --title "Launch v2.0" ... --set up=launch-v2-0` → `... --set up="Launch v2.0"` (parent goal reference)
   - `archive review-pr` → `archive "Review PR"`
   - `note headings review-q2-goals` → `note headings "Review Q2 Goals"`

5. **Common Mistakes table** — update the "Hard-deleting entities" row: `cortx archive <id>` → `cortx archive "<title>"`.

6. **Add a new row to the Command Reference CRUD table:**
   ```
   | `rename "<old>" "<new>"` | Rename entity (cascades to back-refs) | `--dry-run`, `--skip-body` |
   ```

- [ ] **Step 2: Check for using-cortx-cli skill**

Run: `ls skills/using-cortx-cli/` to check if the skill exists.
If yes, apply the same sweep. If the file was deleted (per git status), no action needed.

- [ ] **Step 3: Commit**

```bash
git add skills/
git commit -m "docs: update second-brain-protocol skill for title-based entities"
```

---

## Task 19: JARVIS playbook sweep

**Goal:** JARVIS skill playbooks reference entity IDs — add a one-line note about the new model.

**Files:**
- Modify: `skills/jarvis/SKILL.md`
- Modify: `skills/jarvis/capture.md`, `ingestion.md`, `prioritization.md`, `daily-brief.md`, `weekly-review.md`, `nudges.md` as needed

- [ ] **Step 1: Skim each playbook**

Read each file under `skills/jarvis/`. For each, check if it references entity IDs in a way that implies slug format.

- [ ] **Step 2: Update references**

Replace any slug-style example IDs with title form. At minimum, update `skills/jarvis/SKILL.md` to mention:

> Entities are referenced by their human title (e.g., `"Q2 Planning"`, `"Buy Groceries"`). Use `cortx show "<title>"` to load an entity and `cortx rename "<old>" "<new>"` to change a title.

- [ ] **Step 3: Commit**

```bash
git add skills/jarvis/
git commit -m "docs: update JARVIS playbooks for title-based entity references"
```

---

## Task 20: Final validation and changelog

**Goal:** Full-suite green, clippy green, benchmarks sane, changelog updated.

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Run full validation**

Run all three and verify:

```bash
cargo build --release
cargo test
cargo clippy -- -W clippy::all -D warnings
```

Expected: all pass with zero warnings.

- [ ] **Step 2: Update CHANGELOG.md**

Add a new section at the top:

```markdown
## [Unreleased]

### Changed (BREAKING)
- Entities are now identified by their human title (filesystem-safe), not a slug. Files are named `Buy Groceries.md` instead of `buy-groceries.md`.
- Link-typed frontmatter fields are stored as Obsidian wikilinks (`project: "[[Website Redesign]]"`). cortx wraps on write and unwraps on read; CLI and query language continue to accept bare titles.
- `cortx show`/`update`/`archive`/`delete`/`note` now take a human title as the positional argument.
- `update --set title=...` is rejected — use `cortx rename` instead.
- `cortx create` rejects on case-insensitive title collision across the vault.

### Added
- `cortx rename "<old>" "<new>"` command with transactional cascade (file rename, frontmatter back-refs, body wikilinks). Flags: `--dry-run`, `--skip-body`.
- `cortx doctor filenames` subcommand: detects filename/title drift, case-insensitive collisions, wikilink format issues. `--fix` auto-repairs drift and wraps bare link values. `--check-bodies` scans note bodies for unresolved wikilinks.
- `--no-validate-links` flag on `create`/`update` bypasses link-target existence checks (for bulk imports).

### Removed
- The `to_slug` function and `deunicode` dependency.
```

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: changelog for title-based entities and wikilink relations"
```

- [ ] **Step 4: Verify clean state**

Run: `git status`
Expected: clean working tree, 20 commits ahead of origin/main.

Run: `cargo test && cargo clippy -- -W clippy::all -D warnings`
Expected: all green.

---
