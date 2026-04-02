use clap::Args;
use crate::config::Config;
use crate::error::Result;
use crate::query::evaluator::evaluate;
use crate::query::parser::parse_query;
use crate::storage::markdown::MarkdownRepository;
use crate::storage::Repository;

#[derive(Args)]
pub struct QueryArgs {
    pub expression: String,

    #[arg(long, default_value = "text")]
    pub format: String,
}

pub fn run(args: &QueryArgs, config: &Config) -> Result<()> {
    let expr = parse_query(&args.expression)?;
    let repo = MarkdownRepository::new(config.vault_path.clone());
    let all = repo.list_all(&config.registry)?;

    let matches: Vec<_> = all.iter().filter(|e| evaluate(&expr, e)).collect();

    if args.format == "json" {
        let items: Vec<serde_json::Value> = matches
            .iter()
            .map(|e| {
                let mut map = serde_json::Map::new();
                for (k, v) in &e.frontmatter {
                    map.insert(k.clone(), serde_json::Value::String(v.to_string()));
                }
                serde_json::Value::Object(map)
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items).unwrap_or_default());
    } else {
        println!("Found {} results:\n", matches.len());
        for entity in &matches {
            println!("- [{}] {} ({})", entity.entity_type, entity.title(), entity.id);
            if let Some(status) = entity.get("status") {
                print!("  status: {status}");
            }
            if let Some(due) = entity.get("due") {
                print!("  due: {due}");
            }
            println!();
        }
    }

    Ok(())
}
