use std::path::{Path, PathBuf};
use crate::error::{CortxError, Result};
use crate::schema::registry::TypeRegistry;

pub struct Config {
    pub vault_path: PathBuf,
    pub registry: TypeRegistry,
}

impl Config {
    pub fn load(vault_path: Option<&str>) -> Result<Self> {
        let vault_path = if let Some(p) = vault_path {
            PathBuf::from(p)
        } else if let Ok(p) = std::env::var("CORTX_VAULT") {
            PathBuf::from(p)
        } else {
            std::env::current_dir()?
        };

        if !vault_path.exists() {
            return Err(CortxError::Storage(format!(
                "vault path does not exist: {}",
                vault_path.display()
            )));
        }

        let types_path = find_types_yaml(&vault_path)?;
        let registry = TypeRegistry::from_yaml_file(&types_path)?;

        Ok(Config {
            vault_path,
            registry,
        })
    }
}

fn find_types_yaml(vault_path: &Path) -> Result<PathBuf> {
    let in_vault = vault_path.join("types.yaml");
    if in_vault.exists() {
        return Ok(in_vault);
    }

    let in_cwd = PathBuf::from("types.yaml");
    if in_cwd.exists() {
        return Ok(in_cwd);
    }

    Err(CortxError::Schema(
        "types.yaml not found in vault directory or current directory".into(),
    ))
}
