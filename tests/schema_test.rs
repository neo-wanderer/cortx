use cortx::schema::registry::TypeRegistry;
use cortx::schema::types::FieldType;
use cortx::schema::validation::validate_frontmatter;
use cortx::value::Value;
use std::collections::HashMap;

#[test]
fn test_load_types_yaml() {
    let yaml = r#"
types:
  task:
    folder: "1_Projects/tasks"
    required: [type, title, status]
    fields:
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
    required: [type, title, status]
    fields:
      type:     { const: task }
      title:    { type: string }
      status:   { enum: [open, in_progress, done] }
      due:      { type: date }
      tags:     { type: "array[string]", default: "[]" }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert_eq!(task_def.folder, "tasks");
    assert!(task_def.required.contains(&"type".to_string()));
    assert_eq!(
        task_def.fields["status"].field_type,
        FieldType::Enum(vec!["open".into(), "in_progress".into(), "done".into()])
    );
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
    assert_eq!(
        task_def.fields["type"].field_type,
        FieldType::Const("task".into())
    );
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
    let registry = TypeRegistry::from_yaml_file(std::path::Path::new("types.yaml")).unwrap();
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
    required: [type, title, status]
    fields:
      type:   { const: task }
      title:  { type: string }
      status: { enum: [open, in_progress, done] }
      due:    { type: date }
      tags:   { type: "array[string]", default: "[]" }
      active: { type: bool }
      priority: { type: number }
"#;
    TypeRegistry::from_yaml_str(yaml).unwrap()
}

#[test]
fn test_validate_valid_frontmatter() {
    let registry = make_registry();
    let mut fm = HashMap::new();
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
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("kinda_done".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("kinda_done"),
        "error should mention bad value: {err}"
    );
}

#[test]
fn test_validate_wrong_const() {
    let registry = make_registry();
    let mut fm = HashMap::new();
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
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert(
        "due".into(),
        Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 4, 5).unwrap()),
    );
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_date_field_rejects_bad_string() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("due".into(), Value::String("not-a-date".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a date"));
}

#[test]
fn test_validate_date_field_rejects_non_string_non_date() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("due".into(), Value::Bool(true));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a date"));
}

#[test]
fn test_validate_array_field_rejects_non_array() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("tags".into(), Value::String("not-an-array".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be an array"));
}

#[test]
fn test_validate_array_field_accepts_null() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("tags".into(), Value::Null);
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_bool_field_rejects_non_bool() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("active".into(), Value::String("yes".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("must be a boolean"));
}

#[test]
fn test_validate_number_field_rejects_non_number() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("priority".into(), Value::String("high".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a number"));
}

#[test]
fn test_validate_multiple_errors() {
    let registry = make_registry();
    let fm = HashMap::new(); // missing all required fields
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("type"));
    assert!(err.contains("title"));
    assert!(err.contains("status"));
}

#[test]
fn test_validate_unknown_fields_allowed() {
    let registry = make_registry();
    let mut fm = HashMap::new();
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Do thing".into()));
    fm.insert("status".into(), Value::String("open".into()));
    fm.insert("custom_field".into(), Value::String("anything".into()));
    let result = validate_frontmatter(&fm, registry.get("task").unwrap());
    assert!(result.is_ok());
}

// -- Registry edge cases --

#[test]
fn test_registry_missing_types_key() {
    let yaml = r#"
something_else:
  task:
    folder: "tasks"
"#;
    let err = TypeRegistry::from_yaml_str(yaml).unwrap_err();
    assert!(err.to_string().contains("missing 'types'"));
}

#[test]
fn test_registry_unknown_field_type() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: []
    fields:
      data: { type: "binary" }
"#;
    let err = TypeRegistry::from_yaml_str(yaml).unwrap_err();
    assert!(err.to_string().contains("unknown field type"));
}

#[test]
fn test_registry_link_field_type() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: []
    fields:
      owner: { type: link, ref: person }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert!(matches!(
        task_def.fields["owner"].field_type,
        FieldType::Link(_)
    ));
}

#[test]
fn test_registry_datetime_field_type() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: []
    fields:
      created: { type: datetime }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert_eq!(task_def.fields["created"].field_type, FieldType::Datetime);
}

#[test]
fn test_registry_bool_field_type() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: []
    fields:
      active: { type: bool }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert_eq!(task_def.fields["active"].field_type, FieldType::Bool);
}

#[test]
fn test_registry_number_field_type() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: []
    fields:
      priority: { type: number }
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert_eq!(task_def.fields["priority"].field_type, FieldType::Number);
}

#[test]
fn test_registry_default_field_type_without_type_key() {
    let yaml = r#"
types:
  task:
    folder: "tasks"
    required: []
    fields:
      notes: {}
"#;
    let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    let task_def = registry.get("task").unwrap();
    assert_eq!(task_def.fields["notes"].field_type, FieldType::String);
}

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
    let FieldType::Link(link_def) = area_field else {
        panic!("expected Link, got {area_field:?}")
    };
    assert!(!link_def.bidirectional);
    let LinkTargets::Single { ref_type, inverse } = &link_def.targets else {
        panic!("expected Single")
    };
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
    let FieldType::Link(link_def) = &task_def.fields["goal"].field_type else {
        panic!()
    };
    assert!(link_def.bidirectional);
    let LinkTargets::Single { ref_type, inverse } = &link_def.targets else {
        panic!()
    };
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
    let FieldType::ArrayLink(link_def) = &note_def.fields["related_goals"].field_type else {
        panic!()
    };
    assert!(link_def.bidirectional);
    let LinkTargets::Single { ref_type, inverse } = &link_def.targets else {
        panic!()
    };
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
    let FieldType::Link(link_def) = &note_def.fields["related"].field_type else {
        panic!()
    };
    assert!(link_def.bidirectional);
    let LinkTargets::Poly(targets) = &link_def.targets else {
        panic!()
    };
    assert_eq!(targets.len(), 2);
    let goal_target = targets.iter().find(|t| t.ref_type == "goal").unwrap();
    assert_eq!(goal_target.inverse.as_deref(), Some("related_notes"));
}

#[test]
fn test_link_polymorphic_unidirectional_sequence() {
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
    let FieldType::Link(link_def) = &log_def.fields["subject"].field_type else {
        panic!()
    };
    assert!(!link_def.bidirectional);
    let LinkTargets::Poly(targets) = &link_def.targets else {
        panic!()
    };
    assert_eq!(targets.len(), 3);
    assert!(targets.iter().any(|t| t.ref_type == "goal"));
    assert!(targets.iter().any(|t| t.ref_type == "task"));
    assert!(targets.iter().any(|t| t.ref_type == "note"));
}
