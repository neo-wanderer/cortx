use crate::config::Config;
use crate::error::Result;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use clap::Args;

#[derive(Args)]
pub struct ShowArgs {
    pub id: String,
}

pub fn run(args: &ShowArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());
    let entity = repo.get_by_id(&args.id, &config.registry)?;

    println!("ID: {}", entity.id);
    println!("Type: {}", entity.entity_type);
    println!("Title: {}", entity.title());

    let mut keys: Vec<&String> = entity.frontmatter.keys().collect();
    keys.sort();
    for key in keys {
        if key == "id" || key == "type" || key == "title" || key == "name" {
            continue;
        }
        println!("  {key}: {}", entity.frontmatter[key]);
    }

    if !entity.body.trim().is_empty() {
        println!("\n{}", entity.body);
    }

    Ok(())
}
