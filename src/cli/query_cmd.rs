use crate::config::Config;
use crate::entity::Entity;
use crate::error::{CortxError, Result};
use crate::query::evaluator::evaluate;
use crate::query::parser::parse_query;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::Args;
use std::cmp::Ordering;

#[derive(Args)]
pub struct QueryArgs {
    pub expression: String,

    #[arg(long, default_value = "text")]
    pub format: String,

    /// Sort by field(s). Format: field[:order][,field[:order]...]
    ///
    /// Order defaults to 'asc'. Example: --sort-by priority:asc,due:desc
    ///
    /// Quoted fields for spaces: --sort-by '"Due By":desc'
    #[arg(long)]
    pub sort_by: Option<String>,
}

/// Sort specification parsed from --sort-by argument
#[derive(Debug, Clone)]
pub struct SortSpec {
    pub field: String,
    pub descending: bool,
}

/// Parse a --sort-by argument into sort specifications.
///
/// Supports:
/// - Single field: `due` or `due:asc`
/// - Multiple fields: `priority:asc,due:desc`
/// - Quoted fields with spaces: `"Due By":desc`
pub fn parse_sort_by(spec: &str) -> Result<Vec<SortSpec>> {
    let mut result = Vec::new();
    let chars: Vec<char> = spec.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Skip whitespace and commas
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        // Parse field name (possibly quoted)
        let field = if chars[i] == '"' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            if i >= chars.len() {
                return Err(CortxError::QueryParse(
                    "unclosed quoted field name in sort specification".into(),
                ));
            }
            let f: String = chars[start..i].iter().collect();
            i += 1; // skip closing quote
            f
        } else {
            let start = i;
            while i < chars.len() && chars[i] != ':' && chars[i] != ',' && !chars[i].is_whitespace()
            {
                i += 1;
            }
            chars[start..i].iter().collect()
        };

        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        // Parse optional :order
        let descending = if i < chars.len() && chars[i] == ':' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != ',' && !chars[i].is_whitespace() {
                i += 1;
            }
            let order: String = chars[start..i].iter().collect();
            match order.trim() {
                "desc" | "DESC" => true,
                "asc" | "ASC" => false,
                _ => {
                    return Err(CortxError::QueryParse(format!(
                        "invalid sort order '{order}', expected 'asc' or 'desc'"
                    )));
                }
            }
        } else {
            false
        };

        let field = field.trim();
        if field.is_empty() {
            return Err(CortxError::QueryParse(
                "empty field name in sort specification".into(),
            ));
        }

        result.push(SortSpec {
            field: field.to_string(),
            descending,
        });
    }

    if result.is_empty() {
        return Err(CortxError::QueryParse("empty sort specification".into()));
    }

    Ok(result)
}

/// Compare two optional values for sorting.
///
/// Null/missing values always sort to the end regardless of sort order.
fn compare_values(a: Option<&Value>, b: Option<&Value>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(av), Some(bv)) => {
            let cmp = av.partial_cmp(bv).unwrap_or(Ordering::Equal);
            if descending { cmp.reverse() } else { cmp }
        }
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Less,    // Nulls to end
        (None, Some(_)) => Ordering::Greater, // Nulls to end
    }
}

/// Sort entities by the given specifications.
///
/// Entities are sorted by each field in order. Fields with missing/null values
/// always sort to the end, regardless of ascending or descending order.
pub fn sort_entities(entities: &mut [&Entity], specs: &[SortSpec]) {
    entities.sort_by(|a, b| {
        for spec in specs {
            let cmp = compare_values(a.get(&spec.field), b.get(&spec.field), spec.descending);
            if cmp != Ordering::Equal {
                return cmp;
            }
        }
        Ordering::Equal
    });
}

pub fn run(args: &QueryArgs, config: &Config) -> Result<()> {
    let expr = parse_query(&args.expression)?;
    let repo = MarkdownRepository::new(config.vault_path.clone());
    let all = repo.list_all(&config.registry)?;

    let mut matches: Vec<_> = all.iter().filter(|e| evaluate(&expr, e)).collect();

    if let Some(sort_by) = &args.sort_by {
        let specs = parse_sort_by(sort_by)?;
        sort_entities(&mut matches, &specs);
    }

    if args.format == "json" {
        let items: Vec<serde_json::Value> = matches
            .iter()
            .map(|e| {
                let mut map = serde_json::Map::new();
                map.insert("id".into(), serde_json::Value::String(e.id.clone()));
                for (k, v) in &e.frontmatter {
                    map.insert(k.clone(), serde_json::Value::String(v.to_string()));
                }
                serde_json::Value::Object(map)
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_default()
        );
    } else {
        // Print result count with sort indication
        if let Some(sort_by) = &args.sort_by {
            println!("Found {} results (sorted by {}):\n", matches.len(), sort_by);
        } else {
            println!("Found {} results:\n", matches.len());
        }

        for entity in &matches {
            println!(
                "- [{}] {} ({})",
                entity.entity_type,
                entity.title(),
                entity.id
            );
            let mut fields = Vec::new();
            if let Some(status) = entity.get("status") {
                fields.push(format!("status: {status}"));
            }
            if let Some(due) = entity.get("due") {
                fields.push(format!("due: {due}"));
            }
            if !fields.is_empty() {
                println!("  {}", fields.join(", "));
            }
        }
    }

    Ok(())
}
