use crate::config::Config;
use crate::error::Result;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use crate::value::Value;
use clap::Args;
use std::collections::HashMap;

#[derive(Args)]
pub struct ArchiveArgs {
    pub id: String,
}

pub fn run(args: &ArchiveArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());

    let mut updates = HashMap::new();
    updates.insert("status".into(), Value::String("archived".into()));

    let entity = repo.update(&args.id, updates, &config.registry)?;
    println!("Archived {}", entity.id);

    Ok(())
}
