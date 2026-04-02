use crate::config::Config;
use crate::error::Result;
use crate::storage::Repository;
use crate::storage::markdown::MarkdownRepository;
use clap::Args;

#[derive(Args)]
pub struct DeleteArgs {
    pub id: String,

    #[arg(long)]
    pub force: bool,
}

pub fn run(args: &DeleteArgs, config: &Config) -> Result<()> {
    let repo = MarkdownRepository::new(config.vault_path.clone());
    repo.delete(&args.id, &config.registry)?;
    println!("Deleted {}", args.id);

    Ok(())
}
