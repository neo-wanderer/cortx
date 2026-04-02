use chrono::NaiveDate;
use cortx::frontmatter::{parse_frontmatter, serialize_entity};
use cortx::value::Value;

#[test]
fn test_parse_frontmatter_basic() {
    let content = r#"---
id: task-001
type: task
title: "Do the thing"
status: open
tags:
  - home
  - urgent
---
# Body content

Some notes here.
"#;
    let (fm, body) = parse_frontmatter(content).unwrap();
    assert_eq!(fm.get("id").unwrap(), &Value::String("task-001".into()));
    assert_eq!(fm.get("type").unwrap(), &Value::String("task".into()));
    assert_eq!(
        fm.get("title").unwrap(),
        &Value::String("Do the thing".into())
    );
    assert_eq!(fm.get("status").unwrap(), &Value::String("open".into()));
    assert_eq!(
        fm.get("tags").unwrap(),
        &Value::Array(vec![
            Value::String("home".into()),
            Value::String("urgent".into()),
        ])
    );
    assert!(body.contains("# Body content"));
    assert!(body.contains("Some notes here."));
}

#[test]
fn test_parse_frontmatter_with_date() {
    let content = "---\ndue: 2026-04-05\n---\nBody\n";
    let (fm, _body) = parse_frontmatter(content).unwrap();
    assert_eq!(
        fm.get("due").unwrap(),
        &Value::Date(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap())
    );
}

#[test]
fn test_parse_frontmatter_no_frontmatter() {
    let content = "# Just a plain markdown file\n\nNo frontmatter here.\n";
    let result = parse_frontmatter(content);
    assert!(result.is_err());
}

#[test]
fn test_serialize_entity_roundtrip() {
    let mut fm = std::collections::HashMap::new();
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do the thing".into()));
    fm.insert("status".into(), Value::String("open".into()));

    let body = "# Notes\n\nSome content.\n";
    let output = serialize_entity(&fm, body);

    assert!(output.starts_with("---\n"));
    assert!(output.contains("id: task-001"));
    assert!(output.contains("type: task"));
    assert!(output.ends_with("# Notes\n\nSome content.\n"));

    // Roundtrip
    let (parsed_fm, parsed_body) = parse_frontmatter(&output).unwrap();
    assert_eq!(
        parsed_fm.get("id").unwrap(),
        &Value::String("task-001".into())
    );
    assert_eq!(parsed_body.trim(), body.trim());
}

#[test]
fn test_parse_frontmatter_unclosed() {
    let content = "---\nid: task-001\ntype: task\n";
    let err = parse_frontmatter(content).unwrap_err();
    assert!(err.to_string().contains("unclosed"));
}

#[test]
fn test_parse_frontmatter_not_a_mapping() {
    // YAML that parses as a scalar, not a mapping
    let content = "---\njust a string\n---\nBody\n";
    let err = parse_frontmatter(content).unwrap_err();
    assert!(err.to_string().contains("mapping"));
}
