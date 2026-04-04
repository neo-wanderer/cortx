use crate::config::Config;
use crate::error::{CortxError, Result};
use crate::schema::types::{FieldType, LinkDef, LinkTargets};
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
    /// Validate types.yaml — check ref integrity and relation consistency
    Validate,
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
        FieldType::Link(def) => format!("link:{}", link_targets_str(def)),
        FieldType::ArrayLink(def) => format!("array[link]:{}", link_targets_str(def)),
    }
}

fn link_targets_str(def: &LinkDef) -> String {
    match &def.targets {
        LinkTargets::Single { ref_type, .. } => ref_type.clone(),
        LinkTargets::Poly(targets) => targets
            .iter()
            .map(|t| t.ref_type.as_str())
            .collect::<Vec<_>>()
            .join("|"),
    }
}

pub fn run(args: &SchemaArgs, config: &Config) -> Result<()> {
    match &args.command {
        SchemaCommands::Types { format } => {
            let mut names: Vec<&str> = config.registry.type_names();
            names.sort_unstable();

            if format == "json" {
                let arr: Vec<serde_json::Value> = names
                    .iter()
                    .map(|n| serde_json::Value::String(n.to_string()))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&arr).unwrap_or_default());
            } else {
                println!("Types ({}):\n", names.len());
                for name in &names {
                    let def = config.registry.get(name).unwrap();
                    println!("  {name}  (folder: {})", def.folder);
                }
            }
        }

        SchemaCommands::Validate => {
            let issues = validate_schema_types(config);
            if issues.is_empty() {
                let type_count = config.registry.type_names().len();
                let (bidir_count, poly_count) = count_relation_stats(config);
                println!(
                    "types.yaml is valid ({type_count} types, {bidir_count} bidirectional link fields, {poly_count} polymorphic fields)."
                );
            } else {
                for issue in &issues {
                    println!("{issue}");
                }
                let err_count = issues.iter().filter(|e| e.starts_with("ERROR")).count();
                let warn_count = issues.iter().filter(|e| e.starts_with("WARN")).count();
                println!("\n{err_count} error(s), {warn_count} warning(s).");
                if err_count > 0 {
                    return Err(CortxError::Validation(format!(
                        "{err_count} schema error(s) found"
                    )));
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
                        FieldType::Link(link_def) | FieldType::ArrayLink(link_def) => {
                            let is_array = matches!(field.field_type, FieldType::ArrayLink(_));
                            obj.insert(
                                "type".into(),
                                serde_json::Value::String(
                                    if is_array { "array[link]" } else { "link" }.into(),
                                ),
                            );
                            match &link_def.targets {
                                LinkTargets::Single { ref_type, .. } if !ref_type.is_empty() => {
                                    obj.insert(
                                        "ref".into(),
                                        serde_json::Value::String(ref_type.clone()),
                                    );
                                }
                                LinkTargets::Poly(targets) => {
                                    let refs: Vec<serde_json::Value> = targets
                                        .iter()
                                        .map(|t| serde_json::Value::String(t.ref_type.clone()))
                                        .collect();
                                    obj.insert("ref".into(), serde_json::Value::Array(refs));
                                }
                                _ => {}
                            }
                            if link_def.bidirectional {
                                obj.insert("bidirectional".into(), serde_json::Value::Bool(true));
                            }
                        }
                        _ => {
                            obj.insert("type".into(), serde_json::Value::String(type_str));
                        }
                    }

                    obj.insert("required".into(), serde_json::Value::Bool(field.required));
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

                let col1 = field_names
                    .iter()
                    .map(|n| n.len())
                    .max()
                    .unwrap_or(5)
                    .max(5);
                let col2 = field_names
                    .iter()
                    .map(|n| field_type_str(&def.fields[*n].field_type).len())
                    .max()
                    .unwrap_or(4)
                    .max(4);

                println!(
                    "{:<col1$}  {:<col2$}  REQUIRED  DEFAULT",
                    "FIELD",
                    "TYPE",
                    col1 = col1,
                    col2 = col2
                );
                println!("{}", "-".repeat(col1 + col2 + 22));

                for field_name in &field_names {
                    let field = &def.fields[*field_name];
                    let type_str = field_type_str(&field.field_type);
                    let required = if field.required { "yes" } else { "" };
                    let default = field.default.as_deref().unwrap_or("");
                    println!(
                        "{:<col1$}  {:<col2$}  {:<8}  {}",
                        field_name,
                        type_str,
                        required,
                        default,
                        col1 = col1,
                        col2 = col2
                    );
                }
            }
        }
    }

    Ok(())
}

/// Validate all link fields in the registry for ref integrity and inverse consistency.
/// Returns a list of human-readable issue strings prefixed with ERROR or WARN.
fn validate_schema_types(config: &Config) -> Vec<String> {
    let mut issues: Vec<String> = Vec::new();

    for type_name in config.registry.type_names() {
        let type_def = config.registry.get(type_name).unwrap();

        for (field_name, field_def) in &type_def.fields {
            let (link_def, is_array) = match &field_def.field_type {
                FieldType::Link(d) => (d, false),
                FieldType::ArrayLink(d) => (d, true),
                _ => continue,
            };

            // inverse_one on array[link] doesn't make sense
            if link_def.inverse_one && is_array {
                issues.push(format!(
                    "WARN   {type_name}.{field_name} — inverse_one: true on array[link] field (ignored)"
                ));
            }

            // Collect (ref_type, inverse) pairs to check
            let targets: Vec<(&str, Option<&str>)> = match &link_def.targets {
                LinkTargets::Single { ref_type, inverse } => {
                    if ref_type.is_empty() {
                        continue;
                    }
                    vec![(ref_type.as_str(), inverse.as_deref())]
                }
                LinkTargets::Poly(targets) => targets
                    .iter()
                    .map(|t| (t.ref_type.as_str(), t.inverse.as_deref()))
                    .collect(),
            };

            for (ref_type, inverse) in targets {
                // Check ref type exists in registry
                let target_def = match config.registry.get(ref_type) {
                    Some(d) => d,
                    None => {
                        issues.push(format!(
                            "ERROR  {type_name}.{field_name} — ref type '{ref_type}' not found in registry"
                        ));
                        continue;
                    }
                };

                if !link_def.bidirectional {
                    continue;
                }

                // Bidirectional: check inverse is declared
                let inv_field = match inverse {
                    Some(f) => f,
                    None => {
                        issues.push(format!(
                            "ERROR  {type_name}.{field_name} — bidirectional: true but no inverse declared for ref '{ref_type}'"
                        ));
                        continue;
                    }
                };

                // Check inverse field exists on target type
                if !target_def.fields.contains_key(inv_field) {
                    issues.push(format!(
                        "ERROR  {type_name}.{field_name} — inverse '{inv_field}' not found on type '{ref_type}'"
                    ));
                }

                // Check for reflexive loop (same type, same field)
                if ref_type == type_name && inv_field == field_name {
                    issues.push(format!(
                        "ERROR  {type_name}.{field_name} — reflexive bidirectional link points to same field"
                    ));
                }
            }
        }
    }

    issues
}

fn count_relation_stats(config: &Config) -> (usize, usize) {
    let mut bidir = 0usize;
    let mut poly = 0usize;
    for type_name in config.registry.type_names() {
        let type_def = config.registry.get(type_name).unwrap();
        for field_def in type_def.fields.values() {
            let link_def = match &field_def.field_type {
                FieldType::Link(d) | FieldType::ArrayLink(d) => d,
                _ => continue,
            };
            if link_def.bidirectional {
                bidir += 1;
            }
            if matches!(link_def.targets, LinkTargets::Poly(_)) {
                poly += 1;
            }
        }
    }
    (bidir, poly)
}
