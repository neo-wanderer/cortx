use crate::config::Config;
use crate::error::{CortxError, Result};
use crate::schema::validation::validate_frontmatter;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::{Args, Subcommand};
use std::collections::HashMap;

#[derive(Args)]
pub struct DoctorArgs {
    #[command(subcommand)]
    pub command: DoctorCommands,
}

#[derive(Subcommand)]
pub enum DoctorCommands {
    /// Validate all files against schemas
    Validate,
    /// Check bidirectional relation consistency; use --fix to auto-repair missing inverses
    Links {
        #[arg(long, default_value_t = false)]
        fix: bool,
    },
}

pub fn run(args: &DoctorArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());

    match &args.command {
        DoctorCommands::Validate => {
            let all = repo.list_all(&config.registry)?;
            let mut errors = 0;

            for entity in &all {
                if let Some(type_def) = config.registry.get(&entity.entity_type)
                    && let Err(e) = validate_frontmatter(&entity.frontmatter, type_def)
                {
                    errors += 1;
                    println!("ERROR in {} ({}): {e}", entity.id, entity.entity_type);
                }
            }

            if errors == 0 {
                println!("All {} entities pass validation.", all.len());
            } else {
                println!(
                    "\n{errors} validation error(s) found in {} entities.",
                    all.len()
                );
            }
        }
        DoctorCommands::Links { fix } => {
            use crate::schema::types::{FieldType, LinkTargets};

            let all = repo.list_all(&config.registry)?;
            let mut issues = 0;
            let mut repaired = 0;

            for entity in &all {
                let type_def = match config.registry.get(&entity.entity_type) {
                    Some(d) => d,
                    None => continue,
                };

                for (field_name, field_def) in &type_def.fields {
                    let link_def = match &field_def.field_type {
                        FieldType::Link(d) | FieldType::ArrayLink(d) if d.bidirectional => d,
                        _ => continue,
                    };

                    let ref_id = match entity.frontmatter.get(field_name).and_then(|v| v.as_str()) {
                        Some(s) if !s.is_empty() => s.to_string(),
                        _ => continue,
                    };

                    let (_ref_type_name, inverse_field) = match &link_def.targets {
                        LinkTargets::Single {
                            ref_type,
                            inverse: Some(inv),
                        } => (ref_type.clone(), inv.clone()),
                        LinkTargets::Poly(targets) => {
                            let matched = targets.iter().find_map(|t| {
                                let ref_path = config
                                    .vault_path
                                    .join(&config.registry.get(&t.ref_type)?.folder)
                                    .join(format!("{ref_id}.md"));
                                if ref_path.exists() {
                                    t.inverse.clone().map(|inv| (t.ref_type.clone(), inv))
                                } else {
                                    None
                                }
                            });
                            match matched {
                                Some(pair) => pair,
                                None => continue,
                            }
                        }
                        _ => continue,
                    };

                    let ref_entity = match repo.get_by_id(&ref_id, &config.registry) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    let has_back_ref = match ref_entity.frontmatter.get(&inverse_field) {
                        Some(Value::Array(items)) => {
                            items.contains(&Value::String(entity.id.clone()))
                        }
                        _ => false,
                    };

                    if !has_back_ref {
                        issues += 1;
                        println!(
                            "MISSING INVERSE: {}.{} = {} — {}.{} does not contain {}",
                            entity.id, field_name, ref_id, ref_id, inverse_field, entity.id
                        );

                        if *fix {
                            let mut updates = HashMap::new();
                            let mut items = match ref_entity.frontmatter.get(&inverse_field) {
                                Some(Value::Array(arr)) => arr.clone(),
                                _ => vec![],
                            };
                            items.push(Value::String(entity.id.clone()));
                            updates.insert(inverse_field.clone(), Value::Array(items));
                            repo.update(&ref_id, updates, &config.registry)?;
                            repaired += 1;
                            println!("  FIXED");
                        }
                    }
                }
            }

            if issues == 0 {
                println!(
                    "No bidirectional relation inconsistencies found across {} entities.",
                    all.len()
                );
            } else if *fix {
                println!("\n{issues} issue(s) found, {repaired} repaired.");
            } else {
                println!("\n{issues} issue(s) found. Run with --fix to auto-repair.");
                return Err(CortxError::Validation(format!(
                    "{issues} relation inconsistency/ies"
                )));
            }
        }
    }

    Ok(())
}
