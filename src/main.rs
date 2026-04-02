pub mod cli;
pub mod config;
pub mod entity;
pub mod error;
pub mod frontmatter;
pub mod query;
pub mod schema;
pub mod storage;
pub mod value;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

fn main() {
    let cli = Cli::parse();

    let config = match Config::load(cli.vault.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {e}");
            std::process::exit(1);
        }
    };

    let result = match &cli.command {
        Commands::Create(args) => cli::create::run(args, &config),
        Commands::Show(args) => cli::show::run(args, &config),
        Commands::Update(args) => cli::update::run(args, &config),
        Commands::Archive(args) => cli::archive::run(args, &config),
        Commands::Delete(args) => cli::delete::run(args, &config),
        Commands::Query(args) => cli::query_cmd::run(args, &config),
        Commands::Meta(args) => cli::meta::run(args, &config),
        Commands::Note(args) => cli::note::run(args, &config),
        Commands::Doctor(args) => cli::doctor::run(args, &config),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
