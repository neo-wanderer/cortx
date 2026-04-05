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

fn write_minimal_schema(vault: &TestVault) {
    fs::write(
        vault.path().join("types.yaml"),
        r#"types:
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
"#,
    )
    .unwrap();
}

#[test]
fn doctor_filenames_detects_drift() {
    let vault = TestVault::new();
    write_minimal_schema(&vault);
    // File named "Wrong Name.md" but title says "Correct Name"
    fs::create_dir_all(vault.path().join("tasks")).unwrap();
    fs::write(
        vault.path().join("tasks/Wrong Name.md"),
        "---\ntype: task\ntitle: Correct Name\n---\n",
    )
    .unwrap();

    cortx_cmd(&vault)
        .args(["doctor", "filenames"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("DRIFT"));
}

#[test]
fn doctor_filenames_fix_renames_drifted_file() {
    let vault = TestVault::new();
    write_minimal_schema(&vault);
    fs::create_dir_all(vault.path().join("tasks")).unwrap();
    fs::write(
        vault.path().join("tasks/Wrong Name.md"),
        "---\ntype: task\ntitle: Correct Name\n---\n",
    )
    .unwrap();

    cortx_cmd(&vault)
        .args(["doctor", "filenames", "--fix"])
        .assert()
        .success();

    assert!(!vault.file_exists("tasks/Wrong Name.md"));
    assert!(vault.file_exists("tasks/Correct Name.md"));
}

#[test]
fn doctor_filenames_detects_bare_link_value() {
    let vault = TestVault::new();
    write_minimal_schema(&vault);
    fs::create_dir_all(vault.path().join("projects")).unwrap();
    fs::create_dir_all(vault.path().join("tasks")).unwrap();
    fs::write(
        vault.path().join("projects/Website Redesign.md"),
        "---\ntype: project\ntitle: Website Redesign\n---\n",
    )
    .unwrap();
    fs::write(
        vault.path().join("tasks/Buy Groceries.md"),
        "---\ntype: task\ntitle: Buy Groceries\nproject: Website Redesign\n---\n",
    )
    .unwrap();

    cortx_cmd(&vault)
        .args(["doctor", "filenames"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("WIKILINK FORMAT"));
}

#[test]
fn doctor_filenames_fix_wraps_bare_link_value() {
    let vault = TestVault::new();
    write_minimal_schema(&vault);
    fs::create_dir_all(vault.path().join("projects")).unwrap();
    fs::create_dir_all(vault.path().join("tasks")).unwrap();
    fs::write(
        vault.path().join("projects/Website Redesign.md"),
        "---\ntype: project\ntitle: Website Redesign\n---\n",
    )
    .unwrap();
    fs::write(
        vault.path().join("tasks/Buy Groceries.md"),
        "---\ntype: task\ntitle: Buy Groceries\nproject: Website Redesign\n---\n",
    )
    .unwrap();

    cortx_cmd(&vault)
        .args(["doctor", "filenames", "--fix"])
        .assert()
        .success();

    let content = vault.read_file("tasks/Buy Groceries.md");
    assert!(content.contains("[[Website Redesign]]"), "got: {content}");
}
