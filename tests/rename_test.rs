mod common;

use assert_cmd::Command;
use common::TestVault;
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
fn rename_updates_filename_and_back_refs() {
    let vault = TestVault::new();

    // Create a project (area is required for project, use a trick — create area first)
    // But production schema may not require area. Let's try without.
    cortx_cmd(&vault)
        .args([
            "create",
            "project",
            "--title",
            "Website Redesign",
            "--set",
            "status=active",
        ])
        .assert()
        .success();

    // Create a task linked to it
    cortx_cmd(&vault)
        .args([
            "create",
            "task",
            "--title",
            "Buy Groceries",
            "--set",
            "project=Website Redesign",
        ])
        .assert()
        .success();

    // Rename the project
    cortx_cmd(&vault)
        .args(["rename", "Website Redesign", "Brand Refresh"])
        .assert()
        .success();

    // Old file gone, new file present
    assert!(!vault.file_exists("1_Projects/Website Redesign.md"));
    assert!(vault.file_exists("1_Projects/Brand Refresh.md"));

    // Task's back-ref updated
    let task_content = vault.read_file("1_Projects/tasks/Buy Groceries.md");
    assert!(
        task_content.contains("[[Brand Refresh]]"),
        "got: {task_content}"
    );
    assert!(!task_content.contains("[[Website Redesign]]"));

    // Renamed file's own title field updated
    let project_content = vault.read_file("1_Projects/Brand Refresh.md");
    assert!(
        project_content.contains("title: Brand Refresh"),
        "got: {project_content}"
    );
}

#[test]
fn rename_rewrites_body_wikilinks() {
    let vault = TestVault::new();

    cortx_cmd(&vault)
        .args([
            "create",
            "project",
            "--title",
            "Website Redesign",
            "--set",
            "status=active",
        ])
        .assert()
        .success();

    // Append a body wikilink to the project file itself
    let original = vault.read_file("1_Projects/Website Redesign.md");
    fs::write(
        vault.path().join("1_Projects/Website Redesign.md"),
        format!("{original}\n\nSee [[Website Redesign]] for context.\n"),
    )
    .unwrap();

    cortx_cmd(&vault)
        .args(["rename", "Website Redesign", "Brand Refresh"])
        .assert()
        .success();

    let content = vault.read_file("1_Projects/Brand Refresh.md");
    assert!(
        content.contains("See [[Brand Refresh]] for context."),
        "body not rewritten: {content}"
    );
}

#[test]
fn rename_skip_body_preserves_body_wikilinks() {
    let vault = TestVault::new();

    cortx_cmd(&vault)
        .args([
            "create",
            "project",
            "--title",
            "Website Redesign",
            "--set",
            "status=active",
        ])
        .assert()
        .success();

    let original = vault.read_file("1_Projects/Website Redesign.md");
    fs::write(
        vault.path().join("1_Projects/Website Redesign.md"),
        format!("{original}\n\nSee [[Website Redesign]] for context.\n"),
    )
    .unwrap();

    cortx_cmd(&vault)
        .args(["rename", "Website Redesign", "Brand Refresh", "--skip-body"])
        .assert()
        .success();

    let content = vault.read_file("1_Projects/Brand Refresh.md");
    assert!(
        content.contains("See [[Website Redesign]] for context."),
        "body should be preserved: {content}"
    );
}

#[test]
fn rename_dry_run_writes_nothing() {
    let vault = TestVault::new();

    cortx_cmd(&vault)
        .args([
            "create",
            "project",
            "--title",
            "Website Redesign",
            "--set",
            "status=active",
        ])
        .assert()
        .success();

    let before = vault.read_file("1_Projects/Website Redesign.md");

    cortx_cmd(&vault)
        .args(["rename", "Website Redesign", "Brand Refresh", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));

    assert!(vault.file_exists("1_Projects/Website Redesign.md"));
    assert!(!vault.file_exists("1_Projects/Brand Refresh.md"));
    assert_eq!(vault.read_file("1_Projects/Website Redesign.md"), before);
}

#[test]
fn rename_rejects_collision() {
    let vault = TestVault::new();

    cortx_cmd(&vault)
        .args([
            "create",
            "project",
            "--title",
            "Website Redesign",
            "--set",
            "status=active",
        ])
        .assert()
        .success();

    cortx_cmd(&vault)
        .args([
            "create",
            "project",
            "--title",
            "Brand Refresh",
            "--set",
            "status=active",
        ])
        .assert()
        .success();

    cortx_cmd(&vault)
        .args(["rename", "Website Redesign", "Brand Refresh"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("collides"));
}
