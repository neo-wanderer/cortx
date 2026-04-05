use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::Repository;
use super::file_lock;
use crate::entity::Entity;
use crate::error::{CortxError, Result};
use crate::frontmatter::{parse_frontmatter, serialize_entity};
use crate::schema::registry::TypeRegistry;
use crate::schema::validation::validate_frontmatter;
use crate::value::Value;

pub struct MarkdownRepository {
    vault_path: PathBuf,
}

impl MarkdownRepository {
    pub fn new(vault_path: PathBuf) -> Self {
        MarkdownRepository { vault_path }
    }

    fn resolve_path(&self, type_name: &str, id: &str, registry: &TypeRegistry) -> Result<PathBuf> {
        let type_def = registry
            .get(type_name)
            .ok_or_else(|| CortxError::Schema(format!("unknown type '{type_name}'")))?;
        Ok(self
            .vault_path
            .join(&type_def.folder)
            .join(format!("{id}.md")))
    }

    fn find_file_by_id(&self, id: &str, registry: &TypeRegistry) -> Result<PathBuf> {
        for type_name in registry.type_names() {
            if let Some(type_def) = registry.get(type_name) {
                let path = self
                    .vault_path
                    .join(&type_def.folder)
                    .join(format!("{id}.md"));
                if path.exists() {
                    return Ok(path);
                }
            }
        }
        Err(CortxError::NotFound(format!("entity '{id}' not found")))
    }

    fn read_entity(&self, path: &Path, registry: &TypeRegistry) -> Result<Entity> {
        let content = std::fs::read_to_string(path)?;
        let (mut fm, body) = parse_frontmatter(&content)?;
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Unwrap link-typed fields using the entity's type definition
        if let Some(type_name) = fm.get("type").and_then(|v| v.as_str())
            && let Some(type_def) = registry.get(type_name)
        {
            crate::wikilink::unwrap_frontmatter(&mut fm, type_def)?;
        }

        Ok(Entity::new(id, fm, body).with_path(path.to_path_buf()))
    }

    /// If `field_name` is a bidirectional link in the schema, update the inverse
    /// field on the referenced entity. Acquires a lock on the ref file only.
    fn apply_bidirectional(
        &self,
        owning_id: &str,
        field_name: &str,
        new_value: &Value,
        registry: &TypeRegistry,
        type_def: &crate::schema::types::TypeDefinition,
    ) -> Result<()> {
        use crate::schema::types::{FieldType, LinkTargets};

        let field_def = match type_def.fields.get(field_name) {
            Some(f) => f,
            None => return Ok(()),
        };

        let link_def = match &field_def.field_type {
            FieldType::Link(d) | FieldType::ArrayLink(d) if d.bidirectional => d,
            _ => return Ok(()),
        };

        let ref_ids: Vec<String> = match new_value {
            Value::String(s) if !s.is_empty() => vec![s.clone()],
            Value::Array(items) => items
                .iter()
                .filter_map(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
                .collect(),
            _ => return Ok(()),
        };
        if ref_ids.is_empty() {
            return Ok(());
        }

        for ref_id in &ref_ids {
            let (ref_type, inverse_field) = match &link_def.targets {
                LinkTargets::Single {
                    ref_type,
                    inverse: Some(inv),
                } => (ref_type.clone(), inv.clone()),
                LinkTargets::Poly(targets) => {
                    let matched = targets.iter().find_map(|t| {
                        let ref_path = self.resolve_path(&t.ref_type, ref_id, registry).ok()?;
                        if ref_path.exists() {
                            t.inverse
                                .as_deref()
                                .map(|inv| (t.ref_type.clone(), inv.to_string()))
                        } else {
                            None
                        }
                    });
                    match matched {
                        Some((rt, inv)) => (rt, inv),
                        None => continue,
                    }
                }
                _ => continue,
            };

            let ref_path = self.resolve_path(&ref_type, ref_id, registry)?;
            if !ref_path.exists() {
                continue;
            }

            let _lock = file_lock::FileLock::acquire(&ref_path)?;

            let ref_content = std::fs::read_to_string(&ref_path)?;
            let (mut ref_fm, ref_body) = parse_frontmatter(&ref_content)?;

            // Unwrap link-typed fields so we're working with bare titles
            if let Some(ref_type_name) = ref_fm.get("type").and_then(|v| v.as_str())
                && let Some(ref_type_def) = registry.get(ref_type_name)
            {
                crate::wikilink::unwrap_frontmatter(&mut ref_fm, ref_type_def)?;
            }

            let arr = ref_fm
                .entry(inverse_field)
                .or_insert_with(|| Value::Array(vec![]));
            if !matches!(arr, Value::Array(_)) {
                *arr = Value::Array(vec![]);
            }
            if let Value::Array(items) = arr {
                let id_val = Value::String(owning_id.to_string());
                if !items.contains(&id_val) {
                    items.push(id_val);
                }
            }

            // Wrap link-typed fields before serialization
            if let Some(ref_type_name) = ref_fm.get("type").and_then(|v| v.as_str()).map(String::from)
                && let Some(ref_type_def) = registry.get(&ref_type_name)
            {
                crate::wikilink::wrap_frontmatter(&mut ref_fm, ref_type_def);
            }

            let updated = serialize_entity(&ref_fm, &ref_body);
            std::fs::write(&ref_path, updated)?;
        }

        Ok(())
    }

    fn scan_folder(&self, folder: &Path, registry: &TypeRegistry) -> Result<Vec<Entity>> {
        if !folder.exists() {
            return Ok(Vec::new());
        }

        let paths: Vec<PathBuf> = WalkDir::new(folder)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.path().to_path_buf())
            .collect();

        let entities: Vec<Entity> = paths
            .par_iter()
            .filter_map(|path| self.read_entity(path, registry).ok())
            .collect();

        Ok(entities)
    }
}

impl Repository for MarkdownRepository {
    fn create(
        &self,
        id: &str,
        frontmatter: HashMap<String, Value>,
        body: &str,
        registry: &TypeRegistry,
    ) -> Result<Entity> {
        let type_name = frontmatter
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CortxError::Validation("missing 'type' field".into()))?
            .to_string();

        let type_def = registry
            .get(&type_name)
            .ok_or_else(|| CortxError::Schema(format!("unknown type '{type_name}'")))?;

        validate_frontmatter(&frontmatter, type_def)?;

        let path = self.resolve_path(&type_name, id, registry)?;

        if path.exists() {
            return Err(CortxError::Storage(format!(
                "entity '{id}' already exists at {}",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Wrap link-typed fields before serialization
        let mut fm_for_write = frontmatter.clone();
        crate::wikilink::wrap_frontmatter(&mut fm_for_write, type_def);

        let content = serialize_entity(&fm_for_write, body);
        let _lock = file_lock::FileLock::acquire(&path)?;
        std::fs::write(&path, content)?;

        // Maintain bidirectional inverse fields (use bare-title values)
        for (field_name, value) in &frontmatter {
            self.apply_bidirectional(id, field_name, value, registry, type_def)?;
        }

        Ok(Entity::new(id.to_string(), frontmatter, body.to_string()).with_path(path))
    }

    fn get_by_id(&self, id: &str, registry: &TypeRegistry) -> Result<Entity> {
        let path = self.find_file_by_id(id, registry)?;
        self.read_entity(&path, registry)
    }

    fn update(
        &self,
        id: &str,
        updates: HashMap<String, Value>,
        registry: &TypeRegistry,
    ) -> Result<Entity> {
        let path = self.find_file_by_id(id, registry)?;

        // Acquire file lock
        let lock = file_lock::FileLock::acquire(&path)?;

        let mut entity = self.read_entity(&path, registry)?;

        let updates_snapshot = updates.clone();

        for (key, val) in updates {
            entity.frontmatter.insert(key, val);
        }

        let today = chrono::Local::now().date_naive();
        entity
            .frontmatter
            .insert("updated_at".into(), Value::Date(today));

        if let Some(type_def) = registry.get(&entity.entity_type) {
            validate_frontmatter(&entity.frontmatter, type_def)?;
        }

        // Wrap link-typed fields before serialization
        let mut fm_for_write = entity.frontmatter.clone();
        if let Some(type_def) = registry.get(&entity.entity_type) {
            crate::wikilink::wrap_frontmatter(&mut fm_for_write, type_def);
        }
        let content = serialize_entity(&fm_for_write, &entity.body);
        std::fs::write(&path, content)?;

        lock.release()?;

        // Maintain bidirectional inverse fields for link fields that changed
        let type_def_for_bidir = registry.get(&entity.entity_type);
        if let Some(type_def) = type_def_for_bidir {
            for (field_name, value) in &updates_snapshot {
                self.apply_bidirectional(id, field_name, value, registry, type_def)?;
            }
        }

        entity.file_path = Some(path);
        Ok(entity)
    }

    fn delete(&self, id: &str, registry: &TypeRegistry) -> Result<()> {
        let path = self.find_file_by_id(id, registry)?;

        let lock = file_lock::FileLock::acquire(&path)?;

        std::fs::remove_file(&path)?;

        lock.release()?;

        Ok(())
    }

    fn list_by_type(&self, entity_type: &str, registry: &TypeRegistry) -> Result<Vec<Entity>> {
        let type_def = registry
            .get(entity_type)
            .ok_or_else(|| CortxError::Schema(format!("unknown type '{entity_type}'")))?;
        let folder = self.vault_path.join(&type_def.folder);
        self.scan_folder(&folder, registry)
    }

    fn list_all(&self, registry: &TypeRegistry) -> Result<Vec<Entity>> {
        let mut all = Vec::new();
        for type_name in registry.type_names() {
            let mut entities = self.list_by_type(type_name, registry)?;
            all.append(&mut entities);
        }
        Ok(all)
    }
}
