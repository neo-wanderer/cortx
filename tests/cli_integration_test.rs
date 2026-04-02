mod common;

use assert_cmd::Command;
use common::TestVault;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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
            "create",
            "task",
            "--title",
            "Fix bug",
            "--id",
            "task-fix",
            "--set",
            "status=in_progress",
            "--set",
            "due=2026-04-10",
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
        .args([
            "create",
            "task",
            "--title",
            "Archive me",
            "--id",
            "task-arch",
        ])
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
        .args([
            "create",
            "task",
            "--title",
            "Open task",
            "--id",
            "task-open",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Done task",
            "--id",
            "task-done",
            "--set",
            "status=done",
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
            "create",
            "person",
            "--name",
            "Jane Doe",
            "--id",
            "person-jane",
            "--tags",
            "founder,design",
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
            "create",
            "task",
            "--title",
            "Overdue task",
            "--id",
            "task-overdue",
            "--set",
            "due=2020-01-01",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "query",
            r#"type = "task" and status != "done" and due < today"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Overdue task"));
}

#[test]
fn test_meta_distinct_tags() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "T1",
            "--id",
            "task-t1",
            "--tags",
            "home,urgent",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "T2",
            "--id",
            "task-t2",
            "--tags",
            "home,work",
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
            "create",
            "task",
            "--title",
            "T3",
            "--id",
            "task-c3",
            "--set",
            "status=done",
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
        .args([
            "create",
            "task",
            "--title",
            "Valid task",
            "--id",
            "task-valid",
        ])
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
            "note",
            "insert-after-heading",
            "note-test",
            "--heading",
            "## Action Items",
            "--content",
            "Item 2 (added by agent)",
        ])
        .assert()
        .success();
    let content = vault.read_file("3_Resources/notes/note-test.md");
    assert!(content.contains("Item 2 (added by agent)"));
}

// -- Init tests --

#[test]
fn test_init_creates_vault_structure() {
    let tmp = TempDir::new().unwrap();
    let vault_path = tmp.path().join("my_vault");
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized cortx vault"));

    assert!(vault_path.join("0_Inbox").exists());
    assert!(vault_path.join("1_Projects/tasks").exists());
    assert!(vault_path.join("3_Resources/notes").exists());
    assert!(vault_path.join("5_People").exists());
    assert!(vault_path.join("5_Companies").exists());
    assert!(vault_path.join("types.yaml").exists());
}

#[test]
fn test_init_does_not_overwrite_existing_types_yaml() {
    let tmp = TempDir::new().unwrap();
    let vault_path = tmp.path().join("vault2");
    fs::create_dir_all(&vault_path).unwrap();
    fs::write(vault_path.join("types.yaml"), "custom: true").unwrap();

    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .success();

    let content = fs::read_to_string(vault_path.join("types.yaml")).unwrap();
    assert_eq!(content, "custom: true");
}

// -- Doctor links tests --

#[test]
fn test_doctor_links_no_broken() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Some task",
            "--id",
            "task-link1",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["doctor", "links"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No broken links"));
}

#[test]
fn test_doctor_links_finds_broken_in_body() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-links.md",
        "---\nid: note-links\ntype: note\ntitle: Links test\ntags: []\n---\nSee [[nonexistent-entity]].\n",
    );
    cortx_cmd(&vault)
        .args(["doctor", "links"])
        .assert()
        .success()
        .stdout(predicate::str::contains("BROKEN LINK"))
        .stdout(predicate::str::contains("nonexistent-entity"));
}

#[test]
fn test_doctor_links_finds_broken_in_frontmatter() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-fmlink.md",
        "---\nid: note-fmlink\ntype: note\ntitle: FM Links\ntags: []\nref: '[[missing-ref]]'\n---\nBody.\n",
    );
    cortx_cmd(&vault)
        .args(["doctor", "links"])
        .assert()
        .success()
        .stdout(predicate::str::contains("BROKEN LINK"))
        .stdout(predicate::str::contains("missing-ref"));
}

#[test]
fn test_doctor_validate_with_errors() {
    let vault = TestVault::new();
    // Write a task file missing required fields
    vault.write_file(
        "1_Projects/tasks/task-bad.md",
        "---\nid: task-bad\ntype: task\n---\nBad task.\n",
    );
    cortx_cmd(&vault)
        .args(["doctor", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("validation error"));
}

// -- Query JSON format --

#[test]
fn test_query_json_format() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "JSON task",
            "--id",
            "task-json",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-json"));
}

#[test]
fn test_query_no_results() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["query", r#"type = "nonexistent""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 0 results"));
}

#[test]
fn test_query_with_status_and_due() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Due task",
            "--id",
            "task-due",
            "--set",
            "due=2026-04-10",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["query", r#"type = "task""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: open"))
        .stdout(predicate::str::contains("due: 2026-04-10"));
}

// -- Note replace-block --

#[test]
fn test_note_replace_block() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-block.md",
        "---\nid: note-block\ntype: note\ntitle: Block test\ntags: []\n---\n# Content\n\n<!-- block:id=summary -->\nOld summary.\n<!-- /block:id=summary -->\n\nMore text.\n",
    );
    cortx_cmd(&vault)
        .args([
            "note",
            "replace-block",
            "note-block",
            "--block-id",
            "summary",
            "--content",
            "New summary content.",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Replaced block"));
    let content = vault.read_file("3_Resources/notes/note-block.md");
    assert!(content.contains("New summary content."));
    assert!(!content.contains("Old summary."));
}

// -- Note read-lines --

#[test]
fn test_note_read_lines() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-lines.md",
        "---\nid: note-lines\ntype: note\ntitle: Lines test\ntags: []\n---\nLine one.\nLine two.\nLine three.\nLine four.\n",
    );
    cortx_cmd(&vault)
        .args([
            "note",
            "read-lines",
            "note-lines",
            "--start",
            "2",
            "--end",
            "3",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Line two."))
        .stdout(predicate::str::contains("Line three."));
}

// -- Note error cases --

#[test]
fn test_note_insert_after_heading_not_found() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-nohead.md",
        "---\nid: note-nohead\ntype: note\ntitle: No Head\ntags: []\n---\nSome text.\n",
    );
    cortx_cmd(&vault)
        .args([
            "note",
            "insert-after-heading",
            "note-nohead",
            "--heading",
            "## Missing",
            "--content",
            "stuff",
        ])
        .assert()
        .failure();
}

#[test]
fn test_note_replace_block_not_found() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-noblock.md",
        "---\nid: note-noblock\ntype: note\ntitle: No Block\ntags: []\n---\nSome text.\n",
    );
    cortx_cmd(&vault)
        .args([
            "note",
            "replace-block",
            "note-noblock",
            "--block-id",
            "missing",
            "--content",
            "stuff",
        ])
        .assert()
        .failure();
}

// -- Create with --name and auto-generated id --

#[test]
fn test_create_with_auto_generated_id() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Auto ID"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task-"));
}

#[test]
fn test_create_with_tags() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Tagged",
            "--id",
            "task-tagged",
            "--tags",
            "alpha, beta",
        ])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/task-tagged.md");
    assert!(content.contains("alpha"));
    assert!(content.contains("beta"));
}

#[test]
fn test_create_with_array_field() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Array set",
            "--id",
            "task-arr",
            "--set",
            "tags=[foo, bar]",
        ])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/task-arr.md");
    assert!(content.contains("foo"));
    assert!(content.contains("bar"));
}

// -- Meta without filter --

#[test]
fn test_meta_distinct_no_filter() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "T1", "--id", "task-nf1"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["meta", "distinct", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("open"));
}

#[test]
fn test_meta_count_by_no_filter() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "T1", "--id", "task-cb1"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["meta", "count-by", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("open: 1"));
}

// -- Config error: non-existent vault --

#[test]
fn test_config_nonexistent_vault() {
    Command::cargo_bin("cortx")
        .unwrap()
        .arg("--vault")
        .arg("/nonexistent/path/vault")
        .args(["query", r#"type = "task""#])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist").or(predicate::str::contains("Error")));
}

// -- Show with body content --

#[test]
fn test_show_with_body() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-body.md",
        "---\nid: note-body\ntype: note\ntitle: Body test\ntags: []\n---\n# Important\n\nThis is the body.\n",
    );
    cortx_cmd(&vault)
        .args(["show", "note-body"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Body test"))
        .stdout(predicate::str::contains("This is the body."));
}

// -- Note replace-block with missing close tag --

#[test]
fn test_note_replace_block_missing_close_tag() {
    let vault = TestVault::new();
    vault.write_file(
        "3_Resources/notes/note-noclose.md",
        "---\nid: note-noclose\ntype: note\ntitle: No Close\ntags: []\n---\n<!-- block:id=test -->\nContent here.\n",
    );
    cortx_cmd(&vault)
        .args([
            "note",
            "replace-block",
            "note-noclose",
            "--block-id",
            "test",
            "--content",
            "new content",
        ])
        .assert()
        .failure();
}

// -- Meta count-by with array field (tags) --

#[test]
fn test_meta_count_by_array_field() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "T1",
            "--id",
            "task-cba1",
            "--tags",
            "alpha,beta",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "T2",
            "--id",
            "task-cba2",
            "--tags",
            "beta,gamma",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["meta", "count-by", "tags", "--where", r#"type = "task""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("beta: 2"))
        .stdout(predicate::str::contains("alpha: 1"))
        .stdout(predicate::str::contains("gamma: 1"));
}

// -- Create duplicate entity --

#[test]
fn test_create_duplicate_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "First", "--id", "task-dup"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Second", "--id", "task-dup"])
        .assert()
        .failure();
}

// -- Show nonexistent entity --

#[test]
fn test_show_nonexistent_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["show", "nonexistent-id"])
        .assert()
        .failure();
}

// -- Config with CORTX_VAULT env var --

#[test]
fn test_config_via_env_var() {
    let vault = TestVault::new();
    fs::copy("types.yaml", vault.path().join("types.yaml")).unwrap();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Env task", "--id", "task-env"])
        .assert()
        .success();

    // Now use CORTX_VAULT env var instead of --vault
    Command::cargo_bin("cortx")
        .unwrap()
        .env("CORTX_VAULT", vault.path().to_str().unwrap())
        .args(["show", "task-env"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Env task"));
}

// -- Config: types.yaml not found --

#[test]
fn test_config_no_types_yaml_found() {
    let tmp = TempDir::new().unwrap();
    // Set CWD to temp dir (no types.yaml anywhere) and use --vault pointing to it
    Command::cargo_bin("cortx")
        .unwrap()
        .current_dir(tmp.path())
        .arg("--vault")
        .arg(tmp.path().to_str().unwrap())
        .args(["query", r#"type = "task""#])
        .assert()
        .failure()
        .stderr(predicate::str::contains("types.yaml not found"));
}

// -- Config: CWD fallback --

#[test]
fn test_config_cwd_fallback() {
    // When no --vault and no CORTX_VAULT, falls back to CWD
    // Run from a temp dir that has types.yaml and vault structure
    let vault = TestVault::new();
    fs::copy("types.yaml", vault.path().join("types.yaml")).unwrap();

    Command::cargo_bin("cortx")
        .unwrap()
        .current_dir(vault.path())
        .env_remove("CORTX_VAULT")
        .args(["query", r#"type = "task""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 0 results"));
}

// -- Update nonexistent entity --

#[test]
fn test_update_nonexistent_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["update", "nonexistent-id", "--set", "status=done"])
        .assert()
        .failure();
}

// -- Delete nonexistent entity --

#[test]
fn test_delete_nonexistent_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["delete", "nonexistent-id", "--force"])
        .assert()
        .failure();
}

// -- Archive nonexistent entity --

#[test]
fn test_archive_nonexistent_entity() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["archive", "nonexistent-id"])
        .assert()
        .failure();
}

// -- Init error (init on a read-only parent is hard to test, but we can test idempotent init) --

#[test]
fn test_init_idempotent() {
    let tmp = TempDir::new().unwrap();
    let vault_path = tmp.path().join("idempotent_vault");
    // Run init twice - should succeed both times
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .success();
}
