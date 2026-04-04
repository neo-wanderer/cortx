use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::file_lock;
use super::Repository;
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

    fn read_entity(&self, path: &Path) -> Result<Entity> {
        let content = std::fs::read_to_string(path)?;
        let (fm, body) = parse_frontmatter(&content)?;
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        Ok(Entity::new(id, fm, body).with_path(path.to_path_buf()))
    }

    fn scan_folder(&self, folder: &Path) -> Result<Vec<Entity>> {
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
            .filter_map(|path| self.read_entity(path).ok())
            .collect();

        Ok(entities)
    }
}

impl Repository for MarkdownRepository {
    fn create(
        &self,
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

        let id = frontmatter
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CortxError::Validation("missing 'id' field".into()))?;

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

        let content = serialize_entity(&frontmatter, body);
        std::fs::write(&path, content)?;

        Ok(Entity::new(id.to_string(), frontmatter, body.to_string()).with_path(path))
    }

    fn get_by_id(&self, id: &str, registry: &TypeRegistry) -> Result<Entity> {
        let path = self.find_file_by_id(id, registry)?;
        self.read_entity(&path)
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

        let mut entity = self.read_entity(&path)?;

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

        let content = serialize_entity(&entity.frontmatter, &entity.body);
        std::fs::write(&path, content)?;

        lock.release()?;

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
        self.scan_folder(&folder)
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
