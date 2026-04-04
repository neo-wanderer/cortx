use crate::error::{CortxError, Result};
use crate::value::Value;
use std::collections::HashMap;

/// Parse a Markdown file's content into (frontmatter HashMap, body string).
///
/// Expects `---` delimited YAML frontmatter at the top of the content.
///
/// # Examples
///
/// ```
/// use cortx::frontmatter::parse_frontmatter;
/// use cortx::value::Value;
///
/// let content = "---\ntype: task\ntitle: Do the thing\n---\n# Body\n";
/// let (fm, body) = parse_frontmatter(content).unwrap();
/// assert_eq!(fm.get("type").unwrap(), &Value::String("task".into()));
/// assert!(body.contains("# Body"));
/// ```
///
/// # Errors
///
/// Returns an error if no `---` delimiters are found.
pub fn parse_frontmatter(content: &str) -> Result<(HashMap<String, Value>, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(CortxError::Storage("no YAML frontmatter found".into()));
    }

    let after_first = &trimmed[3..];
    let close_pos = after_first
        .find("\n---")
        .ok_or_else(|| CortxError::Storage("unclosed YAML frontmatter".into()))?;

    let yaml_str = &after_first[..close_pos];
    let body_start = close_pos + 4;
    let body = after_first[body_start..]
        .trim_start_matches('\n')
        .to_string();

    let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_str)?;
    let mapping = yaml_value
        .as_mapping()
        .ok_or_else(|| CortxError::Storage("frontmatter must be a YAML mapping".into()))?;

    let mut fm = HashMap::new();
    for (k, v) in mapping {
        if let Some(key) = k.as_str() {
            fm.insert(key.to_string(), Value::from_yaml(v));
        }
    }

    Ok((fm, body))
}

/// Serialize a frontmatter HashMap and body back into a Markdown string.
///
/// Keys are sorted alphabetically for deterministic output.
///
/// # Examples
///
/// ```
/// use cortx::frontmatter::serialize_entity;
/// use cortx::value::Value;
/// use std::collections::HashMap;
///
/// let mut fm = HashMap::new();
/// fm.insert("type".into(), Value::String("task".into()));
/// fm.insert("title".into(), Value::String("Do the thing".into()));
///
/// let output = serialize_entity(&fm, "# Notes\n");
/// assert!(output.starts_with("---\n"));
/// assert!(output.contains("title: Do the thing"));
/// assert!(output.ends_with("# Notes\n"));
/// ```
pub fn serialize_entity(frontmatter: &HashMap<String, Value>, body: &str) -> String {
    let mut yaml_map = serde_yaml::Mapping::new();

    let mut keys: Vec<&String> = frontmatter.keys().collect();
    keys.sort();

    for key in keys {
        let val = &frontmatter[key];
        yaml_map.insert(serde_yaml::Value::String(key.clone()), val.to_yaml());
    }

    let yaml_str = serde_yaml::to_string(&serde_yaml::Value::Mapping(yaml_map)).unwrap_or_default();

    format!("---\n{yaml_str}---\n{body}")
}
