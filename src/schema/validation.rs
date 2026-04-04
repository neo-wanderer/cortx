use super::types::{FieldType, TypeDefinition};
use crate::error::{CortxError, Result};
use crate::value::Value;
use std::collections::HashMap;

/// Validate a frontmatter HashMap against a TypeDefinition schema.
///
/// Checks required fields, enum values, const fields, date formats,
/// and array types.
///
/// # Examples
///
/// ```
/// use cortx::schema::registry::TypeRegistry;
/// use cortx::schema::validation::validate_frontmatter;
/// use cortx::value::Value;
/// use std::collections::HashMap;
///
/// let yaml = r#"
/// types:
///   task:
///     folder: "tasks"
///     required: [id, type, status]
///     fields:
///       id:     { type: string }
///       type:   { const: task }
///       status: { enum: [open, done] }
/// "#;
/// let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
/// let type_def = registry.get("task").unwrap();
///
/// let mut fm = HashMap::new();
/// fm.insert("id".into(), Value::String("t1".into()));
/// fm.insert("type".into(), Value::String("task".into()));
/// fm.insert("status".into(), Value::String("open".into()));
/// assert!(validate_frontmatter(&fm, type_def).is_ok());
///
/// fm.insert("status".into(), Value::String("invalid".into()));
/// assert!(validate_frontmatter(&fm, type_def).is_err());
/// ```
pub fn validate_frontmatter(
    frontmatter: &HashMap<String, Value>,
    type_def: &TypeDefinition,
) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    // Check required fields
    for field_name in &type_def.required {
        if !frontmatter.contains_key(field_name) {
            errors.push(format!("missing required field '{field_name}'"));
        }
    }

    // Check field values against schema
    for (field_name, value) in frontmatter {
        if let Some(field_def) = type_def.fields.get(field_name) {
            match &field_def.field_type {
                FieldType::Const(expected) => {
                    if let Value::String(s) = value
                        && s != expected
                    {
                        errors.push(format!(
                            "field '{field_name}' must be '{expected}', got '{s}'"
                        ));
                    }
                }
                FieldType::Enum(variants) => {
                    if let Value::String(s) = value
                        && !variants.contains(s)
                    {
                        errors.push(format!(
                            "field '{field_name}' must be one of [{}], got '{s}'",
                            variants.join(", ")
                        ));
                    }
                }
                FieldType::Date => match value {
                    Value::Date(_) => {}
                    Value::String(s) => {
                        if Value::parse_as_date(s).is_none() {
                            errors.push(format!(
                                "field '{field_name}' must be a date (YYYY-MM-DD), got '{s}'"
                            ));
                        }
                    }
                    _ => {
                        errors.push(format!("field '{field_name}' must be a date"));
                    }
                },
                FieldType::ArrayString => {
                    if !matches!(value, Value::Array(_) | Value::Null) {
                        errors.push(format!("field '{field_name}' must be an array of strings"));
                    }
                }
                FieldType::Bool => {
                    if !matches!(value, Value::Bool(_)) {
                        errors.push(format!("field '{field_name}' must be a boolean"));
                    }
                }
                FieldType::Number => {
                    if !matches!(value, Value::Number(_)) {
                        errors.push(format!("field '{field_name}' must be a number"));
                    }
                }
                FieldType::String | FieldType::Link(_) | FieldType::ArrayLink(_) | FieldType::Datetime => {
                    // String-like fields; link refs are string IDs — no value-level validation here
                }
            }
        }
        // Unknown fields are allowed (forward compatibility)
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(CortxError::Validation(errors.join("; ")))
    }
}
