use crate::value::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// A Second Brain entity parsed from a Markdown file.
///
/// Each entity has YAML frontmatter (structured metadata) and a Markdown body.
/// The `id` and `entity_type` are extracted from frontmatter for convenience.
///
/// # Examples
///
/// ```
/// use cortx::entity::Entity;
/// use cortx::value::Value;
/// use std::collections::HashMap;
///
/// let mut fm = HashMap::new();
/// fm.insert("id".into(), Value::String("task-1".into()));
/// fm.insert("type".into(), Value::String("task".into()));
/// fm.insert("title".into(), Value::String("Buy milk".into()));
///
/// let entity = Entity::new(fm, "# Notes\n".into());
/// assert_eq!(entity.id, "task-1");
/// assert_eq!(entity.entity_type, "task");
/// assert_eq!(entity.title(), "Buy milk");
/// ```
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub frontmatter: HashMap<String, Value>,
    pub body: String,
    pub file_path: Option<PathBuf>,
}

impl Entity {
    pub fn new(frontmatter: HashMap<String, Value>, body: String) -> Self {
        let id = frontmatter
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let entity_type = frontmatter
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        Entity {
            id,
            entity_type,
            frontmatter,
            body,
            file_path: None,
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn get(&self, field: &str) -> Option<&Value> {
        self.frontmatter.get(field)
    }

    pub fn title(&self) -> &str {
        self.frontmatter
            .get("title")
            .or_else(|| self.frontmatter.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&self.id)
    }
}
