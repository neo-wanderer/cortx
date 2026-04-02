use crate::error::Result;
use clap::Args;
use std::fs;
use std::path::Path;

#[derive(Args)]
pub struct InitArgs {
    /// Path to create the vault in (defaults to current directory)
    pub path: Option<String>,
}

pub fn run(args: &InitArgs) -> Result<()> {
    let vault_path = args
        .path
        .as_ref()
        .map(|p| Path::new(p).to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let folders = [
        "0_Inbox",
        "1_Projects",
        "1_Projects/tasks",
        "2_Areas",
        "3_Resources",
        "3_Resources/notes",
        "4_Archive",
        "5_People",
        "5_Companies",
    ];

    for folder in &folders {
        fs::create_dir_all(vault_path.join(folder))?;
    }

    let types_dest = vault_path.join("types.yaml");
    if !types_dest.exists() {
        let default_types = include_str!("../../types.yaml");
        fs::write(&types_dest, default_types)?;
    }

    println!("Initialized cortx vault at {}", vault_path.display());
    println!("Created folders:");
    for folder in &folders {
        println!("  {folder}/");
    }
    println!("  types.yaml");

    Ok(())
}
