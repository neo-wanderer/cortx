pub mod create;
pub mod show;
pub mod update;
pub mod archive;
pub mod delete;
pub mod query_cmd;
pub mod meta;
pub mod note;
pub mod doctor;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cortx", version, about = "Second Brain CLI for agents and humans")]
pub struct Cli {
    #[arg(long, global = true)]
    pub vault: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
