use crate::error::{CortxError, Result};
use crate::global_config::GlobalConfig;
use crate::schema::registry::TypeRegistry;
use std::path::{Path, PathBuf};

pub struct Config {
    pub vault_path: PathBuf,
    pub registry: TypeRegistry,
}

impl Config {
    pub fn load(vault_path: Option<&str>, vault_name: Option<&str>) -> Result<Self> {
        let vault_path = if let Some(p) = vault_path {
            // 1. Explicit --vault path
            PathBuf::from(p)
        } else if let Some(name) = vault_name {
            // 2. --vault-name: named lookup in global config
            let global = GlobalConfig::load()?;
            global.resolve_path(Some(name)).ok_or_else(|| {
                CortxError::Storage(format!("vault '{name}' not found in global config"))
            })?
        } else if let Ok(p) = std::env::var("CORTX_VAULT") {
            // 3. CORTX_VAULT env var
            PathBuf::from(p)
        } else if let Some(p) = GlobalConfig::load()?.resolve_path(None) {
            // 4. Default vault from global config
            p
        } else {
            // 5. Current working directory
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
