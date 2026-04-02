use cortx::schema::registry::TypeRegistry;
use cortx::schema::types::{FieldType, TypeDefinition};
use cortx::schema::validation::validate_frontmatter;
use std::collections::HashMap;
use cortx::value::Value;

#[test]
fn test_load_types_yaml() {
    let yaml = r#"
types:
  task:
    folder: "1_Projects/tasks"
    required: [id, type, title, status]
    fields:
      id:       { type: string }
      type:     { const: task }
      title:    { type: string }
      status:   { enum: [open, in_progress, waiting, done, cancelled] }
      due:      { type: date }
      tags:     { type: "array[string]", default: "[]" }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    assert!(registry.get("task").is_some());
    assert!(registry.get("unknown").is_none());
}

#[test]
fn test_type_definition_fields() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: [id, type, title, status]
    fields:
      id:       { type: string }
      type:     { const: task }
      title:    { type: string }
      status:   { enum: [open, in_progress, done] }
      due:      { type: date }
      tags:     { type: "array[string]", default: "[]" }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert_eq!(task_def.folder, "tasks");
    assert!(task_def.required.contains(&"id".to_string()));
    assert_eq!(task_def.fields["status"].field_type, FieldType::Enum(vec![
        "open".into(), "in_progress".into(), "done".into()
    ]));
    assert_eq!(task_def.fields["due"].field_type, FieldType::Date);
    assert_eq!(task_def.fields["tags"].field_type, FieldType::ArrayString);
}

#[test]
fn test_type_definition_const_field() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: [type]
    fields:
      type: { const: task }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert_eq!(task_def.fields["type"].field_type, FieldType::Const("task".into()));
}

#[test]
fn test_registry_type_names() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: []
    fields: {}
  person:
    folder: "people"
    required: []
    fields: {}
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let mut names = registry.type_names();
    names.sort();
    assert_eq!(names, vec!["person", "task"]);
}

#[test]
fn test_load_real_types_yaml() {
    let registry = TypeRegistry::from_yaml_file(
        std::path::Path::new("types.yaml")
    ).unwrap();
    assert!(registry.get("task").is_some());
    assert!(registry.get("project").is_some());
    assert!(registry.get("person").is_some());
    assert!(registry.get("company").is_some());
    assert!(registry.get("note").is_some());
    assert!(registry.get("area").is_some());
    assert!(registry.get("resource").is_some());
}

fn make_registry() -> TypeRegistry {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: [id, type, title, status]
    fields:
      id:     { type: string }
      type:   { const: task }
      title:  { type: string }
      status: { enum: [open, in_progress, done] }
      due:    { type: date }
      tags:   { type: "array[string]", default: "[]" }
"#;
    TypeRegistry::from_yaml_str(yaml).unwrap()
}

#[test]
fn test_validate_valid_frontmatter() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_missing_required_field() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("task".into()));
    // missing title and status
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("title"), "error should mention 'title': {err}");
}

#[test]
fn test_validate_invalid_enum_value() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("kinda_done".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("kinda_done"), "error should mention bad value: {err}");
}

#[test]
fn test_validate_wrong_const() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("project".into())); // wrong const
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
}

#[test]
fn test_validate_date_field_accepts_date() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("due".into(), Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 4, 5).unwrap()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_ok());
}
