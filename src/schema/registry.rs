use super::types::{FieldDefinition, FieldType, TypeDefinition};
use crate::error::{CortxError, Result};
use std::collections::HashMap;

#[derive(Debug)]
pub struct TypeRegistry {
    types: HashMap<String, TypeDefinition>,
}

impl TypeRegistry {
    /// Load a TypeRegistry from a YAML string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortx::schema::registry::TypeRegistry;
    ///
    /// let yaml = r#"
    /// types:
    ///   task:
    ///     folder: "tasks"
    ///     required: [id, type]
    ///     fields:
    ///       id:   { type: string }
    ///       type: { const: task }
    /// "#;
    /// let registry = TypeRegistry::from_yaml_str(yaml).unwrap();
    /// assert!(registry.get("task").is_some());
    /// assert!(registry.get("unknown").is_none());
    /// ```
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let root: serde_yaml::Value = serde_yaml::from_str(yaml)?;
        let types_map = root
            .get("types")
            .and_then(|v| v.as_mapping())
            .ok_or_else(|| CortxError::Schema("missing 'types' key in config".into()))?;

        let mut types = HashMap::new();

        for (type_name_val, type_def_val) in types_map {
            let type_name = type_name_val
                .as_str()
                .ok_or_else(|| CortxError::Schema("type name must be a string".into()))?
                .to_string();

            let folder = type_def_val
                .get("folder")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let required: Vec<String> = type_def_val
                .get("required")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let fields_map = type_def_val
                .get("fields")
                .and_then(|v| v.as_mapping())
                .cloned()
                .unwrap_or_default();

            let mut fields = HashMap::new();
            for (field_name_val, field_def_val) in &fields_map {
                let field_name = field_name_val
                    .as_str()
                    .ok_or_else(|| CortxError::Schema("field name must be a string".into()))?
                    .to_string();

                let field_def = Self::parse_field_def(field_def_val, &required, &field_name)?;
                fields.insert(field_name, field_def);
            }

            types.insert(
                type_name.clone(),
                TypeDefinition {
                    name: type_name,
                    folder,
                    required,
                    fields,
                },
            );
        }

        Ok(TypeRegistry { types })
    }

    pub fn from_yaml_file(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&content)
    }

    pub fn get(&self, type_name: &str) -> Option<&TypeDefinition> {
        self.types.get(type_name)
    }

    pub fn type_names(&self) -> Vec<&str> {
        self.types.keys().map(|s| s.as_str()).collect()
    }

    fn parse_field_def(
        val: &serde_yaml::Value,
        required_fields: &[String],
        field_name: &str,
    ) -> Result<FieldDefinition> {
        let field_type = if let Some(const_val) = val.get("const").and_then(|v| v.as_str()) {
            FieldType::Const(const_val.to_string())
        } else if let Some(enum_seq) = val.get("enum").and_then(|v| v.as_sequence()) {
            let variants: Vec<String> = enum_seq
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            FieldType::Enum(variants)
        } else if let Some(type_str) = val.get("type").and_then(|v| v.as_str()) {
            match type_str {
                "string" => FieldType::String,
                "date" => FieldType::Date,
                "datetime" => FieldType::Datetime,
                "bool" => FieldType::Bool,
                "number" => FieldType::Number,
                "array[string]" => FieldType::ArrayString,
                t if t.starts_with("link") => {
                    let ref_type = val.get("ref").and_then(|v| v.as_str()).map(String::from);
                    FieldType::Link { ref_type }
                }
                other => {
                    return Err(CortxError::Schema(format!(
                        "unknown field type '{other}' for field '{field_name}'"
                    )));
                }
            }
        } else {
            FieldType::String
        };

        let is_required = required_fields.contains(&field_name.to_string());
        let is_optional = val
            .get("optional")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let default = val
            .get("default")
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(FieldDefinition {
            field_type,
            required: is_required && !is_optional,
            default,
        })
    }
}
