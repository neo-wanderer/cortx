mod common;

use assert_cmd::Command;
use common::TestVault;
use predicates::prelude::*;
use std::fs;

fn cortx_cmd(vault: &TestVault) -> Command {
    let mut cmd = Command::cargo_bin("cortx").unwrap();
    cmd.arg("--vault").arg(vault.path().to_str().unwrap());
    if !vault.file_exists("types.yaml") {
        fs::copy("types.yaml", vault.path().join("types.yaml")).unwrap();
    }
    cmd
}

#[test]
fn test_create_and_show_task() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Buy milk", "--id", "task-milk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task-milk"));
    assert!(vault.file_exists("1_Projects/tasks/task-milk.md"));
    cortx_cmd(&vault)
        .args(["show", "task-milk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Buy milk"));
}

#[test]
fn test_create_with_set_fields() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create", "task", "--title", "Fix bug", "--id", "task-fix",
            "--set", "status=in_progress", "--set", "due=2026-04-10",
        ])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/task-fix.md");
    assert!(content.contains("status: in_progress"));
    assert!(content.contains("due: 2026-04-10"));
}

#[test]
fn test_update_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Fix bug", "--id", "task-fix"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["update", "task-fix", "--set", "status=done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated task-fix"));
    let content = vault.read_file("1_Projects/tasks/task-fix.md");
    assert!(content.contains("status: done"));
}

#[test]
fn test_archive_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Archive me", "--id", "task-arch"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["archive", "task-arch"])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/task-arch.md");
    assert!(content.contains("status: archived"));
}

#[test]
fn test_delete_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Temp", "--id", "task-tmp"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["delete", "task-tmp", "--force"])
        .assert()
        .success();
    assert!(!vault.file_exists("1_Projects/tasks/task-tmp.md"));
}

#[test]
fn test_query_filters() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Open task", "--id", "task-open"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create", "task", "--title", "Done task", "--id", "task-done",
            "--set", "status=done",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["query", r#"type = "task" and status = "open""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Open task"))
        .stdout(predicate::str::contains("1 results"));
}

#[test]
fn test_create_person() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create", "person", "--name", "Jane Doe",
            "--id", "person-jane", "--tags", "founder,design",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created person-jane"));
    assert!(vault.file_exists("5_People/person-jane.md"));
}

#[test]
fn test_query_overdue_tasks() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create", "task", "--title", "Overdue task", "--id", "task-overdue",
            "--set", "due=2020-01-01",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["query", r#"type = "task" and status != "done" and due < today"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Overdue task"));
}

#[test]
fn test_meta_distinct_tags() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create", "task", "--title", "T1",
            "--id", "task-t1", "--tags", "home,urgent",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create", "task", "--title", "T2",
            "--id", "task-t2", "--tags", "home,work",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["meta", "distinct", "tags", "--where", r#"type = "task""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("home"))
        .stdout(predicate::str::contains("urgent"))
        .stdout(predicate::str::contains("work"));
}

#[test]
fn test_meta_count_by_status() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "T1", "--id", "task-c1"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "T2", "--id", "task-c2"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create", "task", "--title", "T3", "--id", "task-c3",
            "--set", "status=done",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["meta", "count-by", "status", "--where", r#"type = "task""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("open: 2"))
        .stdout(predicate::str::contains("done: 1"));
}

#[test]
fn test_doctor_validate() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Valid task", "--id", "task-valid"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["doctor", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pass validation"));
}

#[test]
fn test_note_headings_and_insert() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-test.md",
        "---\nid: note-test\ntype: note\ntitle: Test Note\ntags: []\n---\n# Overview\n\nSome text.\n\n## Action Items\n\n- Item 1\n",
    );
    cortx_cmd(&vault)
        .args(["note", "headings", "note-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Overview"))
        .stdout(predicate::str::contains("## Action Items"));
    cortx_cmd(&vault)
        .args([
            "note", "insert-after-heading", "note-test",
            "--heading", "## Action Items",
            "--content", "Item 2 (added by agent)",
        ])
        .assert()
        .success();
    let content = vault.read_file("3_Resources/notes/note-test.md");
    assert!(content.contains("Item 2 (added by agent)"));
}
