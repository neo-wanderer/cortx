use std::collections::HashMap;
use crate::error::{CortxError, Result};
use crate::value::Value;
use super::types::{FieldType, TypeDefinition};

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
                    if let Value::String(s) = value {
                        if s != expected {
                            errors.push(format!(
                                "field '{field_name}' must be '{expected}', got '{s}'"
                            ));
                        }
                    }
                }
                FieldType::Enum(variants) => {
                    if let Value::String(s) = value {
                        if !variants.contains(s) {
                            errors.push(format!(
                                "field '{field_name}' must be one of [{}], got '{s}'",
                                variants.join(", ")
                            ));
                        }
                    }
                }
                FieldType::Date => {
                    match value {
                        Value::Date(_) => {}
                        Value::String(s) => {
                            if Value::parse_as_date(s).is_none() {
                                errors.push(format!(
                                    "field '{field_name}' must be a date (YYYY-MM-DD), got '{s}'"
                                ));
                            }
                        }
                        _ => {
                            errors.push(format!(
                                "field '{field_name}' must be a date"
                            ));
                        }
                    }
                }
                FieldType::ArrayString => {
                    if !matches!(value, Value::Array(_) | Value::Null) {
                        errors.push(format!(
                            "field '{field_name}' must be an array of strings"
                        ));
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
                FieldType::String | FieldType::Link { .. } | FieldType::Datetime => {
                    // String-like fields accept any string value
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
