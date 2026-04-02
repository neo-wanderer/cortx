use std::collections::HashMap;
use std::path::PathBuf;
use crate::value::Value;

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
