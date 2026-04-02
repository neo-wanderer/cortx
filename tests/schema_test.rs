use cortx::schema::registry::TypeRegistry;
use cortx::schema::types::{FieldType, TypeDefinition};

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
