use crate::config::Config;
use crate::error::{CortxError, Result};
use crate::frontmatter::{parse_frontmatter, serialize_entity};
use crate::schema::types::FieldType;
use crate::slug::sanitize_title;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::Args;
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Args)]
pub struct RenameArgs {
    /// The current title of the entity
    pub old_title: String,

    /// The new title
    pub new_title: String,

    /// Show the plan without applying changes
    #[arg(long)]
    pub dry_run: bool,

    /// Skip rewriting body wikilinks (only update frontmatter back-refs)
    #[arg(long)]
    pub skip_body: bool,
}

pub fn run(args: &RenameArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone())
        .with_link_validation(false); // we're rewriting existing refs, not creating new ones

    let old_id = sanitize_title(&args.old_title);
    let new_id = sanitize_title(&args.new_title);

    if new_id.is_empty() {
        return Err(CortxError::Validation(
            "new title sanitizes to empty id — provide alphanumeric content".into(),
        ));
    }
    if old_id == new_id {
        println!("No change: old and new ids are identical after sanitization.");
        return Ok(());
    }

    // Resolve the entity being renamed
    let old_entity = repo.get_by_id(&old_id, &config.registry)?;
    let old_path = old_entity
        .file_path
        .clone()
        .ok_or_else(|| CortxError::Storage(format!("entity '{old_id}' has no file path")))?;

    // Case-insensitive collision check for new_id (excluding the file being renamed)
    if collision_exists(&config.vault_path, &new_id, &old_path, &config.registry)? {
        return Err(CortxError::Storage(format!(
            "new id '{new_id}' collides with an existing file (case-insensitive). \
             Choose a different title."
        )));
    }

    // Compute the new path (same folder, new stem)
    let new_path = old_path.with_file_name(format!("{new_id}.md"));

    // Plan: file rename + frontmatter back-ref rewrites + body wikilink rewrites
    let back_ref_sites = find_back_refs(&config.vault_path, &old_id, &old_path, &config.registry)?;
    let body_sites = if args.skip_body {
        Vec::new()
    } else {
        find_body_wikilinks(&config.vault_path, &old_id, &old_path)?
    };

    println!(
        "renamed: {} → {}",
        rel(&old_path, &config.vault_path),
        rel(&new_path, &config.vault_path)
    );
    if !back_ref_sites.is_empty() {
        println!(
            "updated {} frontmatter back-reference(s):",
            back_ref_sites.len()
        );
        for site in &back_ref_sites {
            println!("  {} ({})", rel(&site.path, &config.vault_path), site.field);
        }
    }
    if !body_sites.is_empty() {
        println!("updated {} body wikilink site(s):", body_sites.len());
        for site in &body_sites {
            println!("  {}", rel(&site.path, &config.vault_path));
        }
    }

    if args.dry_run {
        println!("(dry-run, no files written)");
        return Ok(());
    }

    // --- Transactional apply ---
    // 1. Snapshot every file we're about to touch
    let mut snapshots: HashMap<PathBuf, String> = HashMap::new();
    snapshots.insert(old_path.clone(), std::fs::read_to_string(&old_path)?);
    for site in &back_ref_sites {
        if !snapshots.contains_key(&site.path) {
            snapshots.insert(site.path.clone(), std::fs::read_to_string(&site.path)?);
        }
    }
    for site in &body_sites {
        if !snapshots.contains_key(&site.path) {
            snapshots.insert(site.path.clone(), std::fs::read_to_string(&site.path)?);
        }
    }

    // 2. Apply, rolling back on any failure
    let result: Result<()> = (|| {
        apply_rename(&old_path, &new_path, &args.new_title)?;
        for site in &back_ref_sites {
            rewrite_frontmatter_back_ref(&site.path, &old_id, &new_id, &config.registry)?;
        }
        for site in &body_sites {
            // If the body-site was the entity being renamed, it's now at new_path
            let effective_path = if site.path == old_path {
                new_path.clone()
            } else {
                site.path.clone()
            };
            rewrite_body_wikilinks(&effective_path, &old_id, &new_id)?;
        }
        Ok(())
    })();

    if let Err(e) = result {
        // Rollback: restore every snapshotted file
        if new_path.exists() && new_path != old_path {
            let _ = std::fs::remove_file(&new_path);
        }
        for (path, original) in &snapshots {
            std::fs::write(path, original)?;
        }
        return Err(e);
    }

    Ok(())
}

fn rel(path: &std::path::Path, vault: &std::path::Path) -> String {
    path.strip_prefix(vault)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn collision_exists(
    vault: &std::path::Path,
    new_id: &str,
    exclude: &std::path::Path,
    registry: &crate::schema::registry::TypeRegistry,
) -> Result<bool> {
    let lower = new_id.to_lowercase();
    for type_name in registry.type_names() {
        let Some(td) = registry.get(type_name) else {
            continue;
        };
        let folder = vault.join(&td.folder);
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
            if path == exclude {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && stem.to_lowercase() == lower
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub(crate) struct BackRefSite {
    pub path: PathBuf,
    pub field: String,
}

fn find_back_refs(
    vault: &std::path::Path,
    old_id: &str,
    exclude: &std::path::Path,
    registry: &crate::schema::registry::TypeRegistry,
) -> Result<Vec<BackRefSite>> {
    let mut sites = Vec::new();
    for type_name in registry.type_names() {
        let Some(td) = registry.get(type_name) else {
            continue;
        };
        let folder = vault.join(&td.folder);
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
            if path == exclude {
                continue;
            }
            let content = std::fs::read_to_string(path)?;
            let (fm, _) = match parse_frontmatter(&content) {
                Ok(x) => x,
                Err(_) => continue, // skip unparseable files
            };
            let Some(entity_type) = fm.get("type").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(entity_td) = registry.get(entity_type) else {
                continue;
            };

            for (field_name, fd) in &entity_td.fields {
                let is_link = matches!(fd.field_type, FieldType::Link(_) | FieldType::ArrayLink(_));
                if !is_link {
                    continue;
                }
                let Some(val) = fm.get(field_name) else {
                    continue;
                };
                if link_value_matches(val, old_id) {
                    sites.push(BackRefSite {
                        path: path.to_path_buf(),
                        field: field_name.clone(),
                    });
                    break; // one record per file is enough
                }
            }
        }
    }
    Ok(sites)
}

fn link_value_matches(val: &Value, target_id: &str) -> bool {
    // Values here are WRAPPED (raw from file via parse_frontmatter, no unwrap)
    let wrapped = format!("[[{target_id}]]");
    match val {
        Value::String(s) => s == &wrapped,
        Value::Array(items) => items
            .iter()
            .any(|v| matches!(v, Value::String(s) if s == &wrapped)),
        _ => false,
    }
}

fn apply_rename(
    old_path: &std::path::Path,
    new_path: &std::path::Path,
    new_title: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(old_path)?;
    let (mut fm, body) = parse_frontmatter(&content)?;
    // Update title field
    fm.insert("title".into(), Value::String(new_title.to_string()));
    // Bump updated_at
    fm.insert(
        "updated_at".into(),
        Value::Date(chrono::Local::now().date_naive()),
    );
    let new_content = serialize_entity(&fm, &body);
    std::fs::write(new_path, new_content)?;
    if new_path != old_path {
        std::fs::remove_file(old_path)?;
    }
    Ok(())
}

pub(crate) struct BodyRefSite {
    pub path: PathBuf,
}

fn find_body_wikilinks(
    vault: &std::path::Path,
    old_id: &str,
    _exclude: &std::path::Path,
) -> Result<Vec<BodyRefSite>> {
    // Note: we do NOT exclude the renamed entity itself — its own body may
    // contain self-references that need updating after the rename.
    let mut sites = Vec::new();
    let token = format!("[[{old_id}]]");
    for entry in WalkDir::new(vault).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = std::fs::read_to_string(path)?;
        // Split frontmatter from body
        let body = match content.trim_start().starts_with("---") {
            true => match content.find("\n---") {
                Some(close) => {
                    let body_start = close + 4;
                    if body_start < content.len() {
                        &content[body_start..]
                    } else {
                        ""
                    }
                }
                None => &content[..],
            },
            false => &content[..],
        };
        if body.contains(&token) {
            sites.push(BodyRefSite {
                path: path.to_path_buf(),
            });
        }
    }
    Ok(sites)
}

fn rewrite_body_wikilinks(path: &std::path::Path, old_id: &str, new_id: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let (fm, body) = parse_frontmatter(&content)?;
    let old_tok = format!("[[{old_id}]]");
    let new_tok = format!("[[{new_id}]]");
    let new_body = body.replace(&old_tok, &new_tok);
    let new_content = serialize_entity(&fm, &new_body);
    std::fs::write(path, new_content)?;
    Ok(())
}

fn rewrite_frontmatter_back_ref(
    path: &std::path::Path,
    old_id: &str,
    new_id: &str,
    registry: &crate::schema::registry::TypeRegistry,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let (mut fm, body) = parse_frontmatter(&content)?;
    let Some(entity_type) = fm.get("type").and_then(|v| v.as_str()).map(String::from) else {
        return Ok(());
    };
    let Some(entity_td) = registry.get(&entity_type).cloned() else {
        return Ok(());
    };

    let old_wrapped = format!("[[{old_id}]]");
    let new_wrapped = format!("[[{new_id}]]");

    for (field_name, fd) in &entity_td.fields {
        let is_link = matches!(fd.field_type, FieldType::Link(_) | FieldType::ArrayLink(_));
        if !is_link {
            continue;
        }
        let Some(val) = fm.get_mut(field_name) else {
            continue;
        };
        match val {
            Value::String(s) if *s == old_wrapped => {
                *s = new_wrapped.clone();
            }
            Value::Array(items) => {
                for item in items.iter_mut() {
                    if let Value::String(s) = item
                        && *s == old_wrapped
                    {
                        *s = new_wrapped.clone();
                    }
                }
            }
            _ => {}
        }
    }

    let new_content = serialize_entity(&fm, &body);
    std::fs::write(path, new_content)?;
    Ok(())
}
