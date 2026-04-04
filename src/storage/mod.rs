pub mod file_lock;
pub mod markdown;

use crate::entity::Entity;
use crate::error::Result;
use crate::schema::registry::TypeRegistry;
use crate::value::Value;
use std::collections::HashMap;

/// Storage abstraction for entity persistence.
///
/// Implementations handle the details of where and how entities are stored.
/// The Markdown adapter (`MarkdownRepository`) stores entities as `.md` files
/// with YAML frontmatter. Future adapters (e.g., SQLite) implement the same trait.
pub trait Repository {
    fn create(
        &self,
        id: &str,
        frontmatter: HashMap<String, Value>,
        body: &str,
        registry: &TypeRegistry,
    ) -> Result<Entity>;

    fn get_by_id(&self, id: &str, registry: &TypeRegistry) -> Result<Entity>;

    fn update(
        &self,
        id: &str,
        updates: HashMap<String, Value>,
        registry: &TypeRegistry,
    ) -> Result<Entity>;

    fn delete(&self, id: &str, registry: &TypeRegistry) -> Result<()>;

    fn list_by_type(&self, entity_type: &str, registry: &TypeRegistry) -> Result<Vec<Entity>>;

    fn list_all(&self, registry: &TypeRegistry) -> Result<Vec<Entity>>;
}
