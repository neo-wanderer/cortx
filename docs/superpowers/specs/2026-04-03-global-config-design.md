# Global Config & Multi-Vault Support

**Date:** 2026-04-03

## Summary

Add a persistent global config file at `~/.cortx/config.toml` that stores named vault paths. Commands resolve the active vault through a priority chain ending at this config. The `init` command registers vaults into the config and guards against re-initializing an already-initialized vault.

---

## Config File Structure

Location: `~/.cortx/config.toml`

```toml
default = "personal"

[vaults.personal]
path = "/Users/vignesh/vault"

[vaults.work]
path = "/Users/vignesh/work-vault"
```

- `default` names the active vault. Optional — if absent, resolution falls through to cwd.
- Each vault is a named entry under `[vaults.*]` with a `path` field.
- File is created on first `cortx init --name <name>`, updated on subsequent named inits.

---

## New Module: `src/global_config.rs`

```rust
pub struct VaultEntry {
    pub path: PathBuf,
}

pub struct GlobalConfig {
    pub default: Option<String>,
    pub vaults: HashMap<String, VaultEntry>,
}

impl GlobalConfig {
    pub fn load() -> Result<Self>
    pub fn save(&self) -> Result<()>
    pub fn register_vault(&mut self, name: &str, path: PathBuf) -> Result<()>
    pub fn resolve_path(&self, name: Option<&str>) -> Option<PathBuf>
}
```

**Behavior:**
- `load()` returns an empty `GlobalConfig` (not an error) if the file does not exist — first `init` creates it.
- `save()` creates `~/.cortx/` if it does not exist, then writes the TOML file.
- `register_vault` errors if the vault name is already registered.
- `resolve_path(None)` returns the default vault path; `resolve_path(Some("work"))` looks up by name.

---

## CLI Changes

### `src/cli/mod.rs`

Add `--vault-name` as a global flag alongside the existing `--vault`:

```rust
pub struct Cli {
    #[arg(long, global = true)]
    pub vault: Option<String>,       // existing: explicit path

    #[arg(long, global = true)]
    pub vault_name: Option<String>,  // new: named vault lookup

    #[command(subcommand)]
    pub command: Commands,
}
```

### `src/config.rs`

Update `Config::load` signature:

```rust
pub fn load(vault_path: Option<&str>, vault_name: Option<&str>) -> Result<Self>
```

**Resolution order:**
1. `--vault <path>` — use directly, skip all other steps
2. `--vault-name <name>` — look up path in `GlobalConfig`
3. `CORTX_VAULT` env var — use directly
4. Default vault from `GlobalConfig` (if `default` key is set)
5. Current working directory — last resort

### `src/cli/init.rs`

Add `--name` flag to `InitArgs`:

```rust
pub struct InitArgs {
    pub path: Option<String>,
    #[arg(long)]
    pub name: Option<String>,  // register this vault under a name in global config
}
```

**Init behavior:**
1. Resolve target path (arg or cwd).
2. If `types.yaml` already exists at target path → error: `"vault already initialized at <path>"`.
3. Create vault folder structure and copy `types.yaml`.
4. If `--name` is provided:
   - Call `GlobalConfig::load()` (empty if missing).
   - Call `register_vault(name, path)` — errors if name already taken.
   - If this is the first vault in the config, set it as `default` automatically.
   - Call `save()`.
5. If `--name` is omitted, vault is created but not registered (safe — no config side effects).

---

## Error Cases

| Scenario | Error message |
|---|---|
| `--vault-name foo` and `foo` not in config | `"vault 'foo' not found in global config"` |
| `cortx init --name work` and `work` already registered | `"vault name 'work' is already registered"` |
| `cortx init` on already-initialized path | `"vault already initialized at <path>"` |
| `~/.cortx/config.toml` is malformed TOML | `"failed to parse global config: <toml error>"` |

---

## Testing

- Unit tests for `GlobalConfig`: load from missing file, load/save roundtrip, register duplicate name, resolve default, resolve by name.
- Integration tests for `init`: duplicate init fails, `--name` registers in config, first named vault becomes default.
- Integration tests for resolution order: `--vault` overrides config default, `--vault-name` resolves correctly.
- Existing tests must continue to pass without modification (cwd fallback still works).

---

## Out of Scope

- `cortx vault list` / `cortx vault remove` commands (future work).
- Changing the default vault via CLI (future: `cortx vault use <name>`).
- Migrating or validating stale paths in config.
