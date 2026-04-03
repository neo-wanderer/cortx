# Global Config & Multi-Vault Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `~/.cortx/config.toml` as a persistent store for named vault paths, with `cortx init --name <name>` to register vaults and `--vault-name <name>` to select them.

**Architecture:** A new `GlobalConfig` struct in `src/global_config.rs` owns all reads/writes to `~/.cortx/config.toml`. `Config::load` gains a `vault_name` parameter and consults `GlobalConfig` in its resolution chain. `init` gains a `--name` flag and guards against re-initializing an already-initialized vault.

**Tech Stack:** Rust, clap 4 (derive), serde + `toml` crate for config file I/O, thiserror for errors.

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Modify | `Cargo.toml` | Add `toml` dependency |
| Create | `src/global_config.rs` | `GlobalConfig` struct, load/save/register/resolve |
| Modify | `src/lib.rs` | Expose `global_config` module |
| Modify | `src/main.rs` | Expose `global_config` module, update `Config::load` call |
| Modify | `src/cli/mod.rs` | Add `--vault-name` global flag |
| Modify | `src/config.rs` | Update `load` signature, implement 5-step resolution order |
| Modify | `src/cli/init.rs` | Add `--name` flag, duplicate-init guard, vault registration |
| Modify | `tests/cli_integration_test.rs` | Integration tests for new behaviors |

---

### Task 1: Add `toml` crate dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, add to `[dependencies]`:
```toml
toml = { version = "0.8", features = ["parse"] }
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1 | head -20
```
Expected: no errors (warnings about unused are fine).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add toml dependency for global config"
```

---

### Task 2: Create `src/global_config.rs` with unit tests

**Files:**
- Create: `src/global_config.rs`

- [ ] **Step 1: Write failing unit tests first**

Create `src/global_config.rs` with the tests block only (no implementation yet):

```rust
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
    todo!()
}

impl GlobalConfig {
    pub fn load() -> Result<Self> {
        todo!()
    }

    pub fn save(&self) -> Result<()> {
        todo!()
    }

    pub fn register_vault(&mut self, name: &str, path: PathBuf) -> Result<()> {
        todo!()
    }

    pub fn resolve_path(&self, name: Option<&str>) -> Option<PathBuf> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn config_in(dir: &TempDir) -> GlobalConfig {
        GlobalConfig::default()
    }

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
        cfg.register_vault("work", PathBuf::from("/tmp/work")).unwrap();
        assert!(cfg.vaults.contains_key("work"));
        assert_eq!(cfg.vaults["work"].path, PathBuf::from("/tmp/work"));
    }

    #[test]
    fn register_vault_duplicate_name_errors() {
        let mut cfg = GlobalConfig::default();
        cfg.register_vault("work", PathBuf::from("/tmp/work")).unwrap();
        let result = cfg.register_vault("work", PathBuf::from("/tmp/other"));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("work"), "error should mention vault name");
    }

    #[test]
    fn resolve_path_returns_default_vault() {
        let mut cfg = GlobalConfig::default();
        cfg.vaults.insert("personal".into(), VaultEntry { path: PathBuf::from("/tmp/personal") });
        cfg.default = Some("personal".into());
        assert_eq!(cfg.resolve_path(None), Some(PathBuf::from("/tmp/personal")));
    }

    #[test]
    fn resolve_path_by_name() {
        let mut cfg = GlobalConfig::default();
        cfg.vaults.insert("work".into(), VaultEntry { path: PathBuf::from("/tmp/work") });
        cfg.default = Some("work".into());
        // named lookup ignores default
        cfg.vaults.insert("personal".into(), VaultEntry { path: PathBuf::from("/tmp/personal") });
        assert_eq!(cfg.resolve_path(Some("personal")), Some(PathBuf::from("/tmp/personal")));
    }

    #[test]
    fn resolve_path_missing_name_returns_none() {
        let mut cfg = GlobalConfig::default();
        cfg.vaults.insert("work".into(), VaultEntry { path: PathBuf::from("/tmp/work") });
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
        cfg.register_vault("personal", PathBuf::from("/tmp/personal")).unwrap();
        cfg.default = Some("personal".into());
        let serialized = toml::to_string(&cfg).unwrap();
        let reloaded: GlobalConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(reloaded.default, Some("personal".into()));
        assert_eq!(reloaded.vaults["personal"].path, PathBuf::from("/tmp/personal"));
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail (todo! panics)**

```bash
cargo test --lib global_config 2>&1 | tail -20
```
Expected: multiple test failures with `not yet implemented`.

- [ ] **Step 3: Implement `global_config_path`, `load`, `save`, `register_vault`, `resolve_path`**

Replace the `todo!()` bodies with full implementations:

```rust
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
```

- [ ] **Step 4: Run unit tests — all should pass**

```bash
cargo test --lib global_config 2>&1 | tail -20
```
Expected: `test result: ok. 7 passed; 0 failed`.

- [ ] **Step 5: Expose the module in `src/lib.rs` and `src/main.rs`**

In `src/lib.rs`, add after `pub mod config;`:
```rust
pub mod global_config;
```

In `src/main.rs`, add after `pub mod config;`:
```rust
pub mod global_config;
```

- [ ] **Step 6: Confirm build clean**

```bash
cargo build 2>&1 | grep -E "^error"
```
Expected: no output (no errors).

- [ ] **Step 7: Commit**

```bash
git add src/global_config.rs src/lib.rs src/main.rs
git commit -m "feat: add GlobalConfig for persistent vault registry"
```

---

### Task 3: Add `--vault-name` CLI flag and update `Config::load` resolution order

**Files:**
- Modify: `src/cli/mod.rs`
- Modify: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add `--vault-name` to `Cli` struct in `src/cli/mod.rs`**

Replace the existing `Cli` struct:
```rust
#[derive(Parser)]
#[command(
    name = "cortx",
    version,
    about = "Second Brain CLI for agents and humans"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub vault: Option<String>,

    #[arg(long, global = true)]
    pub vault_name: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}
```

- [ ] **Step 2: Update `Config::load` in `src/config.rs`**

Replace the full file contents:
```rust
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
```

- [ ] **Step 3: Update the `Config::load` call in `src/main.rs`**

Find the line:
```rust
let config = match Config::load(cli.vault.as_deref()) {
```
Replace it with:
```rust
let config = match Config::load(cli.vault.as_deref(), cli.vault_name.as_deref()) {
```

- [ ] **Step 4: Verify the build**

```bash
cargo build 2>&1 | grep -E "^error"
```
Expected: no output.

- [ ] **Step 5: Run existing tests to confirm nothing broke**

```bash
cargo test --test cli_integration_test 2>&1 | tail -10
```
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/cli/mod.rs src/config.rs src/main.rs
git commit -m "feat: add --vault-name flag and 5-step vault resolution order"
```

---

### Task 4: Update `init` — duplicate guard and `--name` registration

**Files:**
- Modify: `src/cli/init.rs`

- [ ] **Step 1: Replace `src/cli/init.rs` with the updated implementation**

```rust
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
```

- [ ] **Step 2: Verify build**

```bash
cargo build 2>&1 | grep -E "^error"
```
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add src/cli/init.rs
git commit -m "feat: guard init against re-init and support --name vault registration"
```

---

### Task 5: Integration tests for new behaviors

**Files:**
- Modify: `tests/cli_integration_test.rs`

- [ ] **Step 1: Write tests for duplicate init guard**

Add the following tests to the bottom of `tests/cli_integration_test.rs`:

```rust
#[test]
fn test_init_fails_if_already_initialized() {
    let dir = TempDir::new().unwrap();
    // First init succeeds
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", dir.path().to_str().unwrap()])
        .assert()
        .success();
    // Second init on same path fails
    Command::cargo_bin("cortx")
        .unwrap()
        .args(["init", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("vault already initialized at"));
}

#[test]
fn test_init_with_name_registers_in_global_config() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    // Use a temp HOME so we don't pollute the real ~/.cortx/config.toml
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args(["init", vault_dir.path().to_str().unwrap(), "--name", "testonly"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered vault 'testonly'"));
    // Config file must exist
    assert!(dir.path().join(".cortx").join("config.toml").exists());
    let config_content =
        fs::read_to_string(dir.path().join(".cortx").join("config.toml")).unwrap();
    assert!(config_content.contains("testonly"));
}

#[test]
fn test_init_first_named_vault_becomes_default() {
    let dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args(["init", vault_dir.path().to_str().unwrap(), "--name", "myvault"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set 'myvault' as the default vault."));
    let config_content =
        fs::read_to_string(dir.path().join(".cortx").join("config.toml")).unwrap();
    assert!(config_content.contains("default = \"myvault\""));
}

#[test]
fn test_init_duplicate_name_errors() {
    let dir = TempDir::new().unwrap();
    let vault1 = TempDir::new().unwrap();
    let vault2 = TempDir::new().unwrap();
    // First registration succeeds
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args(["init", vault1.path().to_str().unwrap(), "--name", "shared"])
        .assert()
        .success();
    // Second registration with same name fails
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", dir.path())
        .args(["init", vault2.path().to_str().unwrap(), "--name", "shared"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("vault name 'shared' is already registered"));
}

#[test]
fn test_vault_name_flag_resolves_correct_vault() {
    let home_dir = TempDir::new().unwrap();
    let vault_dir = TempDir::new().unwrap();
    // Init and register a named vault
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", home_dir.path())
        .args(["init", vault_dir.path().to_str().unwrap(), "--name", "mywork"])
        .assert()
        .success();
    // Create an entity using --vault-name
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", home_dir.path())
        .args([
            "--vault-name", "mywork",
            "create", "task",
            "--title", "Named vault task",
            "--id", "task-named-vault",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task-named-vault"));
    // Verify the file exists in the named vault
    assert!(vault_dir.path().join("1_Projects/tasks/task-named-vault.md").exists());
}

#[test]
fn test_vault_name_unknown_errors() {
    let home_dir = TempDir::new().unwrap();
    Command::cargo_bin("cortx")
        .unwrap()
        .env("HOME", home_dir.path())
        .args(["--vault-name", "ghost", "query", "type = \"task\""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("vault 'ghost' not found in global config"));
}
```

- [ ] **Step 2: Run the new integration tests**

```bash
cargo test --test cli_integration_test test_init_fails 2>&1 | tail -10
cargo test --test cli_integration_test test_init_with_name 2>&1 | tail -10
cargo test --test cli_integration_test test_init_first_named 2>&1 | tail -10
cargo test --test cli_integration_test test_init_duplicate_name 2>&1 | tail -10
cargo test --test cli_integration_test test_vault_name_flag 2>&1 | tail -10
cargo test --test cli_integration_test test_vault_name_unknown 2>&1 | tail -10
```
Expected: all 6 pass.

- [ ] **Step 3: Run the full test suite to confirm no regressions**

```bash
cargo test 2>&1 | tail -15
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/cli_integration_test.rs
git commit -m "test: integration tests for global config and vault name resolution"
```

---

### Task 6: Final verification

- [ ] **Step 1: Run clippy**

```bash
cargo clippy -- -W clippy::all 2>&1 | grep -E "^error"
```
Expected: no errors.

- [ ] **Step 2: Run full test suite one more time**

```bash
cargo test 2>&1 | tail -5
```
Expected: all tests pass, 0 failures.

- [ ] **Step 3: Smoke test the full flow manually**

```bash
# Create a temp vault and init with a name
TMPVAULT=$(mktemp -d)
cargo run -- init "$TMPVAULT" --name smoketest
# Verify config file was created
cat ~/.cortx/config.toml
# Use the named vault
cargo run -- --vault-name smoketest create task --title "Smoke test task" --id task-smoke
# Verify entity created in the temp vault
ls "$TMPVAULT/1_Projects/tasks/"
# Clean up
rm -rf "$TMPVAULT"
```
Expected: config.toml shows `smoketest` vault, entity file appears in temp vault.
