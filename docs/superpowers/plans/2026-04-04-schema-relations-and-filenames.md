# Schema Relations & Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enrich `types.yaml` with first-class relation fields (bidirectional, polymorphic, cardinality-inferred), remove `id` from frontmatter (derived from filename), replace UUID filenames with human-readable title slugs, and add `cortx schema validate` + upgraded `cortx doctor links --fix`.

**Architecture:** The changes are tightly coupled and must be executed in dependency order: slug module → Entity identity → create pipeline → types.yaml cleanup → LinkDef schema types (requires updating every match arm simultaneously) → schema validate → bidirectional writes → doctor links upgrade → integration test cleanup.

**Tech Stack:** Rust, `deunicode = "1"` (new), `serde_yaml`, existing `clap`/`rayon`/`regex` stack.

---

## File Map

| File | Change |
|---|---|
| `Cargo.toml` | Add `deunicode = "1"` |
| `src/slug.rs` | **Create** — title → slug conversion |
| `src/lib.rs` | Expose `pub mod slug` |
| `src/entity.rs` | `Entity::new(id, fm, body)` — id explicit, not from frontmatter |
| `src/storage/mod.rs` | `Repository::create(id, fm, body, registry)` — id explicit |
| `src/storage/markdown.rs` | Derive id from `file_stem()`; two-file lock for bidirectional writes |
| `src/cli/create.rs` | Slug from title; no `id` in frontmatter; collision error |
| `types.yaml` | Remove all `id` field declarations and from `required` lists |
| `src/schema/types.rs` | Replace `FieldType::Link` with `Link(LinkDef)` / `ArrayLink(LinkDef)`; add `LinkDef`, `LinkTargets`, `PolyTarget` |
| `src/schema/registry.rs` | Parse new link YAML syntax into `LinkDef` |
| `src/schema/validation.rs` | Handle `Link(LinkDef)` / `ArrayLink(LinkDef)` variants |
| `src/cli/schema.rs` | Add `Validate` subcommand; add `field_type_str` for new variants |
| `src/cli/doctor.rs` | Upgrade `Links` to schema-aware; add `--fix` flag |
| `tests/schema_test.rs` | Add relation parsing tests; remove `id` from YAML fixtures |
| `tests/storage_test.rs` | Update `create()` calls; remove `id` from fixture YAML |
| `tests/cli_integration_test.rs` | Remove `--id` from creates; slug-based IDs throughout |

---

## Task 1: Add `deunicode` and implement `src/slug.rs`

**Files:**
- Modify: `Cargo.toml`
- Create: `src/slug.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing tests in `src/slug.rs`**

```rust
// src/slug.rs
pub fn to_slug(title: &str) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_lowercased_and_hyphenated() {
        assert_eq!(to_slug("Buy groceries"), "buy-groceries");
    }

    #[test]
    fn unicode_transliterated() {
        assert_eq!(to_slug("Réunion café"), "reunion-cafe");
    }

    #[test]
    fn special_chars_stripped() {
        assert_eq!(to_slug("Meeting: John @ Acme"), "meeting-john-acme");
    }

    #[test]
    fn multiple_spaces_collapsed() {
        assert_eq!(to_slug("Q2  Planning"), "q2-planning");
    }

    #[test]
    fn leading_trailing_hyphens_trimmed() {
        assert_eq!(to_slug("  hello world  "), "hello-world");
    }

    #[test]
    fn numbers_preserved() {
        assert_eq!(to_slug("Sprint 3 Goals"), "sprint-3-goals");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test slug
```

Expected: compile error (`todo!()` panics on first test that runs).

- [ ] **Step 3: Add `deunicode` to `Cargo.toml`**

Add this line in `[dependencies]`:
```toml
deunicode = "1"
```

- [ ] **Step 4: Implement `to_slug`**

```rust
// src/slug.rs
use deunicode::deunicode;

/// Convert a title string to a URL-safe slug.
///
/// Rules: transliterate Unicode → ASCII, lowercase, replace non-alphanumeric
/// runs with a single hyphen, trim leading/trailing hyphens.
///
/// # Examples
/// ```
/// use cortx::slug::to_slug;
/// assert_eq!(to_slug("Buy groceries"), "buy-groceries");
/// assert_eq!(to_slug("Réunion café"), "reunion-cafe");
/// ```
pub fn to_slug(title: &str) -> String {
    let ascii = deunicode(title);
    let mut slug = String::new();
    let mut prev_hyphen = true; // suppress leading hyphens
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }
    // Trim trailing hyphen
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_lowercased_and_hyphenated() {
        assert_eq!(to_slug("Buy groceries"), "buy-groceries");
    }

    #[test]
    fn unicode_transliterated() {
        assert_eq!(to_slug("Réunion café"), "reunion-cafe");
    }

    #[test]
    fn special_chars_stripped() {
        assert_eq!(to_slug("Meeting: John @ Acme"), "meeting-john-acme");
    }

    #[test]
    fn multiple_spaces_collapsed() {
        assert_eq!(to_slug("Q2  Planning"), "q2-planning");
    }

    #[test]
    fn leading_trailing_hyphens_trimmed() {
        assert_eq!(to_slug("  hello world  "), "hello-world");
    }

    #[test]
    fn numbers_preserved() {
        assert_eq!(to_slug("Sprint 3 Goals"), "sprint-3-goals");
    }
}
```

- [ ] **Step 5: Expose `slug` module in `src/lib.rs`**

Current `src/lib.rs`:
```rust
pub mod entity;
pub mod error;
pub mod frontmatter;
pub mod query;
pub mod schema;
pub mod storage;
pub mod value;
```

Add `pub mod slug;` after the existing entries:
```rust
pub mod entity;
pub mod error;
pub mod frontmatter;
pub mod query;
pub mod schema;
pub mod slug;
pub mod storage;
pub mod value;
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cargo test slug
```

Expected: 6 tests pass.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/slug.rs src/lib.rs
git commit -m "feat: add slug module with unicode transliteration"
```

---

## Task 2: Update `Entity::new()` — explicit `id` parameter

**Files:**
- Modify: `src/entity.rs`
- Modify: `src/storage/markdown.rs` (read_entity only)

The `id` field is no longer stored in frontmatter. `Entity::new()` receives it as an explicit parameter. `markdown.rs::read_entity()` derives it from the filename stem.

- [ ] **Step 1: Update `Entity::new()` in `src/entity.rs`**

Replace the entire `entity.rs` with:

```rust
use crate::value::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// A Second Brain entity parsed from a Markdown file.
///
/// The `id` is the filename stem — not stored in frontmatter. This keeps
/// frontmatter clean and Obsidian-compatible.
///
/// # Examples
///
/// ```
/// use cortx::entity::Entity;
/// use cortx::value::Value;
/// use std::collections::HashMap;
///
/// let mut fm = HashMap::new();
/// fm.insert("type".into(), Value::String("task".into()));
/// fm.insert("title".into(), Value::String("Buy milk".into()));
///
/// let entity = Entity::new("buy-milk".into(), fm, "# Notes\n".into());
/// assert_eq!(entity.id, "buy-milk");
/// assert_eq!(entity.entity_type, "task");
/// assert_eq!(entity.title(), "Buy milk");
/// ```
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub frontmatter: HashMap<String, Value>,
    pub body: String,
    pub file_path: Option<PathBuf>,
}

impl Entity {
    pub fn new(id: String, frontmatter: HashMap<String, Value>, body: String) -> Self {
        let entity_type = frontmatter
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        Entity {
            id,
            entity_type,
            frontmatter,
            body,
            file_path: None,
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn get(&self, field: &str) -> Option<&Value> {
        self.frontmatter.get(field)
    }

    pub fn title(&self) -> &str {
        self.frontmatter
            .get("title")
            .or_else(|| self.frontmatter.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&self.id)
    }
}
```

- [ ] **Step 2: Update `read_entity` in `src/storage/markdown.rs`**

Replace the `read_entity` method (lines 49-53):

```rust
fn read_entity(&self, path: &Path) -> Result<Entity> {
    let content = std::fs::read_to_string(path)?;
    let (fm, body) = parse_frontmatter(&content)?;
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    Ok(Entity::new(id, fm, body).with_path(path.to_path_buf()))
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo build 2>&1 | head -40
```

Expected: errors only in `markdown.rs::create()` (still reads `id` from frontmatter) and possibly doctests. Other call sites of `Entity::new` are in `markdown.rs` only.

- [ ] **Step 4: Run doctests**

```bash
cargo test --doc
```

Expected: entity doctest passes with new signature.

- [ ] **Step 5: Commit**

```bash
git add src/entity.rs src/storage/markdown.rs
git commit -m "refactor: Entity::new takes explicit id, derive from filename stem"
```

---

## Task 3: Update `Repository::create()` — explicit id, slug from title

**Files:**
- Modify: `src/storage/mod.rs`
- Modify: `src/storage/markdown.rs`
- Modify: `src/cli/create.rs`

- [ ] **Step 1: Write failing integration test in `tests/cli_integration_test.rs`**

Add these tests after the existing ones:

```rust
#[test]
fn test_create_slug_from_title() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Buy groceries"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created buy-groceries"));
    assert!(vault.file_exists("1_Projects/tasks/buy-groceries.md"));
    let content = vault.read_file("1_Projects/tasks/buy-groceries.md");
    assert!(!content.contains("id:"), "id must not appear in frontmatter");
}

#[test]
fn test_create_collision_fails() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Buy groceries"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Buy groceries"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_create_explicit_id_overrides_slug() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Buy groceries", "--id", "2026-04-04-buy-groceries"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created 2026-04-04-buy-groceries"));
    assert!(vault.file_exists("1_Projects/tasks/2026-04-04-buy-groceries.md"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_create_slug_from_title test_create_collision_fails test_create_explicit_id_overrides_slug 2>&1 | tail -20
```

Expected: FAIL — current code still uses UUID IDs and writes `id:` to frontmatter.

- [ ] **Step 3: Update `Repository::create()` signature in `src/storage/mod.rs`**

```rust
pub trait Repository {
    fn create(
        &self,
        id: &str,
        frontmatter: HashMap<String, Value>,
        body: &str,
        registry: &TypeRegistry,
    ) -> Result<Entity>;

    fn get_by_id(&self, id: &str, registry: &TypeRegistry) -> Result<Entity>;

    fn update(
        &self,
        id: &str,
        updates: HashMap<String, Value>,
        registry: &TypeRegistry,
    ) -> Result<Entity>;

    fn delete(&self, id: &str, registry: &TypeRegistry) -> Result<()>;

    fn list_by_type(&self, entity_type: &str, registry: &TypeRegistry) -> Result<Vec<Entity>>;

    fn list_all(&self, registry: &TypeRegistry) -> Result<Vec<Entity>>;
}
```

- [ ] **Step 4: Update `MarkdownRepository::create()` in `src/storage/markdown.rs`**

Replace the `create` method:

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

    let content = serialize_entity(&frontmatter, body);
    std::fs::write(&path, content)?;

    Ok(Entity::new(id.to_string(), frontmatter, body.to_string()).with_path(path))
}
```

- [ ] **Step 5: Update `src/cli/create.rs`**

Replace the entire file:

```rust
use crate::config::Config;
use crate::error::Result;
use crate::slug::to_slug;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::Args;
use std::collections::HashMap;

#[derive(Args)]
pub struct CreateArgs {
    /// Entity type (resolved from types.yaml)
    pub entity_type: String,

    #[arg(long)]
    pub id: Option<String>,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(long)]
    pub name: Option<String>,

    #[arg(long)]
    pub tags: Option<String>,

    #[arg(long = "set", num_args = 1)]
    pub fields: Vec<String>,
}

pub fn run(args: &CreateArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());

    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String(args.entity_type.clone()));

    if let Some(title) = &args.title {
        fm.insert("title".into(), Value::String(title.clone()));
    }
    if let Some(name) = &args.name {
        fm.insert("name".into(), Value::String(name.clone()));
    }

    // Determine id: explicit --id flag, else slug from title/name
    let id = if let Some(explicit) = &args.id {
        explicit.clone()
    } else {
        let base = args.title.as_deref()
            .or(args.name.as_deref())
            .unwrap_or(&args.entity_type);
        to_slug(base)
    };

    if let Some(type_def) = config.registry.get(&args.entity_type)
        && type_def.fields.contains_key("status")
        && !args.fields.iter().any(|f| f.starts_with("status="))
    {
        fm.insert("status".into(), Value::String("open".into()));
    }

    if let Some(tags) = &args.tags {
        let tag_list: Vec<Value> = tags
            .split(',')
            .map(|t| Value::String(t.trim().to_string()))
            .collect();
        fm.insert("tags".into(), Value::Array(tag_list));
    } else {
        fm.insert("tags".into(), Value::Array(vec![]));
    }

    for kv in &args.fields {
        if let Some((k, v)) = kv.split_once('=') {
            let value = parse_cli_value(v);
            fm.insert(k.to_string(), value);
        }
    }

    let today = chrono::Local::now().date_naive();
    fm.insert("created_at".into(), Value::Date(today));
    fm.insert("updated_at".into(), Value::Date(today));

    let entity = repo.create(&id, fm, "", &config.registry)?;
    println!("Created {} ({})", entity.id, entity.entity_type);
    if let Some(path) = &entity.file_path {
        println!("  File: {}", path.display());
    }

    Ok(())
}

pub fn parse_cli_value(v: &str) -> Value {
    match v {
        "today" => return Value::Date(chrono::Local::now().date_naive()),
        "yesterday" => {
            return Value::Date(chrono::Local::now().date_naive() - chrono::Duration::days(1));
        }
        "tomorrow" => {
            return Value::Date(chrono::Local::now().date_naive() + chrono::Duration::days(1));
        }
        _ => {}
    }
    if let Some(date_val) = Value::parse_as_date(v) {
        return date_val;
    }
    if v.starts_with('[') && v.ends_with(']') {
        let inner = &v[1..v.len() - 1];
        let arr: Vec<Value> = inner
            .split(',')
            .map(|s| Value::String(s.trim().trim_matches('"').to_string()))
            .collect();
        return Value::Array(arr);
    }
    Value::String(v.to_string())
}
```

- [ ] **Step 6: Run new tests to verify they pass**

```bash
cargo test test_create_slug_from_title test_create_collision_fails test_create_explicit_id_overrides_slug
```

Expected: 3 PASS.

- [ ] **Step 7: Run full test suite (expect failures in old storage_test fixtures)**

```bash
cargo test 2>&1 | grep -E "^(test |FAILED|error)"
```

Expected: `storage_test` failures because those tests still pass `id` in frontmatter and call the old `repo.create(fm, ...)` signature. We'll fix in Task 9 (test cleanup).

- [ ] **Step 8: Commit**

```bash
git add src/storage/mod.rs src/storage/markdown.rs src/cli/create.rs tests/cli_integration_test.rs
git commit -m "feat: slug-based filenames, id derived from filename, removed from frontmatter"
```

---

## Task 4: Clean up `types.yaml` — remove `id` field

**Files:**
- Modify: `types.yaml`

- [ ] **Step 1: Remove `id` fields from every type in `types.yaml`**

Replace the entire `types.yaml` with:

```yaml
types:
  task:
    folder: "1_Projects/tasks"
    required: [type, title, status]
    fields:
      type:       { const: task }
      title:      { type: string }
      status:     { enum: [open, in_progress, waiting, done, cancelled, archived] }
      project:    { type: link, ref: project }
      area:       { type: link, ref: area }
      assignee:   { type: link, ref: person }
      created_at: { type: date }
      updated_at: { type: date }
      scheduled:  { type: date }
      due:        { type: date }
      tags:       { type: "array[string]", default: "[]" }

  project:
    folder: "1_Projects"
    required: [type, title, status]
    fields:
      type:       { const: project }
      title:      { type: string }
      status:     { enum: [active, on_hold, completed, cancelled, archived] }
      area:       { type: link, ref: area }
      owner:      { type: link, ref: person }
      due:        { type: date }
      created_at: { type: date }
      updated_at: { type: date }
      tags:       { type: "array[string]", default: "[]" }

  area:
    folder: "2_Areas"
    required: [type, title]
    fields:
      type:       { const: area }
      title:      { type: string }
      archived:   { type: bool }
      created_at: { type: date }
      updated_at: { type: date }
      tags:       { type: "array[string]", default: "[]" }

  resource:
    folder: "3_Resources"
    required: [type, title]
    fields:
      type:       { const: resource }
      title:      { type: string }
      area:       { type: link, ref: area }
      created_at: { type: date }
      updated_at: { type: date }
      tags:       { type: "array[string]", default: "[]" }

  note:
    folder: "3_Resources/notes"
    required: [type, title]
    fields:
      type:       { const: note }
      title:      { type: string }
      area:       { type: link, ref: area }
      created_at: { type: date }
      updated_at: { type: date }
      tags:       { type: "array[string]", default: "[]" }

  person:
    folder: "5_People"
    required: [type, name]
    fields:
      type:         { const: person }
      name:         { type: string }
      relationship: { enum: [personal, professional, family, other] }
      company:      { type: link, ref: company }
      email:        { type: string }
      phone:        { type: string }
      created_at:   { type: date }
      updated_at:   { type: date }
      tags:         { type: "array[string]", default: "[]" }

  company:
    folder: "5_Companies"
    required: [type, name]
    fields:
      type:       { const: company }
      name:       { type: string }
      domain:     { type: string }
      industry:   { type: string }
      created_at: { type: date }
      updated_at: { type: date }
      tags:       { type: "array[string]", default: "[]" }
```

- [ ] **Step 2: Verify it loads**

```bash
cargo run -- schema types
```

Expected: lists 7 types without error.

- [ ] **Step 3: Commit**

```bash
git add types.yaml
git commit -m "chore: remove id field from all types in types.yaml"
```

---

## Task 5: Extend schema types — `LinkDef`, update ALL match arms

**Files:**
- Modify: `src/schema/types.rs`
- Modify: `src/schema/registry.rs`
- Modify: `src/schema/validation.rs`
- Modify: `src/cli/schema.rs`

This is one atomic compile unit — changing `FieldType::Link` breaks all downstream match arms. All four files must be updated together.

- [ ] **Step 1: Write failing schema parse tests in `tests/schema_test.rs`**

Add these at the bottom of the file:

```rust
#[test]
fn test_link_single_unidirectional() {
    use cortx::schema::types::{FieldType, LinkTargets};
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type]
    fields:
      type: { const: task }
      area: { type: link, ref: area }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    let area_field = &task_def.fields["area"].field_type;
    let FieldType::Link(link_def) = area_field else { panic!("expected Link") };
    assert!(!link_def.bidirectional);
    let LinkTargets::Single { ref_type, inverse } = &link_def.targets else { panic!("expected Single") };
    assert_eq!(ref_type, "area");
    assert!(inverse.is_none());
}

#[test]
fn test_link_single_bidirectional() {
    use cortx::schema::types::{FieldType, LinkTargets};
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type]
    fields:
      type: { const: task }
      goal:
        type: link
        ref: goal
        bidirectional: true
        inverse: tasks
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    let FieldType::Link(link_def) = &task_def.fields["goal"].field_type else { panic!() };
    assert!(link_def.bidirectional);
    let LinkTargets::Single { ref_type, inverse } = &link_def.targets else { panic!() };
    assert_eq!(ref_type, "goal");
    assert_eq!(inverse.as_deref(), Some("tasks"));
}

#[test]
fn test_link_array_bidirectional() {
    use cortx::schema::types::{FieldType, LinkTargets};
    let yaml = r#"
types:
  note:
    folder: "notes"
    required: [type]
    fields:
      type: { const: note }
      related_goals:
        type: "array[link]"
        ref: goal
        bidirectional: true
        inverse: related_notes
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let note_def = registry.get("note").unwrap();
    let FieldType::ArrayLink(link_def) = &note_def.fields["related_goals"].field_type else { panic!() };
    assert!(link_def.bidirectional);
    let LinkTargets::Single { ref_type, inverse } = &link_def.targets else { panic!() };
    assert_eq!(ref_type, "goal");
    assert_eq!(inverse.as_deref(), Some("related_notes"));
}

#[test]
fn test_link_polymorphic_bidirectional() {
    use cortx::schema::types::{FieldType, LinkTargets};
    let yaml = r#"
types:
  note:
    folder: "notes"
    required: [type]
    fields:
      type: { const: note }
      related:
        type: link
        bidirectional: true
        ref:
          goal: { inverse: related_notes }
          task: { inverse: related_notes }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let note_def = registry.get("note").unwrap();
    let FieldType::Link(link_def) = &note_def.fields["related"].field_type else { panic!() };
    assert!(link_def.bidirectional);
    let LinkTargets::Poly(targets) = &link_def.targets else { panic!() };
    assert_eq!(targets.len(), 2);
    let goal_target = targets.iter().find(|t| t.ref_type == "goal").unwrap();
    assert_eq!(goal_target.inverse.as_deref(), Some("related_notes"));
}

#[test]
fn test_link_polymorphic_unidirectional_array_shorthand() {
    use cortx::schema::types::{FieldType, LinkTargets};
    let yaml = r#"
types:
  log:
    folder: "logs"
    required: [type]
    fields:
      type: { const: log }
      subject:
        type: link
        ref: [goal, task, note]
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let log_def = registry.get("log").unwrap();
    let FieldType::Link(link_def) = &log_def.fields["subject"].field_type else { panic!() };
    assert!(!link_def.bidirectional);
    let LinkTargets::Poly(targets) = &link_def.targets else { panic!() };
    assert_eq!(targets.len(), 3);
    assert!(targets.iter().any(|t| t.ref_type == "goal"));
    assert!(targets.iter().any(|t| t.ref_type == "task"));
    assert!(targets.iter().any(|t| t.ref_type == "note"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_link_ 2>&1 | tail -10
```

Expected: compile error — `LinkTargets`, `PolyTarget` not yet defined.

- [ ] **Step 3: Replace `src/schema/types.rs`**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct PolyTarget {
    pub ref_type: String,
    pub inverse: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkTargets {
    /// ref: "goal" — single target type
    Single { ref_type: String, inverse: Option<String> },
    /// ref: [goal, task] or ref: { goal: { inverse: ... }, ... }
    Poly(Vec<PolyTarget>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkDef {
    pub targets: LinkTargets,
    pub bidirectional: bool,
    /// true = inverse is also a single link (one-to-one). Default: inverse is array[link].
    pub inverse_one: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Date,
    Datetime,
    Bool,
    Number,
    ArrayString,
    Enum(Vec<std::string::String>),
    Const(std::string::String),
    Link(LinkDef),
    ArrayLink(LinkDef),
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<std::string::String>,
}

#[derive(Debug, Clone)]
pub struct TypeDefinition {
    pub name: std::string::String,
    pub folder: std::string::String,
    pub required: Vec<std::string::String>,
    pub fields: HashMap<std::string::String, FieldDefinition>,
}
```

- [ ] **Step 4: Update `src/schema/registry.rs` — parse new link syntax**

Replace `parse_field_def` with:

```rust
fn parse_field_def(
    val: &serde_yaml::Value,
    required_fields: &[String],
    field_name: &str,
) -> Result<FieldDefinition> {
    let field_type = if let Some(const_val) = val.get("const").and_then(|v| v.as_str()) {
        FieldType::Const(const_val.to_string())
    } else if let Some(enum_seq) = val.get("enum").and_then(|v| v.as_sequence()) {
        let variants: Vec<String> = enum_seq
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        FieldType::Enum(variants)
    } else if let Some(type_str) = val.get("type").and_then(|v| v.as_str()) {
        match type_str {
            "string" => FieldType::String,
            "date" => FieldType::Date,
            "datetime" => FieldType::Datetime,
            "bool" => FieldType::Bool,
            "number" => FieldType::Number,
            "array[string]" => FieldType::ArrayString,
            "link" => {
                let link_def = Self::parse_link_def(val)?;
                FieldType::Link(link_def)
            }
            "array[link]" => {
                let link_def = Self::parse_link_def(val)?;
                FieldType::ArrayLink(link_def)
            }
            other => {
                return Err(CortxError::Schema(format!(
                    "unknown field type '{other}' for field '{field_name}'"
                )));
            }
        }
    } else {
        FieldType::String
    };

    let is_required = required_fields.contains(&field_name.to_string());
    let is_optional = val
        .get("optional")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let default = val
        .get("default")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(FieldDefinition {
        field_type,
        required: is_required && !is_optional,
        default,
    })
}

fn parse_link_def(val: &serde_yaml::Value) -> Result<LinkDef> {
    let bidirectional = val
        .get("bidirectional")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let inverse_one = val
        .get("inverse_one")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let ref_val = val.get("ref");
    let targets = match ref_val {
        // ref: "goal"  (single string)
        Some(v) if v.is_str() => {
            let ref_type = v.as_str().unwrap().to_string();
            let inverse = if bidirectional {
                val.get("inverse").and_then(|v| v.as_str()).map(String::from)
            } else {
                None
            };
            LinkTargets::Single { ref_type, inverse }
        }
        // ref: [goal, task, note]  (sequence — polymorphic unidirectional)
        Some(v) if v.is_sequence() => {
            let targets = v
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|t| t.as_str())
                .map(|ref_type| PolyTarget {
                    ref_type: ref_type.to_string(),
                    inverse: None,
                })
                .collect();
            LinkTargets::Poly(targets)
        }
        // ref: { goal: { inverse: related_notes }, ... }  (mapping — polymorphic bidirectional)
        Some(v) if v.is_mapping() => {
            let targets = v
                .as_mapping()
                .unwrap()
                .iter()
                .map(|(k, v)| {
                    let ref_type = k.as_str().unwrap_or("").to_string();
                    let inverse = v
                        .get("inverse")
                        .and_then(|i| i.as_str())
                        .map(String::from);
                    PolyTarget { ref_type, inverse }
                })
                .collect();
            LinkTargets::Poly(targets)
        }
        // No ref — generic untyped link
        None => LinkTargets::Single {
            ref_type: String::new(),
            inverse: None,
        },
        _ => {
            return Err(CortxError::Schema(
                "link field 'ref' must be a string, sequence, or mapping".into(),
            ));
        }
    };

    Ok(LinkDef {
        targets,
        bidirectional,
        inverse_one,
    })
}
```

Also add the missing import at the top of `registry.rs`:

```rust
use super::types::{FieldDefinition, FieldType, LinkDef, LinkTargets, PolyTarget, TypeDefinition};
```

- [ ] **Step 5: Update `src/schema/validation.rs` — handle new FieldType variants**

Replace the match arm for `FieldType::Link` / `FieldType::String` in `validate_frontmatter`:

```rust
FieldType::String
| FieldType::Link(_)
| FieldType::ArrayLink(_)
| FieldType::Datetime => {
    // String-like fields accept any string value; link refs are string IDs
}
```

- [ ] **Step 6: Update `src/cli/schema.rs` — fix `field_type_str` for new variants**

Replace the `field_type_str` function and the `Link` match arm in the JSON output block:

```rust
fn field_type_str(ft: &FieldType) -> String {
    match ft {
        FieldType::String => "string".into(),
        FieldType::Date => "date".into(),
        FieldType::Datetime => "datetime".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Number => "number".into(),
        FieldType::ArrayString => "array[string]".into(),
        FieldType::Const(v) => format!("const:{v}"),
        FieldType::Enum(variants) => format!("enum[{}]", variants.join(",")),
        FieldType::Link(def) => format!("link:{}", link_def_targets_str(def)),
        FieldType::ArrayLink(def) => format!("array[link]:{}", link_def_targets_str(def)),
    }
}

fn link_def_targets_str(def: &LinkDef) -> String {
    match &def.targets {
        LinkTargets::Single { ref_type, .. } => ref_type.clone(),
        LinkTargets::Poly(targets) => {
            targets.iter().map(|t| t.ref_type.as_str()).collect::<Vec<_>>().join("|")
        }
    }
}
```

Also update the JSON output match arm for `Link` (in `SchemaCommands::Show`):

```rust
FieldType::Link(link_def) | FieldType::ArrayLink(link_def) => {
    let is_array = matches!(field.field_type, FieldType::ArrayLink(_));
    obj.insert(
        "type".into(),
        serde_json::Value::String(if is_array { "array[link]" } else { "link" }.into()),
    );
    match &link_def.targets {
        LinkTargets::Single { ref_type, .. } => {
            obj.insert("ref".into(), serde_json::Value::String(ref_type.clone()));
        }
        LinkTargets::Poly(targets) => {
            let refs: Vec<serde_json::Value> = targets
                .iter()
                .map(|t| serde_json::Value::String(t.ref_type.clone()))
                .collect();
            obj.insert("ref".into(), serde_json::Value::Array(refs));
        }
    }
    if link_def.bidirectional {
        obj.insert("bidirectional".into(), serde_json::Value::Bool(true));
    }
}
```

Add the missing imports at the top of `schema.rs`:
```rust
use crate::schema::types::{FieldType, LinkDef, LinkTargets};
```

- [ ] **Step 7: Run the new schema tests**

```bash
cargo test test_link_
```

Expected: 5 PASS.

- [ ] **Step 8: Run full suite to check for regressions**

```bash
cargo test 2>&1 | grep -E "FAILED|error\[" | head -20
```

Expected: existing storage_test failures (still using old `repo.create(fm, ...)` API from Task 3). No new failures from schema changes.

- [ ] **Step 9: Commit**

```bash
git add src/schema/types.rs src/schema/registry.rs src/schema/validation.rs src/cli/schema.rs tests/schema_test.rs
git commit -m "feat: LinkDef relation types with bidirectional, polymorphic, cardinality support"
```

---

## Task 6: Add `cortx schema validate` command

**Files:**
- Modify: `src/cli/schema.rs`

- [ ] **Step 1: Write failing integration test**

Add to `tests/cli_integration_test.rs`:

```rust
#[test]
fn test_schema_validate_valid() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["schema", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_schema_validate_invalid_ref() {
    let vault = TestVault::new();
    // Write a types.yaml with a bad ref
    vault.write_file("types.yaml", r#"
types:
  task:
    folder: "1_Projects/tasks"
    required: [type, title]
    fields:
      type:  { const: task }
      title: { type: string }
      goal:
        type: link
        ref: nonexistent_type
        bidirectional: true
        inverse: tasks
"#);
    // Override the vault's types.yaml
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.arg("--vault").arg(vault.path().to_str().unwrap());
    cmd.args(["schema", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("nonexistent_type"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_schema_validate
```

Expected: FAIL — `Validate` subcommand not yet implemented.

- [ ] **Step 3: Add `Validate` to `SchemaCommands` in `src/cli/schema.rs`**

Add to the `SchemaCommands` enum:

```rust
/// Validate types.yaml — check ref integrity and relation consistency
Validate,
```

- [ ] **Step 4: Implement the `Validate` handler in `schema.rs::run()`**

Add the match arm in `run()`:

```rust
SchemaCommands::Validate => {
    let errors = validate_schema_types(config);
    if errors.is_empty() {
        let type_count = config.registry.type_names().len();
        let (bidir_count, poly_count) = count_relation_stats(config);
        println!(
            "types.yaml is valid ({type_count} types, {bidir_count} bidirectional relations, {poly_count} polymorphic fields)."
        );
    } else {
        for e in &errors {
            println!("{e}");
        }
        let err_count = errors.iter().filter(|e| e.starts_with("ERROR")).count();
        let warn_count = errors.iter().filter(|e| e.starts_with("WARN")).count();
        println!("\n{err_count} error(s), {warn_count} warning(s).");
        return Err(CortxError::Validation(
            format!("{err_count} schema error(s) found"),
        ));
    }
}
```

Add these helper functions at the bottom of `schema.rs`:

```rust
fn validate_schema_types(config: &Config) -> Vec<String> {
    let mut issues: Vec<String> = Vec::new();

    for type_name in config.registry.type_names() {
        let type_def = config.registry.get(type_name).unwrap();

        for (field_name, field_def) in &type_def.fields {
            let (link_def, is_array) = match &field_def.field_type {
                FieldType::Link(d) => (d, false),
                FieldType::ArrayLink(d) => (d, true),
                _ => continue,
            };

            // inverse_one on array[link] is a warning
            if link_def.inverse_one && is_array {
                issues.push(format!(
                    "WARN  {type_name}.{field_name} — inverse_one: true on array[link] field (ignored)"
                ));
            }

            let targets: Vec<(&str, Option<&str>)> = match &link_def.targets {
                LinkTargets::Single { ref_type, inverse } => {
                    if ref_type.is_empty() { continue; }
                    vec![(ref_type.as_str(), inverse.as_deref())]
                }
                LinkTargets::Poly(targets) => targets
                    .iter()
                    .map(|t| (t.ref_type.as_str(), t.inverse.as_deref()))
                    .collect(),
            };

            for (ref_type, inverse) in targets {
                // Check ref type exists
                let target_def = match config.registry.get(ref_type) {
                    Some(d) => d,
                    None => {
                        issues.push(format!(
                            "ERROR  {type_name}.{field_name} — ref type '{ref_type}' not found in registry"
                        ));
                        continue;
                    }
                };

                if !link_def.bidirectional {
                    continue;
                }

                // Check inverse declared
                let inv_field = match inverse {
                    Some(f) => f,
                    None => {
                        issues.push(format!(
                            "ERROR  {type_name}.{field_name} — bidirectional: true but no inverse declared for ref '{ref_type}'"
                        ));
                        continue;
                    }
                };

                // Check inverse field exists on target type
                if !target_def.fields.contains_key(inv_field) {
                    issues.push(format!(
                        "ERROR  {type_name}.{field_name} — inverse '{inv_field}' not found on type '{ref_type}'"
                    ));
                }

                // Check reflexive loop
                if ref_type == type_name && inv_field == field_name {
                    issues.push(format!(
                        "ERROR  {type_name}.{field_name} — reflexive bidirectional link points back to same field"
                    ));
                }
            }
        }
    }

    issues
}

fn count_relation_stats(config: &Config) -> (usize, usize) {
    let mut bidir = 0;
    let mut poly = 0;
    for type_name in config.registry.type_names() {
        let type_def = config.registry.get(type_name).unwrap();
        for field_def in type_def.fields.values() {
            let link_def = match &field_def.field_type {
                FieldType::Link(d) | FieldType::ArrayLink(d) => d,
                _ => continue,
            };
            if link_def.bidirectional {
                bidir += 1;
            }
            if matches!(link_def.targets, LinkTargets::Poly(_)) {
                poly += 1;
            }
        }
    }
    (bidir, poly)
}
```

Add missing import at top of `schema.rs`:
```rust
use crate::error::CortxError;
```

- [ ] **Step 5: Run tests**

```bash
cargo test test_schema_validate
```

Expected: 2 PASS.

- [ ] **Step 6: Commit**

```bash
git add src/cli/schema.rs tests/cli_integration_test.rs
git commit -m "feat: cortx schema validate command with ref integrity and inverse checks"
```

---

## Task 7: Bidirectional writes in `src/storage/markdown.rs`

**Files:**
- Modify: `src/storage/markdown.rs`

When `create` or `update` sets a link field that is `bidirectional: true`, the referenced entity's inverse field must be updated atomically (two-file lock, lower path first).

- [ ] **Step 1: Write failing integration test**

Add to `tests/cli_integration_test.rs`:

```rust
#[test]
fn test_bidirectional_create_updates_inverse() {
    let vault = TestVault::new();
    // Write a types.yaml with a bidirectional relation: task.goal ↔ goal.tasks
    vault.write_file("types.yaml", r#"
types:
  goal:
    folder: "goals"
    required: [type, title]
    fields:
      type:  { const: goal }
      title: { type: string }
      tasks: { type: "array[link]", ref: task }
      tags:  { type: "array[string]", default: "[]" }
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type:  { const: task }
      title: { type: string }
      goal:
        type: link
        ref: goal
        bidirectional: true
        inverse: tasks
      tags:  { type: "array[string]", default: "[]" }
"#);
    // Create the goal first
    cortx_cmd(&vault)
        .args(["create", "goal", "--title", "Q2 Goals"])
        .assert()
        .success();
    // Create a task linked to that goal
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Fix login bug", "--set", "goal=q2-goals"])
        .assert()
        .success();
    // The goal file should now have tasks: [fix-login-bug]
    let goal_content = vault.read_file("goals/q2-goals.md");
    assert!(
        goal_content.contains("fix-login-bug"),
        "goal.tasks should contain the new task id: {goal_content}"
    );
}

#[test]
fn test_bidirectional_update_adds_to_inverse() {
    let vault = TestVault::new();
    vault.write_file("types.yaml", r#"
types:
  goal:
    folder: "goals"
    required: [type, title]
    fields:
      type:  { const: goal }
      title: { type: string }
      tasks: { type: "array[link]", ref: task }
      tags:  { type: "array[string]", default: "[]" }
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type:  { const: task }
      title: { type: string }
      goal:
        type: link
        ref: goal
        bidirectional: true
        inverse: tasks
      tags:  { type: "array[string]", default: "[]" }
"#);
    cortx_cmd(&vault)
        .args(["create", "goal", "--title", "Q2 Goals"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Fix login bug"])
        .assert()
        .success();
    // Now link the task to the goal via update
    cortx_cmd(&vault)
        .args(["update", "fix-login-bug", "--set", "goal=q2-goals"])
        .assert()
        .success();
    let goal_content = vault.read_file("goals/q2-goals.md");
    assert!(
        goal_content.contains("fix-login-bug"),
        "goal.tasks should contain the task after update: {goal_content}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_bidirectional_
```

Expected: FAIL — bidirectional writes not yet implemented.

- [ ] **Step 3: Add bidirectional write helper to `src/storage/markdown.rs`**

Add this method to `MarkdownRepository`:

```rust
/// If `field_name` is a bidirectional link in the schema, update the inverse
/// field on the referenced entity. Acquires locks on both files; lower path first.
fn apply_bidirectional(
    &self,
    owning_path: &Path,
    owning_id: &str,
    field_name: &str,
    new_value: &Value,
    registry: &TypeRegistry,
    type_def: &crate::schema::types::TypeDefinition,
) -> Result<()> {
    use crate::schema::types::{FieldType, LinkTargets};

    let field_def = match type_def.fields.get(field_name) {
        Some(f) => f,
        None => return Ok(()),
    };

    let link_def = match &field_def.field_type {
        FieldType::Link(d) | FieldType::ArrayLink(d) if d.bidirectional => d,
        _ => return Ok(()),
    };

    let ref_id = match new_value.as_str() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Ok(()),
    };

    let (ref_type, inverse_field) = match &link_def.targets {
        LinkTargets::Single { ref_type, inverse: Some(inv) } => (ref_type.as_str(), inv.as_str()),
        LinkTargets::Poly(targets) => {
            // Find matching target by looking up the ref_id's type
            let matched = targets.iter().find_map(|t| {
                let ref_path = self.resolve_path(&t.ref_type, &ref_id, registry).ok()?;
                if ref_path.exists() {
                    t.inverse.as_deref().map(|inv| (t.ref_type.as_str(), inv))
                } else {
                    None
                }
            });
            match matched {
                Some((rt, inv)) => (rt, inv),
                None => return Ok(()),
            }
        }
        _ => return Ok(()),
    };

    let ref_path = self.resolve_path(ref_type, &ref_id, registry)?;
    if !ref_path.exists() {
        return Ok(()); // referenced entity doesn't exist yet, skip silently
    }

    // Lock ordering: lexicographically lower path first to prevent deadlocks
    let (first_path, second_path) = if owning_path < ref_path.as_path() {
        (owning_path, ref_path.as_path())
    } else {
        (ref_path.as_path(), owning_path)
    };

    let _lock1 = file_lock::FileLock::acquire(first_path)?;
    let _lock2 = file_lock::FileLock::acquire(second_path)?;

    // Read the referenced entity and append owning_id to the inverse array field
    let ref_content = std::fs::read_to_string(&ref_path)?;
    let (mut ref_fm, ref_body) = parse_frontmatter(&ref_content)?;

    let arr = ref_fm
        .entry(inverse_field.to_string())
        .or_insert_with(|| Value::Array(vec![]));

    if let Value::Array(items) = arr {
        let id_val = Value::String(owning_id.to_string());
        if !items.contains(&id_val) {
            items.push(id_val);
        }
    }

    let updated = serialize_entity(&ref_fm, &ref_body);
    std::fs::write(&ref_path, updated)?;

    Ok(())
}
```

- [ ] **Step 4: Call `apply_bidirectional` from `create()` in `markdown.rs`**

After the line `std::fs::write(&path, content)?;` in `create()`, add:

```rust
// Maintain bidirectional inverse fields
for (field_name, value) in &frontmatter {
    self.apply_bidirectional(&path, id, field_name, value, registry, type_def)?;
}
```

- [ ] **Step 5: Call `apply_bidirectional` from `update()` in `markdown.rs`**

After `entity.frontmatter.insert(key.clone(), val.clone());` in the update loop, before `validate_frontmatter`, add a second loop after writing:

In the `update()` method, after `std::fs::write(&path, content)?;`, add:

```rust
// Maintain bidirectional inverse fields for any link fields that changed
let type_def_for_bidir = registry.get(&entity.entity_type);
if let Some(type_def) = type_def_for_bidir {
    for (field_name, value) in &updates_snapshot {
        self.apply_bidirectional(&path, id, field_name, value, registry, type_def)?;
    }
}
```

To make `updates_snapshot` available, capture the updates before the loop at the top of `update()`:

```rust
let updates_snapshot = updates.clone();
```

- [ ] **Step 6: Run bidirectional tests**

```bash
cargo test test_bidirectional_
```

Expected: 2 PASS.

- [ ] **Step 7: Commit**

```bash
git add src/storage/markdown.rs tests/cli_integration_test.rs
git commit -m "feat: atomic bidirectional writes with two-file lock ordering"
```

---

## Task 8: Upgrade `cortx doctor links` — schema-aware + `--fix`

**Files:**
- Modify: `src/cli/doctor.rs`

Replace the regex-based `[[wiki-link]]` scanner with a schema-aware check that reads `LinkDef` metadata and verifies inverse fields are consistent.

- [ ] **Step 1: Write failing integration test**

Add to `tests/cli_integration_test.rs`:

```rust
#[test]
fn test_doctor_links_detects_missing_inverse() {
    let vault = TestVault::new();
    vault.write_file("types.yaml", r#"
types:
  goal:
    folder: "goals"
    required: [type, title]
    fields:
      type:  { const: goal }
      title: { type: string }
      tasks: { type: "array[link]", ref: task }
      tags:  { type: "array[string]", default: "[]" }
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type:  { const: task }
      title: { type: string }
      goal:
        type: link
        ref: goal
        bidirectional: true
        inverse: tasks
      tags:  { type: "array[string]", default: "[]" }
"#);
    // Create a goal with NO tasks array
    vault.write_file("goals/q2-goals.md", "---\ntype: goal\ntitle: Q2 Goals\ntags: []\n---\n");
    // Create a task that claims to belong to q2-goals — but q2-goals.tasks is missing
    vault.write_file("tasks/fix-login.md", "---\ntype: task\ntitle: Fix login\ngoal: q2-goals\ntags: []\n---\n");

    cortx_cmd(&vault)
        .args(["doctor", "links"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("fix-login"));
}

#[test]
fn test_doctor_links_fix_repairs_inverse() {
    let vault = TestVault::new();
    vault.write_file("types.yaml", r#"
types:
  goal:
    folder: "goals"
    required: [type, title]
    fields:
      type:  { const: goal }
      title: { type: string }
      tasks: { type: "array[link]", ref: task }
      tags:  { type: "array[string]", default: "[]" }
  task:
    folder: "tasks"
    required: [type, title]
    fields:
      type:  { const: task }
      title: { type: string }
      goal:
        type: link
        ref: goal
        bidirectional: true
        inverse: tasks
      tags:  { type: "array[string]", default: "[]" }
"#);
    vault.write_file("goals/q2-goals.md", "---\ntype: goal\ntitle: Q2 Goals\ntags: []\n---\n");
    vault.write_file("tasks/fix-login.md", "---\ntype: task\ntitle: Fix login\ngoal: q2-goals\ntags: []\n---\n");

    // Run with --fix
    cortx_cmd(&vault)
        .args(["doctor", "links", "--fix"])
        .assert()
        .success();

    // Goal should now contain the task in its tasks field
    let goal_content = vault.read_file("goals/q2-goals.md");
    assert!(
        goal_content.contains("fix-login"),
        "goal.tasks should contain fix-login after --fix: {goal_content}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_doctor_links_
```

Expected: FAIL — current `doctor links` uses regex, not schema-aware.

- [ ] **Step 3: Replace `DoctorCommands::Links` handler in `src/cli/doctor.rs`**

Add `--fix` flag to `DoctorCommands`:

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
}
```

Replace the `DoctorCommands::Links` match arm:

```rust
DoctorCommands::Links { fix } => {
    use crate::schema::types::{FieldType, LinkTargets};

    let repo = MarkdownRepository::new(config.vault_path.clone());
    let all = repo.list_all(&config.registry)?;
    let mut issues = 0;
    let mut repaired = 0;

    for entity in &all {
        let type_def = match config.registry.get(&entity.entity_type) {
            Some(d) => d,
            None => continue,
        };

        for (field_name, field_def) in &type_def.fields {
            let link_def = match &field_def.field_type {
                FieldType::Link(d) | FieldType::ArrayLink(d) if d.bidirectional => d,
                _ => continue,
            };

            let ref_id = match entity.frontmatter.get(field_name).and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            };

            let (ref_type_name, inverse_field) = match &link_def.targets {
                LinkTargets::Single { ref_type, inverse: Some(inv) } => {
                    (ref_type.clone(), inv.clone())
                }
                LinkTargets::Poly(targets) => {
                    let matched = targets.iter().find_map(|t| {
                        let ref_path = config.vault_path
                            .join(&config.registry.get(&t.ref_type)?.folder)
                            .join(format!("{ref_id}.md"));
                        if ref_path.exists() {
                            t.inverse.clone().map(|inv| (t.ref_type.clone(), inv))
                        } else {
                            None
                        }
                    });
                    match matched {
                        Some(pair) => pair,
                        None => continue,
                    }
                }
                _ => continue,
            };

            // Find the referenced entity
            let ref_entity = match repo.get_by_id(&ref_id, &config.registry) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check if owning entity's id is in the inverse field
            let has_back_ref = match ref_entity.frontmatter.get(&inverse_field) {
                Some(Value::Array(items)) => {
                    items.contains(&Value::String(entity.id.clone()))
                }
                _ => false,
            };

            if !has_back_ref {
                issues += 1;
                println!(
                    "MISSING INVERSE: {}.{} = {} — {}.{} does not contain {}",
                    entity.id, field_name, ref_id,
                    ref_id, inverse_field, entity.id
                );

                if *fix {
                    let mut updates = HashMap::new();
                    // Read current array and append
                    let mut items = match ref_entity.frontmatter.get(&inverse_field) {
                        Some(Value::Array(arr)) => arr.clone(),
                        _ => vec![],
                    };
                    items.push(Value::String(entity.id.clone()));
                    updates.insert(inverse_field.clone(), Value::Array(items));
                    repo.update(&ref_id, updates, &config.registry)?;
                    repaired += 1;
                    println!("  FIXED");
                }
            }
        }
    }

    if issues == 0 {
        println!("No bidirectional relation inconsistencies found across {} entities.", all.len());
    } else if *fix {
        println!("\n{issues} issue(s) found, {repaired} repaired.");
    } else {
        println!("\n{issues} issue(s) found. Run with --fix to auto-repair.");
        return Err(CortxError::Validation(format!("{issues} relation inconsistency/ies")));
    }
}
```

Add missing imports at top of `doctor.rs`:

```rust
use crate::error::CortxError;
use crate::schema::types::{FieldType, LinkTargets};
use crate::value::Value;
use std::collections::HashMap;
```

Remove the old `use regex::Regex;` import (no longer needed).

- [ ] **Step 4: Run tests**

```bash
cargo test test_doctor_links_
```

Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cli/doctor.rs tests/cli_integration_test.rs
git commit -m "feat: schema-aware doctor links with --fix flag for relation repair"
```

---

## Task 9: Update all existing tests for new ID model

**Files:**
- Modify: `tests/storage_test.rs`
- Modify: `tests/schema_test.rs`
- Modify: `tests/cli_integration_test.rs`

Existing tests use `id:` in frontmatter YAML and pass `id` in the `fm` HashMap. These must be updated to match the new model.

- [ ] **Step 1: Run the full test suite to see what's failing**

```bash
cargo test 2>&1 | grep "FAILED"
```

- [ ] **Step 2: Update `tests/storage_test.rs`**

Replace the entire file:

```rust
mod common;

use common::TestVault;
use cortx::schema::registry::TypeRegistry;
use cortx::storage::Repository;
use cortx::storage::file_lock::FileLock;
use cortx::storage::markdown::MarkdownRepository;
use cortx::value::Value;
use std::collections::HashMap;

fn test_registry() -> TypeRegistry {
    TypeRegistry::from_yaml_file(std::path::Path::new("types.yaml")).unwrap()
}

#[test]
fn test_create_entity() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do the thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("tags".into(), Value::Array(vec![]));

    let entity = repo.create("do-the-thing", fm, "", &registry).unwrap();
    assert_eq!(entity.id, "do-the-thing");
    assert!(vault.file_exists("1_Projects/tasks/do-the-thing.md"));
}

#[test]
fn test_get_entity_by_id() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    vault.write_file(
        "5_People/jane-doe.md",
        "---\ntype: person\nname: Jane Doe\ntags: []\n---\n# Jane\n",
    );

    let entity = repo.get_by_id("jane-doe", &registry).unwrap();
    assert_eq!(entity.id, "jane-doe");
    assert_eq!(
        entity.get("name").unwrap(),
        &Value::String("Jane Doe".into())
    );
}

#[test]
fn test_update_entity() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    vault.write_file(
        "1_Projects/tasks/old-title.md",
        "---\ntype: task\ntitle: Old title\nstatus: open\ntags: []\n---\n# Notes\n",
    );

    let mut updates = HashMap::new();
    updates.insert("status".into(), Value::String("done".into()));

    let entity = repo.update("old-title", updates, &registry).unwrap();
    assert_eq!(entity.get("status").unwrap(), &Value::String("done".into()));

    let content = vault.read_file("1_Projects/tasks/old-title.md");
    assert!(content.contains("status: done"));
}

#[test]
fn test_list_entities_by_type() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    vault.write_file(
        "1_Projects/tasks/task-a.md",
        "---\ntype: task\ntitle: A\nstatus: open\ntags: []\n---\n",
    );
    vault.write_file(
        "1_Projects/tasks/task-b.md",
        "---\ntype: task\ntitle: B\nstatus: done\ntags: []\n---\n",
    );

    let entities = repo.list_by_type("task", &registry).unwrap();
    assert_eq!(entities.len(), 2);
}

#[test]
fn test_delete_entity() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    vault.write_file(
        "1_Projects/tasks/delete-me.md",
        "---\ntype: task\ntitle: Delete me\nstatus: open\ntags: []\n---\n",
    );

    repo.delete("delete-me", &registry).unwrap();
    assert!(!vault.file_exists("1_Projects/tasks/delete-me.md"));
}

#[test]
fn test_file_lock_acquire_and_release() {
    let vault = TestVault::new();
    let lock_path = vault.path().join("test.md");
    std::fs::write(&lock_path, "test").unwrap();

    let lock = FileLock::acquire(&lock_path).unwrap();
    assert!(vault.path().join("test.md.lock").exists());

    lock.release().unwrap();
    assert!(!vault.path().join("test.md.lock").exists());
}

#[test]
fn test_file_lock_contention() {
    let vault = TestVault::new();
    let lock_path = vault.path().join("test.md");
    std::fs::write(&lock_path, "test").unwrap();

    let lock1 = FileLock::acquire(&lock_path).unwrap();

    let lock2 = FileLock::acquire(&lock_path);
    assert!(lock2.is_err());
    let err = lock2.unwrap_err().to_string();
    assert!(err.contains("locked"), "error should mention lock: {err}");

    lock1.release().unwrap();

    let lock3 = FileLock::acquire(&lock_path).unwrap();
    lock3.release().unwrap();
}

#[test]
fn test_file_lock_drop_releases() {
    let vault = TestVault::new();
    let lock_path = vault.path().join("test.md");
    std::fs::write(&lock_path, "test").unwrap();

    {
        let _lock = FileLock::acquire(&lock_path).unwrap();
        assert!(vault.path().join("test.md.lock").exists());
    }
    assert!(!vault.path().join("test.md.lock").exists());
}

#[test]
fn test_update_with_locking() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    vault.write_file(
        "1_Projects/tasks/lock-test.md",
        "---\ntype: task\ntitle: Lock test\nstatus: open\ntags: []\n---\n",
    );

    let mut updates = HashMap::new();
    updates.insert("status".into(), Value::String("done".into()));

    let entity = repo.update("lock-test", updates, &registry).unwrap();
    assert_eq!(entity.get("status").unwrap(), &Value::String("done".into()));
    assert!(!vault.file_exists("1_Projects/tasks/lock-test.md.lock"));
}

#[test]
fn test_create_duplicate_entity() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("First".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("tags".into(), Value::Array(vec![]));

    repo.create("first", fm.clone(), "", &registry).unwrap();
    let err = repo.create("first", fm, "", &registry).unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn test_get_nonexistent_entity() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    let err = repo.get_by_id("nonexistent", &registry).unwrap_err();
    assert!(err.to_string().contains("not found"));
}
```

- [ ] **Step 3: Update `tests/schema_test.rs` — remove `id` from YAML fixtures**

Replace all occurrences of `required: [id, type, title, status]` with `required: [type, title, status]` and remove `id: { type: string }` field lines in the inline YAML strings. Also remove `id` from `assert!(task_def.required.contains(&"id".to_string()))` assertions.

Key replacements in `schema_test.rs`:

```rust
// Before (in every test YAML fixture):
//   required: [id, type, title, status]
//   fields:
//     id:       { type: string }
// After:
//   required: [type, title, status]
//   fields:
//     (no id line)

// Before:
//   assert!(task_def.required.contains(&"id".to_string()));
// After: remove that assertion
```

- [ ] **Step 4: Update old `cli_integration_test.rs` tests**

The tests that use explicit `--id` for entity creation (e.g. `--id task-milk`) should switch to using title slugs or keep explicit IDs where appropriate. Replace each test:

```rust
// test_create_and_show_task: use slug instead
#[test]
fn test_create_and_show_task() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Buy milk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created buy-milk"));
    assert!(vault.file_exists("1_Projects/tasks/buy-milk.md"));
    let content = vault.read_file("1_Projects/tasks/buy-milk.md");
    assert!(!content.contains("id:"), "id must not be in frontmatter");
    cortx_cmd(&vault)
        .args(["show", "buy-milk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Buy milk"));
}

// test_create_with_set_fields
#[test]
fn test_create_with_set_fields() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create", "task",
            "--title", "Fix bug",
            "--set", "status=in_progress",
            "--set", "due=2026-04-10",
        ])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/fix-bug.md");
    assert!(content.contains("status: in_progress"));
    assert!(content.contains("due: 2026-04-10"));
}

// test_update_entity
#[test]
fn test_update_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Fix bug"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["update", "fix-bug", "--set", "status=done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated fix-bug"));
    let content = vault.read_file("1_Projects/tasks/fix-bug.md");
    assert!(content.contains("status: done"));
}
```

Apply the same pattern (remove `--id`, derive ID from title slug) to all remaining tests: `test_archive_entity`, `test_delete_entity`, `test_query_by_type`, `test_query_filter`, `test_schema_types`, `test_schema_show`, etc.

- [ ] **Step 5: Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy -- -W clippy::all 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 7: Run doctests**

```bash
cargo test --doc
```

Expected: all pass.

- [ ] **Step 8: Final commit**

```bash
git add tests/storage_test.rs tests/schema_test.rs tests/cli_integration_test.rs
git commit -m "test: update all tests for slug-based IDs and no frontmatter id field"
```

---

## Self-Review

**Spec coverage check:**

| Spec section | Covered by task |
|---|---|
| Relationship schema — inline on owning field | Task 5 (types.rs + registry.rs) |
| `type: link` vs `type: "array[link]"` cardinality | Task 5 |
| Polymorphic `ref` map | Task 5 |
| `inverse_one: true` one-to-one | Task 5 |
| Atomic bidirectional writes (two-file lock) | Task 7 |
| Lock ordering (lower path first) | Task 7 |
| `cortx doctor links` crash repair path | Task 8 |
| ID removed from frontmatter | Task 2 + Task 3 |
| Obsidian-compatible frontmatter | Task 2 (no `id:` written) |
| Human-readable filename from title slug | Task 3 |
| Unicode transliteration via `deunicode` | Task 1 |
| Collision → fail with error | Task 3 |
| `--id` override for custom names | Task 3 |
| `types.yaml` `id` field removal | Task 4 |
| `cortx schema validate` command | Task 6 |
| Ref integrity check | Task 6 |
| Inverse field existence check | Task 6 |
| Polymorphic inverse check | Task 6 |
| `inverse_one` constraint check | Task 6 |
| Reflexive loop check | Task 6 |
| `doctor links --fix` report-only default | Task 8 |
| `doctor links --fix` auto-repair flag | Task 8 |

All spec sections covered. No gaps found.
