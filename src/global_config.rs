use crate::error::{CortxError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct VaultEntry {
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    pub default: Option<String>,
    #[serde(default)]
    pub vaults: HashMap<String, VaultEntry>,
}

fn global_config_path() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| CortxError::Storage("HOME environment variable not set".into()))?;
    Ok(PathBuf::from(home).join(".cortx").join("config.toml"))
}

impl GlobalConfig {
    pub fn load() -> Result<Self> {
        let path = global_config_path()?;
        if !path.exists() {
            return Ok(GlobalConfig::default());
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content)
            .map_err(|e| CortxError::Storage(format!("failed to parse global config: {e}")))
    }

    pub fn save(&self) -> Result<()> {
        let path = global_config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)
            .map_err(|e| CortxError::Storage(format!("failed to serialize global config: {e}")))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn register_vault(&mut self, name: &str, path: PathBuf) -> Result<()> {
        if self.vaults.contains_key(name) {
            return Err(CortxError::Storage(format!(
                "vault name '{name}' is already registered"
            )));
        }
        self.vaults.insert(name.to_string(), VaultEntry { path });
        Ok(())
    }

    pub fn resolve_path(&self, name: Option<&str>) -> Option<PathBuf> {
        match name {
            Some(n) => self.vaults.get(n).map(|e| e.path.clone()),
            None => {
                let default_name = self.default.as_deref()?;
                self.vaults.get(default_name).map(|e| e.path.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_file_returns_empty() {
        // load() from a nonexistent path should return empty GlobalConfig
        // We test this by directly deserializing empty TOML
        let cfg: GlobalConfig = toml::from_str("").unwrap();
        assert!(cfg.default.is_none());
        assert!(cfg.vaults.is_empty());
    }

    #[test]
    fn register_vault_adds_entry() {
        let mut cfg = GlobalConfig::default();
        cfg.register_vault("work", PathBuf::from("/tmp/work"))
            .unwrap();
        assert!(cfg.vaults.contains_key("work"));
        assert_eq!(cfg.vaults["work"].path, PathBuf::from("/tmp/work"));
    }

    #[test]
    fn register_vault_duplicate_name_errors() {
        let mut cfg = GlobalConfig::default();
        cfg.register_vault("work", PathBuf::from("/tmp/work"))
            .unwrap();
        let result = cfg.register_vault("work", PathBuf::from("/tmp/other"));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("work"), "error should mention vault name");
    }

    #[test]
    fn resolve_path_returns_default_vault() {
        let mut cfg = GlobalConfig::default();
        cfg.vaults.insert(
            "personal".into(),
            VaultEntry {
                path: PathBuf::from("/tmp/personal"),
            },
        );
        cfg.default = Some("personal".into());
        assert_eq!(cfg.resolve_path(None), Some(PathBuf::from("/tmp/personal")));
    }

    #[test]
    fn resolve_path_by_name() {
        let mut cfg = GlobalConfig::default();
        cfg.vaults.insert(
            "work".into(),
            VaultEntry {
                path: PathBuf::from("/tmp/work"),
            },
        );
        cfg.default = Some("work".into());
        // named lookup ignores default
        cfg.vaults.insert(
            "personal".into(),
            VaultEntry {
                path: PathBuf::from("/tmp/personal"),
            },
        );
        assert_eq!(
            cfg.resolve_path(Some("personal")),
            Some(PathBuf::from("/tmp/personal"))
        );
    }

    #[test]
    fn resolve_path_missing_name_returns_none() {
        let mut cfg = GlobalConfig::default();
        cfg.vaults.insert(
            "work".into(),
            VaultEntry {
                path: PathBuf::from("/tmp/work"),
            },
        );
        // no default set
        assert_eq!(cfg.resolve_path(None), None);
    }

    #[test]
    fn resolve_path_unknown_name_returns_none() {
        let cfg = GlobalConfig::default();
        assert_eq!(cfg.resolve_path(Some("ghost")), None);
    }

    #[test]
    fn save_and_reload_roundtrip() {
        // serialize to TOML string and deserialize back
        let mut cfg = GlobalConfig::default();
        cfg.register_vault("personal", PathBuf::from("/tmp/personal"))
            .unwrap();
        cfg.default = Some("personal".into());
        let serialized = toml::to_string(&cfg).unwrap();
        let reloaded: GlobalConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(reloaded.default, Some("personal".into()));
        assert_eq!(
            reloaded.vaults["personal"].path,
            PathBuf::from("/tmp/personal")
        );
    }
}
