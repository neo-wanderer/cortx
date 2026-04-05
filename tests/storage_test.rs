mod common;

use common::TestVault;
use cortx::schema::registry::TypeRegistry;
use cortx::storage::file_lock::FileLock;
use cortx::storage::markdown::MarkdownRepository;
use cortx::storage::Repository;
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

    let entity = repo.create("task-001", fm, "", &registry).unwrap();
    assert_eq!(entity.id, "task-001");
    assert!(vault.file_exists("1_Projects/tasks/task-001.md"));
}

#[test]
fn test_get_entity_by_id() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    vault.write_file(
        "5_People/person-jane.md",
        "---\ntype: person\nname: Jane Doe\ntags: []\n---\n# Jane\n",
    );

    let entity = repo.get_by_id("person-jane", &registry).unwrap();
    assert_eq!(entity.id, "person-jane");
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
        "1_Projects/tasks/task-002.md",
        "---\ntype: task\ntitle: Old title\nstatus: open\ntags: []\n---\n# Notes\n",
    );

    let mut updates = HashMap::new();
    updates.insert("status".into(), Value::String("done".into()));

    let entity = repo.update("task-002", updates, &registry).unwrap();
    assert_eq!(entity.get("status").unwrap(), &Value::String("done".into()));

    let content = vault.read_file("1_Projects/tasks/task-002.md");
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
        "1_Projects/tasks/task-del.md",
        "---\ntype: task\ntitle: Delete me\nstatus: open\ntags: []\n---\n",
    );

    repo.delete("task-del", &registry).unwrap();
    assert!(!vault.file_exists("1_Projects/tasks/task-del.md"));
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
        "1_Projects/tasks/task-lock.md",
        "---\ntype: task\ntitle: Lock test\nstatus: open\ntags: []\n---\n",
    );

    let mut updates = HashMap::new();
    updates.insert("status".into(), Value::String("done".into()));

    let entity = repo.update("task-lock", updates, &registry).unwrap();
    assert_eq!(entity.get("status").unwrap(), &Value::String("done".into()));

    assert!(!vault.file_exists("1_Projects/tasks/task-lock.md.lock"));
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

    repo.create("task-dup", fm.clone(), "", &registry).unwrap();

    let err = repo.create("task-dup", fm, "", &registry).unwrap_err();
    assert!(err.to_string().contains("collides"));
}

#[test]
fn read_entity_unwraps_link_fields() {
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
      related: { type: "array[link]", ref: note }
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

#[test]
fn create_wraps_link_fields_in_file() {
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
      related: { type: "array[link]", ref: note }
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

    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Buy Groceries".into()));
    fm.insert("project".into(), Value::String("Website Redesign".into()));
    fm.insert(
        "related".into(),
        Value::Array(vec![Value::String("Weekly Review".into())]),
    );
    repo.create("Buy Groceries", fm, "", &registry).unwrap();

    // Read the raw file and verify wikilinks are present (quote style is serializer's choice)
    let raw = vault.read_file("tasks/Buy Groceries.md");
    assert!(raw.contains("[[Website Redesign]]"), "got: {raw}");
    assert!(raw.contains("[[Weekly Review]]"), "got: {raw}");
    // And the project field line must have the wikilink
    assert!(
        raw.lines()
            .any(|l| l.starts_with("project:") && l.contains("[[Website Redesign]]")),
        "project line missing wikilink: {raw}"
    );
}

#[test]
fn create_rejects_case_insensitive_collision() {
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
fn find_by_title_case_insensitive() {
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

#[test]
fn test_get_nonexistent_entity() {
    let vault = TestVault::new();
    let registry = test_registry();
    let repo = MarkdownRepository::new(vault.path().to_path_buf());

    let err = repo.get_by_id("nonexistent", &registry).unwrap_err();
    assert!(err.to_string().contains("not found"));
}
