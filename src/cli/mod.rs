pub mod archive;
pub mod create;
pub mod delete;
pub mod doctor;
pub mod init;
pub mod meta;
pub mod note;
pub mod query_cmd;
pub mod show;
pub mod update;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cortx",
    version,
    about = "Second Brain CLI for agents and humans"
)]
pub struct Cli {
    /// Path to the vault directory (overrides all other sources)
    #[arg(long, global = true)]
    pub vault: Option<String>,

    /// Name of a registered vault from ~/.cortx/config.toml
    #[arg(long, global = true)]
    pub vault_name: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new vault
    Init(init::InitArgs),
    /// Create a new entity
    Create(create::CreateArgs),
    /// Show an entity by ID
    Show(show::ShowArgs),
    /// Update entity fields
    Update(update::UpdateArgs),
    /// Archive an entity (soft delete)
    Archive(archive::ArchiveArgs),
    /// Delete an entity
    Delete(delete::DeleteArgs),
    /// Run a query against the vault
    Query(query_cmd::QueryArgs),
    /// Metadata aggregations (distinct, count-by)
    Meta(meta::MetaArgs),
    /// Note editing commands
    Note(note::NoteArgs),
    /// Doctor / lint commands
    Doctor(doctor::DoctorArgs),
}
