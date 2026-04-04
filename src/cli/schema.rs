use crate::config::Config;
use crate::error::{CortxError, Result};
use crate::schema::types::FieldType;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct SchemaArgs {
    #[command(subcommand)]
    pub command: SchemaCommands,
}

#[derive(Subcommand)]
pub enum SchemaCommands {
    /// List all entity types defined in types.yaml
    Types {
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Show fields and schema for a specific type
    Show {
        type_name: String,
        #[arg(long, default_value = "text")]
        format: String,
    },
}

fn field_type_str(ft: &FieldType) -> String {
    match ft {
        FieldType::String => "string".into(),
        FieldType::Date => "date".into(),
        FieldType::Datetime => "datetime".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Number => "number".into(),
        FieldType::ArrayString => "array[string]".into(),
        FieldType::Const(v) => format!("const:{v}"),
        FieldType::Enum(variants) => format!("enum[{}]", variants.join(",")),
        FieldType::Link { ref_type: Some(r) } => format!("link:{r}"),
        FieldType::Link { ref_type: None } => "link".into(),
    }
}

pub fn run(args: &SchemaArgs, config: &Config) -> Result<()> {
    match &args.command {
        SchemaCommands::Types { format } => {
            let mut names: Vec<&str> = config.registry.type_names();
            names.sort_unstable();

            if format == "json" {
                let arr: Vec<serde_json::Value> =
                    names.iter().map(|n| serde_json::Value::String(n.to_string())).collect();
                println!("{}", serde_json::to_string_pretty(&arr).unwrap_or_default());
            } else {
                println!("Types ({}):\n", names.len());
                for name in &names {
                    let def = config.registry.get(name).unwrap();
                    println!("  {name}  (folder: {})", def.folder);
                }
            }
        }

        SchemaCommands::Show { type_name, format } => {
            let def = config
                .registry
                .get(type_name)
                .ok_or_else(|| CortxError::Schema(format!("unknown type '{type_name}'")))?;

            if format == "json" {
                let mut fields_map = serde_json::Map::new();
                let mut field_names: Vec<&str> = def.fields.keys().map(|s| s.as_str()).collect();
                field_names.sort_unstable();

                for field_name in field_names {
                    let field = &def.fields[field_name];
                    let mut obj = serde_json::Map::new();
                    let type_str = field_type_str(&field.field_type);

                    // Split "const:value" / "enum[...]" / "link:ref" into structured form
                    match &field.field_type {
                        FieldType::Const(v) => {
                            obj.insert("type".into(), serde_json::Value::String("const".into()));
                            obj.insert("value".into(), serde_json::Value::String(v.clone()));
                        }
                        FieldType::Enum(variants) => {
                            obj.insert("type".into(), serde_json::Value::String("enum".into()));
                            obj.insert(
                                "values".into(),
                                serde_json::Value::Array(
                                    variants
                                        .iter()
                                        .map(|v| serde_json::Value::String(v.clone()))
                                        .collect(),
                                ),
                            );
                        }
                        FieldType::Link { ref_type } => {
                            obj.insert("type".into(), serde_json::Value::String("link".into()));
                            if let Some(r) = ref_type {
                                obj.insert("ref".into(), serde_json::Value::String(r.clone()));
                            }
                        }
                        _ => {
                            obj.insert(
                                "type".into(),
                                serde_json::Value::String(type_str),
                            );
                        }
                    }

                    obj.insert(
                        "required".into(),
                        serde_json::Value::Bool(field.required),
                    );
                    if let Some(d) = &field.default {
                        obj.insert("default".into(), serde_json::Value::String(d.clone()));
                    }
                    fields_map.insert(field_name.to_string(), serde_json::Value::Object(obj));
                }

                let out = serde_json::json!({
                    "name": def.name,
                    "folder": def.folder,
                    "fields": fields_map,
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            } else {
                println!("Type:   {}", def.name);
                println!("Folder: {}\n", def.folder);

                let mut field_names: Vec<&str> = def.fields.keys().map(|s| s.as_str()).collect();
                field_names.sort_unstable();

                let col1 = field_names.iter().map(|n| n.len()).max().unwrap_or(5).max(5);
                let col2 = field_names
                    .iter()
                    .map(|n| field_type_str(&def.fields[*n].field_type).len())
                    .max()
                    .unwrap_or(4)
                    .max(4);

                println!(
                    "{:<col1$}  {:<col2$}  REQUIRED  DEFAULT",
                    "FIELD", "TYPE", col1 = col1, col2 = col2
                );
                println!("{}", "-".repeat(col1 + col2 + 22));

                for field_name in &field_names {
                    let field = &def.fields[*field_name];
                    let type_str = field_type_str(&field.field_type);
                    let required = if field.required { "yes" } else { "" };
                    let default = field.default.as_deref().unwrap_or("");
                    println!(
                        "{:<col1$}  {:<col2$}  {:<8}  {}",
                        field_name, type_str, required, default,
                        col1 = col1, col2 = col2
                    );
                }
            }
        }
    }

    Ok(())
}
