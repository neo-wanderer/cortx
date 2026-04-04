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

    assert!(vault_path.join("types.yaml").exists());
    // Structural PARA folders always created
    assert!(vault_path.join("0_Inbox").exists());
    assert!(vault_path.join("4_Archive").exists());
    // All type folders from types.yaml are created
    let registry = cortx::schema::registry::TypeRegistry::from_yaml_file(
        &vault_path.join("types.yaml"),
    )
    .unwrap();
    for type_name in registry.type_names() {
        let def = registry.get(type_name).unwrap();
        if !def.folder.is_empty() {
            assert!(
                vault_path.join(&def.folder).exists(),
                "folder '{}' for type '{}' was not created",
                def.folder,
                type_name
            );
        }
    }
}

#[test]
fn test_init_custom_type_folder_created_on_write() {
    // Verify that a type with a custom folder defined in types.yaml
    // gets its folder auto-created when the first entity is written.
    let tmp = TempDir::new().unwrap();
    let vault_path = tmp.path().join("custom_vault");
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .success();

    // Add a custom type to the vault's types.yaml
    let custom_types = r#"types:
  recipe:
    folder: "6_Recipes"
    required: [id, type, title]
    fields:
      id:    { type: string }
      type:  { const: recipe }
      title: { type: string }
      tags:  { type: "array[string]", default: "[]" }
"#;
    fs::write(vault_path.join("types.yaml"), custom_types).unwrap();

    // The folder should not exist yet
    assert!(!vault_path.join("6_Recipes").exists());

    // Creating an entity of the new type should auto-create the folder
    Command::cargo_bin("cortx")
        .unwrap()
        .args([
            "--vault",
            vault_path.to_str().unwrap(),
            "create",
            "recipe",
            "--title",
            "Pasta",
            "--id",
            "recipe-pasta",
        ])
        .assert()
        .success();

    assert!(vault_path.join("6_Recipes").exists());
    assert!(vault_path.join("6_Recipes/recipe-pasta.md").exists());
}

#[test]
fn test_init_fails_if_types_yaml_exists() {
    let tmp = TempDir::new().unwrap();
    let vault_path = tmp.path().join("vault2");
    fs::create_dir_all(&vault_path).unwrap();
    fs::write(vault_path.join("types.yaml"), "custom: true").unwrap();

    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("vault already initialized at"));

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

// -- Init error (duplicate init guard) --

#[test]
fn test_init_duplicate_guard() {
    let tmp = TempDir::new().unwrap();
    let vault_path = tmp.path().join("idempotent_vault");
    // Run init once - should succeed
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .success();
    // Run init again - should fail with "already initialized" error
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("vault already initialized at"));
}

// -- Quoted field names in queries --

#[test]
fn test_query_quoted_field_name() {
    let vault = TestVault::new();
    // Create entity with a field containing a space
    vault.write_file(
        "1_Projects/tasks/task-custom.md",
        "---\nid: task-custom\ntype: task\ntitle: Custom Field Task\nstatus: open\nDue By: 2026-04-15\ntags: []\n---\nBody.\n",
    );
    cortx_cmd(&vault)
        .args(["query", r#""Due By" = "2026-04-15""#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Custom Field Task"));
}

// -- Sort tests --

#[test]
fn test_query_sort_single_field_asc() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Task A",
            "--id",
            "task-sa1",
            "--set",
            "due=2026-04-10",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Task B",
            "--id",
            "task-sa2",
            "--set",
            "due=2026-04-05",
        ])
        .assert()
        .success();
    // Ascending sort: Task B (2026-04-05) should appear before Task A (2026-04-10)
    let output = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "due"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    let pos_b = output_str
        .find("Task B")
        .expect("Task B should be in output");
    let pos_a = output_str
        .find("Task A")
        .expect("Task A should be in output");
    assert!(
        pos_b < pos_a,
        "Task B should appear before Task A in ascending sort"
    );
}

#[test]
fn test_query_sort_single_field_desc() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Task A",
            "--id",
            "task-sd1",
            "--set",
            "due=2026-04-10",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Task B",
            "--id",
            "task-sd2",
            "--set",
            "due=2026-04-05",
        ])
        .assert()
        .success();
    // Descending sort: Task A (2026-04-10) should appear before Task B (2026-04-05)
    let output = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "due:desc"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    let pos_a = output_str
        .find("Task A")
        .expect("Task A should be in output");
    let pos_b = output_str
        .find("Task B")
        .expect("Task B should be in output");
    assert!(
        pos_a < pos_b,
        "Task A should appear before Task B in descending sort"
    );
}

#[test]
fn test_query_sort_multi_field() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Task 1",
            "--id",
            "task-sm1",
            "--set",
            "status=open",
            "--set",
            "priority=1",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Task 2",
            "--id",
            "task-sm2",
            "--set",
            "status=open",
            "--set",
            "priority=2",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Task 3",
            "--id",
            "task-sm3",
            "--set",
            "status=done",
            "--set",
            "priority=1",
        ])
        .assert()
        .success();
    // Sort by status:asc, priority:desc
    // Alphabetically: 'done' < 'open', so Task 3 comes first
    // Within 'open': priority 2 > 1, so Task 2 before Task 1
    // Expected order: Task 3 (done,p1), Task 2 (open,p2), Task 1 (open,p1)
    let output = cortx_cmd(&vault)
        .args([
            "query",
            r#"type = "task""#,
            "--sort-by",
            "status:asc,priority:desc",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    let pos_2 = output_str
        .find("Task 2")
        .expect("Task 2 should be in output");
    let pos_1 = output_str
        .find("Task 1")
        .expect("Task 1 should be in output");
    let pos_3 = output_str
        .find("Task 3")
        .expect("Task 3 should be in output");
    assert!(
        pos_3 < pos_2,
        "Task 3 (done) should appear before Task 2 (open)"
    );
    assert!(
        pos_2 < pos_1,
        "Task 2 (priority 2) should appear before Task 1 (priority 1)"
    );
}

#[test]
fn test_query_sort_nulls_to_end() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "No Due", "--id", "task-sn1"])
        .assert()
        .success(); // no due date
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Has Due",
            "--id",
            "task-sn2",
            "--set",
            "due=2026-04-05",
        ])
        .assert()
        .success();
    // Ascending sort: Has Due should appear before No Due
    let output = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "due"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    let pos_has = output_str
        .find("Has Due")
        .expect("Has Due should be in output");
    let pos_no = output_str
        .find("No Due")
        .expect("No Due should be in output");
    assert!(
        pos_has < pos_no,
        "Has Due should appear before No Due in ascending sort"
    );

    // Descending sort: nulls should still be at end
    let output_desc = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "due:desc"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_desc_str = String::from_utf8(output_desc).unwrap();
    let pos_has_desc = output_desc_str
        .find("Has Due")
        .expect("Has Due should be in output");
    let pos_no_desc = output_desc_str
        .find("No Due")
        .expect("No Due should be in output");
    assert!(
        pos_has_desc < pos_no_desc,
        "Has Due should appear before No Due in descending sort"
    );
}

#[test]
fn test_query_sort_empty_field_rejected() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Test", "--id", "task-sf1"])
        .assert()
        .success();
    // Empty sort specification
    cortx_cmd(&vault)
        .args(["query", "type = \"task\"", "--sort-by", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty sort specification"));
    // Empty field name (just order without field)
    cortx_cmd(&vault)
        .args(["query", "type = \"task\"", "--sort-by", ":asc"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty field name"));
}

#[test]
fn test_query_sort_number_values() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Priority 2",
            "--id",
            "task-p2",
            "--set",
            "priority=2",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Priority 1",
            "--id",
            "task-p1",
            "--set",
            "priority=1",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Priority 3",
            "--id",
            "task-p3",
            "--set",
            "priority=3",
        ])
        .assert()
        .success();
    // Ascending sort by number should show 1, 2, 3
    let output = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "priority"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    let pos_p1 = output_str
        .find("Priority 1")
        .expect("Priority 1 should be in output");
    let pos_p2 = output_str
        .find("Priority 2")
        .expect("Priority 2 should be in output");
    let pos_p3 = output_str
        .find("Priority 3")
        .expect("Priority 3 should be in output");
    assert!(
        pos_p1 < pos_p2,
        "Priority 1 should appear before Priority 2"
    );
    assert!(
        pos_p2 < pos_p3,
        "Priority 2 should appear before Priority 3"
    );

    // Descending sort should show 3, 2, 1
    let output_desc = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "priority:desc"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_desc_str = String::from_utf8(output_desc).unwrap();
    let pos_p1_desc = output_desc_str
        .find("Priority 1")
        .expect("Priority 1 should be in output");
    let pos_p2_desc = output_desc_str
        .find("Priority 2")
        .expect("Priority 2 should be in output");
    let pos_p3_desc = output_desc_str
        .find("Priority 3")
        .expect("Priority 3 should be in output");
    assert!(
        pos_p3_desc < pos_p2_desc,
        "Priority 3 should appear before Priority 2 in desc"
    );
    assert!(
        pos_p2_desc < pos_p1_desc,
        "Priority 2 should appear before Priority 1 in desc"
    );
}

#[test]
fn test_query_sort_multiple_nulls() {
    let vault = TestVault::new();
    // Create multiple entities with null values
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Null 1", "--id", "task-n1"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args(["create", "task", "--title", "Null 2", "--id", "task-n2"])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Has Value",
            "--id",
            "task-hv",
            "--set",
            "due=2026-04-05",
        ])
        .assert()
        .success();
    // Sort by due - Has Value should be first, both nulls at end
    let output = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "due"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    let pos_hv = output_str
        .find("Has Value")
        .expect("Has Value should be in output");
    let pos_n1 = output_str
        .find("Null 1")
        .expect("Null 1 should be in output");
    let pos_n2 = output_str
        .find("Null 2")
        .expect("Null 2 should be in output");
    assert!(pos_hv < pos_n1, "Has Value should appear before nulls");
    assert!(pos_hv < pos_n2, "Has Value should appear before nulls");
}

#[test]
fn test_query_sort_string_values() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Charlie",
            "--id",
            "task-charlie",
            "--set",
            "status=open",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Alice",
            "--id",
            "task-alice",
            "--set",
            "status=open",
        ])
        .assert()
        .success();
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Bob",
            "--id",
            "task-bob",
            "--set",
            "status=open",
        ])
        .assert()
        .success();
    // Ascending sort by title: Alice, Bob, Charlie
    let output = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "title"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    let pos_alice = output_str.find("Alice").expect("Alice should be in output");
    let pos_bob = output_str.find("Bob").expect("Bob should be in output");
    let pos_charlie = output_str
        .find("Charlie")
        .expect("Charlie should be in output");
    assert!(pos_alice < pos_bob, "Alice should appear before Bob");
    assert!(pos_bob < pos_charlie, "Bob should appear before Charlie");

    // Descending sort by title: Charlie, Bob, Alice
    let output_desc = cortx_cmd(&vault)
        .args(["query", r#"type = "task""#, "--sort-by", "title:desc"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_desc_str = String::from_utf8(output_desc).unwrap();
    let pos_alice_desc = output_desc_str
        .find("Alice")
        .expect("Alice should be in output");
    let pos_bob_desc = output_desc_str
        .find("Bob")
        .expect("Bob should be in output");
    let pos_charlie_desc = output_desc_str
        .find("Charlie")
        .expect("Charlie should be in output");
    assert!(
        pos_charlie_desc < pos_bob_desc,
        "Charlie should appear before Bob in desc"
    );
    assert!(
        pos_bob_desc < pos_alice_desc,
        "Bob should appear before Alice in desc"
    );
}

#[test]
fn test_create_with_date_keywords_resolves_to_date() {
    let vault = TestVault::new();
    let today = chrono::Local::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let yesterday = today - chrono::Duration::days(1);

    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Tomorrow task",
            "--id",
            "task-tomorrow",
            "--set",
            "due=tomorrow",
        ])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/task-tomorrow.md");
    assert!(
        content.contains(&format!("due: {}", tomorrow)),
        "expected due to be resolved to {tomorrow}, got:\n{content}"
    );

    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Today task",
            "--id",
            "task-today",
            "--set",
            "due=today",
        ])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/task-today.md");
    assert!(
        content.contains(&format!("due: {}", today)),
        "expected due to be resolved to {today}, got:\n{content}"
    );

    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Yesterday task",
            "--id",
            "task-yesterday",
            "--set",
            "due=yesterday",
        ])
        .assert()
        .success();
    let content = vault.read_file("1_Projects/tasks/task-yesterday.md");
    assert!(
        content.contains(&format!("due: {}", yesterday)),
        "expected due to be resolved to {yesterday}, got:\n{content}"
    );
}

#[test]
fn test_init_with_name_registers_in_global_config() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    // Use a temp HOME so we don't pollute the real ~/.cortx/config.toml
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args([
            "init",
            vault_dir.path().to_str().unwrap(),
            "--name",
            "testonly",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered vault 'testonly'"));
    // Config file must exist
    assert!(dir.path().join(".cortx").join("config.toml").exists());
    let config_content = fs::read_to_string(dir.path().join(".cortx").join("config.toml")).unwrap();
    assert!(config_content.contains("[vaults.testonly]"));
}

#[test]
fn test_init_first_named_vault_becomes_default() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args([
            "init",
            vault_dir.path().to_str().unwrap(),
            "--name",
            "myvault",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Set 'myvault' as the default vault.",
        ));
    let config_content = fs::read_to_string(dir.path().join(".cortx").join("config.toml")).unwrap();
    assert!(config_content.contains("default = \"myvault\""));
}

#[test]
fn test_init_duplicate_name_errors() {
    let dir = TempDir::new().unwrap();
    let vault1 = TempDir::new().unwrap();
    let vault2 = TempDir::new().unwrap();
    // First registration succeeds
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args(["init", vault1.path().to_str().unwrap(), "--name", "shared"])
        .assert()
        .success();
    // Second registration with same name fails
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args(["init", vault2.path().to_str().unwrap(), "--name", "shared"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "vault name 'shared' is already registered",
        ));
}

#[test]
fn test_vault_name_flag_resolves_correct_vault() {
    let home_dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    // Init and register a named vault
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", home_dir.path())
        .args([
            "init",
            vault_dir.path().to_str().unwrap(),
            "--name",
            "mywork",
        ])
        .assert()
        .success();
    // Create an entity using --vault-name
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", home_dir.path())
        .args([
            "--vault-name",
            "mywork",
            "create",
            "task",
            "--title",
            "Named vault task",
            "--id",
            "task-named-vault",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task-named-vault"));
    // Verify the file exists in the named vault
    assert!(
        vault_dir
            .path()
            .join("1_Projects/tasks/task-named-vault.md")
            .exists()
    );
}

#[test]
fn test_vault_name_unknown_errors() {
    let home_dir = TempDir::new().unwrap();
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", home_dir.path())
        .args(["--vault-name", "ghost", "query", "type = \"task\""])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "vault 'ghost' not found in global config",
        ));
}

#[test]
fn test_init_without_name_skips_global_config() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args(["init", vault_dir.path().to_str().unwrap()])
        .assert()
        .success();
    // No config file should be created when --name is not provided
    assert!(!dir.path().join(".cortx").join("config.toml").exists());
}

#[test]
fn test_schema_types_text() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["schema", "types"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Types ("))
        .stdout(predicate::str::contains("task"))
        .stdout(predicate::str::contains("project"))
        .stdout(predicate::str::contains("folder:"));
}

#[test]
fn test_schema_types_json() {
    let vault = TestVault::new();
    let output = cortx_cmd(&vault)
        .args(["schema", "types", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(parsed.is_array());
    let names: Vec<&str> = parsed.as_array().unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(names.contains(&"task"));
    assert!(names.contains(&"project"));
    // Should be sorted
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[test]
fn test_schema_show_text() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["schema", "show", "task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Type:   task"))
        .stdout(predicate::str::contains("Folder:"))
        .stdout(predicate::str::contains("FIELD"))
        .stdout(predicate::str::contains("TYPE"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("enum["));
}

#[test]
fn test_schema_show_json() {
    let vault = TestVault::new();
    let output = cortx_cmd(&vault)
        .args(["schema", "show", "task", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["name"], "task");
    assert!(parsed["folder"].is_string());
    assert!(parsed["fields"].is_object());
    assert_eq!(parsed["fields"]["status"]["type"], "enum");
    assert!(parsed["fields"]["status"]["values"].is_array());
    assert_eq!(parsed["fields"]["id"]["required"], true);
}

#[test]
fn test_schema_show_unknown_type() {
    let vault = TestVault::new();
    cortx_cmd(&vault)
        .args(["schema", "show", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown type 'nonexistent'"));
}

#[test]
fn test_schema_types_custom_vault() {
    // A vault with a custom types.yaml shows only its own types
    let tmp = TempDir::new().unwrap();
    let vault_path = tmp.path().join("custom");
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", vault_path.to_str().unwrap()])
        .assert()
        .success();

    let custom_types = r#"types:
  recipe:
    folder: "6_Recipes"
    required: [id, type, title]
    fields:
      id:    { type: string }
      type:  { const: recipe }
      title: { type: string }
"#;
    fs::write(vault_path.join("types.yaml"), custom_types).unwrap();

    let output = Command::cargo_bin("cortx")
        .unwrap()
        .args(["--vault", vault_path.to_str().unwrap(), "schema", "types", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let names: Vec<&str> = parsed.as_array().unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(names, vec!["recipe"]);
}
