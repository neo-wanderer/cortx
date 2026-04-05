use crate::config::Config;
use crate::error::Result;
use crate::storage::markdown::MarkdownRepository;
use crate::storage::Repository;
use clap::Args;
use std::collections::HashMap;

#[derive(Args)]
pub struct UpdateArgs {
    pub id: String,

    #[arg(long = "set", num_args = 1, required = true)]
    pub updates: Vec<String>,

    /// Skip link-target existence validation (for bulk imports)
    #[arg(long)]
    pub no_validate_links: bool,
}

pub fn run(args: &UpdateArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone())
        .with_link_validation(!args.no_validate_links);

    let mut updates = HashMap::new();
    for kv in &args.updates {
        if let Some((k, v)) = kv.split_once('=') {
            if k == "title" {
                return Err(crate::error::CortxError::Validation(
                    "use 'cortx rename' to change an entity's title".into(),
                ));
            }
            let value = super::create::parse_cli_value(v);
            updates.insert(k.to_string(), value);
        }
    }

    let entity = repo.update(&args.id, updates, &config.registry)?;
    println!("Updated {}", entity.id);

    Ok(())
}
