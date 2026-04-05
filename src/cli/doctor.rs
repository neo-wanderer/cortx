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
    /// Check filename/title drift, case-insensitive collisions, and wikilink format
    Filenames {
        #[arg(long, default_value_t = false)]
        fix: bool,
        /// Additionally scan note bodies for unresolved [[wikilinks]]
        #[arg(long, default_value_t = false)]
        check_bodies: bool,
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

                    // Collect all referenced IDs from this field (scalar link or array[link])
                    let ref_ids: Vec<String> = match entity.frontmatter.get(field_name) {
                        Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
                        Some(Value::Array(items)) => items
                            .iter()
                            .filter_map(|v| {
                                v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string())
                            })
                            .collect(),
                        _ => continue,
                    };
                    if ref_ids.is_empty() {
                        continue;
                    }

                    for ref_id in &ref_ids {
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

                        let ref_entity = match repo.get_by_id(ref_id.as_str(), &config.registry) {
                            Ok(e) => e,
                            Err(_) => {
                                issues += 1;
                                println!(
                                    "DANGLING LINK: {}.{} = {} — entity '{}' not found",
                                    entity.id, field_name, ref_id, ref_id
                                );
                                continue;
                            }
                        };

                        let has_back_ref = if link_def.inverse_one {
                            // One-to-one: inverse field is a scalar string
                            matches!(
                                ref_entity.frontmatter.get(&inverse_field),
                                Some(Value::String(s)) if s == &entity.id
                            )
                        } else {
                            // Many-to-one / many-to-many: inverse field is an array
                            match ref_entity.frontmatter.get(&inverse_field) {
                                Some(Value::Array(items)) => {
                                    items.contains(&Value::String(entity.id.clone()))
                                }
                                _ => false,
                            }
                        };

                        if !has_back_ref {
                            issues += 1;
                            println!(
                                "MISSING INVERSE: {}.{} = {} — {}.{} does not contain {}",
                                entity.id, field_name, ref_id, ref_id, inverse_field, entity.id
                            );

                            if *fix {
                                let mut updates = HashMap::new();
                                if link_def.inverse_one {
                                    // One-to-one: set inverse field to scalar string
                                    updates.insert(
                                        inverse_field.clone(),
                                        Value::String(entity.id.clone()),
                                    );
                                } else {
                                    // Many: append to array
                                    let mut items = match ref_entity.frontmatter.get(&inverse_field)
                                    {
                                        Some(Value::Array(arr)) => arr.clone(),
                                        _ => vec![],
                                    };
                                    if !items.contains(&Value::String(entity.id.clone())) {
                                        items.push(Value::String(entity.id.clone()));
                                    }
                                    updates.insert(inverse_field.clone(), Value::Array(items));
                                }
                                repo.update(ref_id.as_str(), updates, &config.registry)?;
                                repaired += 1;
                                println!("  FIXED");
                            }
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
        DoctorCommands::Filenames { fix, check_bodies } => {
            run_filenames_check(config, *fix, *check_bodies)?;
        }
    }

    Ok(())
}

fn run_filenames_check(config: &Config, fix: bool, check_bodies: bool) -> Result<()> {
    use crate::frontmatter::{parse_frontmatter, serialize_entity};
    use crate::schema::types::FieldType;
    use crate::slug::sanitize_title;
    use crate::wikilink::{is_wrapped, wrap};
    use walkdir::WalkDir;

    let mut issues = 0;
    let mut fixed = 0;
    let mut seen_ids: HashMap<String, std::path::PathBuf> = HashMap::new();

    for type_name in config.registry.type_names() {
        let Some(td) = config.registry.get(type_name) else {
            continue;
        };
        let folder = config.vault_path.join(&td.folder);
        if !folder.exists() {
            continue;
        }

        for entry in WalkDir::new(&folder)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };

            // Case-insensitive collision check
            let lower = stem.to_lowercase();
            if let Some(prev) = seen_ids.get(&lower) {
                if prev != path {
                    issues += 1;
                    println!(
                        "CASE COLLISION: {} and {} differ only in case",
                        prev.display(),
                        path.display()
                    );
                }
            } else {
                seen_ids.insert(lower, path.to_path_buf());
            }

            // Read file to check drift and wikilink format
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    println!("READ ERROR: {}: {e}", path.display());
                    issues += 1;
                    continue;
                }
            };
            let (mut fm, body) = match parse_frontmatter(&content) {
                Ok(x) => x,
                Err(e) => {
                    println!("PARSE ERROR: {}: {e}", path.display());
                    issues += 1;
                    continue;
                }
            };

            // Drift check: filename stem should equal sanitize(title)
            let title = fm
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !title.is_empty() {
                let expected_id = sanitize_title(&title);
                if expected_id != stem {
                    issues += 1;
                    println!(
                        "DRIFT: {} (stem={stem}, title='{title}', expected stem='{expected_id}')",
                        path.display()
                    );
                    if fix && !expected_id.is_empty() {
                        let new_path = path.with_file_name(format!("{expected_id}.md"));
                        if !new_path.exists() {
                            std::fs::rename(path, &new_path)?;
                            fixed += 1;
                            println!("  FIXED: renamed to {}", new_path.display());
                        } else {
                            println!("  (skip fix: target {} exists)", new_path.display());
                        }
                    }
                }
            }

            // Wikilink format check for link-typed fields
            let Some(entity_type) = fm.get("type").and_then(|v| v.as_str()).map(String::from)
            else {
                continue;
            };
            let Some(entity_td) = config.registry.get(&entity_type).cloned() else {
                continue;
            };
            let mut file_fixed = false;
            for (field_name, fd) in &entity_td.fields {
                let is_link = matches!(fd.field_type, FieldType::Link(_) | FieldType::ArrayLink(_));
                if !is_link {
                    continue;
                }
                let Some(val) = fm.get_mut(field_name) else {
                    continue;
                };
                match val {
                    Value::String(s) if !s.is_empty() && !is_wrapped(s) => {
                        issues += 1;
                        println!(
                            "WIKILINK FORMAT: {}.{} = {:?} (not wrapped)",
                            path.display(),
                            field_name,
                            s
                        );
                        if fix {
                            *s = wrap(s);
                            file_fixed = true;
                            println!("  FIXED: wrapped");
                        }
                    }
                    Value::Array(items) => {
                        for item in items.iter_mut() {
                            if let Value::String(s) = item
                                && !s.is_empty()
                                && !is_wrapped(s)
                            {
                                issues += 1;
                                println!(
                                    "WIKILINK FORMAT: {}.{} contains {:?} (not wrapped)",
                                    path.display(),
                                    field_name,
                                    s
                                );
                                if fix {
                                    *s = wrap(s);
                                    file_fixed = true;
                                    println!("  FIXED: wrapped");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if file_fixed {
                std::fs::write(path, serialize_entity(&fm, &body))?;
                fixed += 1;
            }

            // Body wikilink integrity (--check-bodies)
            if check_bodies {
                for token in extract_body_wikilinks(&body) {
                    if find_entity_by_id_global(&config.vault_path, &token, &config.registry)
                        .is_none()
                    {
                        issues += 1;
                        println!("UNRESOLVED BODY LINK: {} → [[{token}]]", path.display());
                    }
                }
            }
        }
    }

    if issues == 0 {
        println!("All filenames, titles, and wikilink formats OK.");
    } else if fix {
        println!("\n{issues} issue(s) found, {fixed} fixed.");
    } else {
        println!(
            "\n{issues} issue(s) found. Run with --fix to auto-repair drift and wikilink format."
        );
        return Err(CortxError::Validation(format!(
            "{issues} filename/wikilink issue(s)"
        )));
    }
    Ok(())
}

fn extract_body_wikilinks(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let inner = &after[..end];
            if !inner.contains('|') && !inner.trim().is_empty() {
                out.push(inner.trim().to_string());
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    out
}

fn find_entity_by_id_global(
    vault: &std::path::Path,
    id: &str,
    registry: &crate::schema::registry::TypeRegistry,
) -> Option<std::path::PathBuf> {
    for type_name in registry.type_names() {
        let Some(td) = registry.get(type_name) else {
            continue;
        };
        let path = vault.join(&td.folder).join(format!("{id}.md"));
        if path.exists() {
            return Some(path);
        }
    }
    None
}
