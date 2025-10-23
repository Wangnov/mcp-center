//! Filesystem layout helpers for mcp-center.

use std::{
    cmp::Ordering,
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{config::ServerConfig, error::CoreError};

/// Environment variable that overrides the default root directory.
const ROOT_ENV_KEY: &str = "MCP_CENTER_ROOT";
const DEFAULT_ROOT_DIRNAME: &str = ".mcp-center";

/// Descriptor for the on-disk directory structure.
#[derive(Clone, Debug)]
pub struct Layout {
    root: PathBuf,
    config_dir: PathBuf,
    servers_dir: PathBuf,
    logs_dir: PathBuf,
    state_dir: PathBuf,
    projects_dir: PathBuf,
}

impl Layout {
    /// Construct a new layout without touching the filesystem.
    pub fn new(root: PathBuf) -> Self {
        let config_dir = root.join("config");
        let servers_dir = config_dir.join("servers");
        let logs_dir = root.join("logs");
        let state_dir = root.join("state");
        let projects_dir = root.join("projects");

        Self { root, config_dir, servers_dir, logs_dir, state_dir, projects_dir }
    }

    /// Ensure that all directories exist on disk.
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            self.root(),
            self.config_dir(),
            self.servers_dir(),
            self.logs_dir(),
            self.state_dir(),
            self.projects_dir(),
        ] {
            if !dir.exists() {
                fs::create_dir_all(dir).map_err(|source| CoreError::CreateDirectory {
                    path: dir.to_path_buf(),
                    source,
                })?;
            }
        }
        Ok(())
    }

    /// Root directory path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Top-level config directory.
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Directory where individual MCP server definitions live.
    pub fn servers_dir(&self) -> &Path {
        &self.servers_dir
    }

    /// Directory that stores aggregated logs.
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// Directory that stores runtime state (pid files, sockets, etc).
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    /// Directory where project registry files are stored.
    pub fn projects_dir(&self) -> &Path {
        &self.projects_dir
    }

    /// Path to a config file by server id.
    pub fn server_config_path(&self, id: &str) -> PathBuf {
        self.servers_dir().join(format!("{id}.toml"))
    }

    /// Path to a log file by server id.
    pub fn server_log_path(&self, id: &str) -> PathBuf {
        self.logs_dir().join(format!("{id}.log"))
    }

    /// Path to a pid file by server id.
    pub fn server_pid_path(&self, id: &str) -> PathBuf {
        self.state_dir().join(format!("{id}.pid"))
    }

    /// Path to the daemon control socket file.
    pub fn daemon_socket_path(&self) -> PathBuf {
        self.state_dir().join("daemon.sock")
    }

    /// Path to the RPC socket file (daemon CLI interface).
    pub fn daemon_rpc_socket_path(&self) -> PathBuf {
        self.state_dir().join("daemon.rpc.sock")
    }

    /// Path to the daemon lock file (prevents concurrent startup).
    pub fn daemon_lock_path(&self) -> PathBuf {
        self.state_dir().join("daemon.lock")
    }

    /// Path to a project configuration file by id.
    pub fn project_config_path(&self, id: &str) -> PathBuf {
        self.projects_dir().join(format!("{id}.toml"))
    }

    /// Return the canonical configuration path (TOML) for a server.
    pub fn server_config_toml_path(&self, id: &str) -> PathBuf {
        self.servers_dir().join(format!("{id}.toml"))
    }

    /// Load a server configuration by id.
    pub fn load_server_config(&self, id: &str) -> Result<ServerConfig> {
        for candidate in self.server_config_candidates(id) {
            if candidate.exists() {
                return ServerConfig::from_file(&candidate);
            }
        }
        Err(CoreError::ServerConfigNotFound { id: id.to_string() }.into())
    }

    /// List all server configurations available in the workspace.
    pub fn list_server_configs(&self) -> Result<Vec<ServerConfig>> {
        let mut configs = Vec::new();
        if !self.servers_dir().exists() {
            return Ok(configs);
        }

        for entry in fs::read_dir(self.servers_dir()).map_err(|source| {
            CoreError::ReadDirectory { path: self.servers_dir().to_path_buf(), source }
        })? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if !is_supported_config_extension(path.extension()) {
                continue;
            }
            let config = ServerConfig::from_file(&path)?;
            configs.push(config);
        }

        configs.sort_by(|a, b| {
            let name_a = a.definition().name.as_deref().unwrap_or_default();
            let name_b = b.definition().name.as_deref().unwrap_or_default();
            match name_a.cmp(name_b) {
                Ordering::Equal => a.definition().id.cmp(&b.definition().id),
                other => other,
            }
        });
        Ok(configs)
    }

    /// Load a server configuration by display name.
    pub fn load_server_config_by_name(&self, name: &str) -> Result<ServerConfig> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(CoreError::ServerNameEmpty { id: None }.into());
        }

        for config in self.list_server_configs()? {
            if config
                .definition()
                .name
                .as_deref()
                .map(|candidate| candidate == trimmed)
                .unwrap_or(false)
            {
                return Ok(config);
            }
        }

        Err(CoreError::ServerConfigNotFoundByName { name: trimmed.to_string() }.into())
    }

    /// Remove a server configuration (and auxiliary files if present).
    pub fn remove_server_config(&self, id: &str) -> Result<()> {
        let mut removed = false;
        for candidate in self.server_config_candidates(id) {
            if candidate.exists() {
                fs::remove_file(&candidate)
                    .map_err(|source| CoreError::RemoveFile { path: candidate.clone(), source })?;
                removed = true;
            }
        }

        if !removed {
            return Err(CoreError::ServerConfigNotFound { id: id.to_string() }.into());
        }

        let log_path = self.server_log_path(id);
        if log_path.exists() {
            let _ = fs::remove_file(&log_path);
        }
        let pid_path = self.server_pid_path(id);
        if pid_path.exists() {
            let _ = fs::remove_file(&pid_path);
        }
        Ok(())
    }

    fn server_config_candidates(&self, id: &str) -> [PathBuf; 2] {
        [self.server_config_toml_path(id), self.servers_dir().join(format!("{id}.json"))]
    }
}

/// Determine the default root directory for mcp-center.
pub fn default_root() -> Result<PathBuf> {
    if let Ok(value) = env::var(ROOT_ENV_KEY) {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value));
        }
    }

    let home = user_home_dir().ok_or(CoreError::HomeDirectoryUnknown)?;
    Ok(home.join(DEFAULT_ROOT_DIRNAME))
}

fn user_home_dir() -> Option<PathBuf> {
    if let Ok(home) = env::var("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }

    if let Ok(profile) = env::var("USERPROFILE") {
        if !profile.is_empty() {
            return Some(PathBuf::from(profile));
        }
    }

    None
}

fn is_supported_config_extension(ext: Option<&OsStr>) -> bool {
    matches!(ext.and_then(|s| s.to_str()), Some("toml" | "json"))
}
