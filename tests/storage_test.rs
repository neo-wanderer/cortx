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
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do the thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("tags".into(), Value::Array(vec![]));

    let entity = repo.create(fm, "", &registry).unwrap();
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
        "---\nid: person-jane\ntype: person\nname: Jane Doe\ntags: []\n---\n# Jane\n",
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
        "---\nid: task-002\ntype: task\ntitle: Old title\nstatus: open\ntags: []\n---\n# Notes\n",
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
        "---\nid: task-a\ntype: task\ntitle: A\nstatus: open\ntags: []\n---\n",
    );
    vault.write_file(
        "1_Projects/tasks/task-b.md",
        "---\nid: task-b\ntype: task\ntitle: B\nstatus: done\ntags: []\n---\n",
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
        "---\nid: task-del\ntype: task\ntitle: Delete me\nstatus: open\ntags: []\n---\n",
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
        "---\nid: task-lock\ntype: task\ntitle: Lock test\nstatus: open\ntags: []\n---\n",
    );

    let mut updates = HashMap::new();
    updates.insert("status".into(), Value::String("done".into()));

    let entity = repo.update("task-lock", updates, &registry).unwrap();
    assert_eq!(entity.get("status").unwrap(), &Value::String("done".into()));

    assert!(!vault.file_exists("1_Projects/tasks/task-lock.md.lock"));
}
