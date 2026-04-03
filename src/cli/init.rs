use crate::error::{CortxError, Result};
use crate::global_config::GlobalConfig;
use clap::Args;
use std::fs;
use std::path::Path;

#[derive(Args)]
pub struct InitArgs {
    /// Path to create the vault in (defaults to current directory)
    pub path: Option<String>,

    /// Register this vault under a name in the global config (~/.cortx/config.toml)
    #[arg(long)]
    pub name: Option<String>,
}

pub fn run(args: &InitArgs) -> Result<()> {
    let vault_path = args
        .path
        .as_ref()
        .map(|p| Path::new(p).to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    // Guard: fail if already initialized
    if vault_path.join("types.yaml").exists() {
        return Err(CortxError::Storage(format!(
            "vault already initialized at {}",
            vault_path.display()
        )));
    }

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
    let default_types = include_str!("../../types.yaml");
    fs::write(&types_dest, default_types)?;

    println!("Initialized cortx vault at {}", vault_path.display());
    println!("Created folders:");
    for folder in &folders {
        println!("  {folder}/");
    }
    println!("  types.yaml");

    // Register in global config if --name was provided
    if let Some(name) = &args.name {
        let mut global = GlobalConfig::load()?;
        global.register_vault(name, vault_path.canonicalize()?)?;
        if global.vaults.len() == 1 {
            global.default = Some(name.clone());
            println!("Set '{name}' as the default vault.");
        }
        global.save()?;
        println!("Registered vault '{name}' in ~/.cortx/config.toml");
    }

    Ok(())
}
