use crate::config::Config;
use crate::error::Result;
use crate::query::evaluator::evaluate;
use crate::query::parser::parse_query;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::{Args, Subcommand};
use std::collections::HashMap;

#[derive(Args)]
pub struct MetaArgs {
    #[command(subcommand)]
    pub command: MetaCommands,
}

#[derive(Subcommand)]
pub enum MetaCommands {
    /// Get distinct values for a field
    Distinct {
        field: String,
        #[arg(long = "where")]
        filter: Option<String>,
    },
    /// Count entities grouped by a field
    CountBy {
        field: String,
        #[arg(long = "where")]
        filter: Option<String>,
    },
}

pub fn run(args: &MetaArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());
    let all = repo.list_all(&config.registry)?;

    match &args.command {
        MetaCommands::Distinct { field, filter } => {
            let filtered = apply_filter(all, filter.as_deref())?;
            let mut unique = std::collections::BTreeSet::new();

            for entity in &filtered {
                if let Some(val) = entity.get(field) {
                    match val {
                        Value::Array(items) => {
                            for item in items {
                                unique.insert(item.to_string());
                            }
                        }
                        other => {
                            unique.insert(other.to_string());
                        }
                    }
                }
            }

            println!("Distinct values for '{field}' ({} values):\n", unique.len());
            for v in &unique {
                println!("  {v}");
            }
        }
        MetaCommands::CountBy { field, filter } => {
            let filtered = apply_filter(all, filter.as_deref())?;
            let mut counts: HashMap<String, usize> = HashMap::new();

            for entity in &filtered {
                if let Some(val) = entity.get(field) {
                    match val {
                        Value::Array(items) => {
                            for item in items {
                                *counts.entry(item.to_string()).or_default() += 1;
                            }
                        }
                        other => {
                            *counts.entry(other.to_string()).or_default() += 1;
                        }
                    }
                }
            }

            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            println!("Count by '{field}':\n");
            for (val, count) in &sorted {
                println!("  {val}: {count}");
            }
        }
    }

    Ok(())
}

fn apply_filter(
    entities: Vec<crate::entity::Entity>,
    filter: Option<&str>,
) -> Result<Vec<crate::entity::Entity>> {
    match filter {
        Some(expr_str) => {
            let expr = parse_query(expr_str)?;
            Ok(entities
                .into_iter()
                .filter(|e| evaluate(&expr, e))
                .collect())
        }
        None => Ok(entities),
    }
}
