use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use blake3::Hasher;
use serde::{Deserialize, Serialize};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt as _;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt as _;

use crate::{error::CoreError, paths::Layout};

const PROJECT_ID_HEX_LEN: usize = 16;

/// Identifier for a project tracked by MCP Center.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Generate a deterministic project id from a filesystem path string.
    pub fn from_path(path: &Path) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&path_signature(path));
        let hex = hasher.finalize().to_hex().to_string();
        let slice = &hex[..PROJECT_ID_HEX_LEN.min(hex.len())];
        ProjectId(slice.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn path_signature(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(windows)]
    {
        path.as_os_str()
            .encode_wide()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<u8>>()
    }
    #[cfg(not(any(unix, windows)))]
    {
        path.to_string_lossy().as_bytes().to_vec()
    }
}

impl From<ProjectId> for String {
    fn from(value: ProjectId) -> Self {
        value.0
    }
}

impl From<&ProjectId> for String {
    fn from(value: &ProjectId) -> Self {
        value.0.clone()
    }
}

/// Tool-level permission control for a specific server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "type")]
pub enum ToolPermission {
    /// Allow all tools from this server
    #[default]
    All,
    /// Allow only specified tools (whitelist)
    AllowList { tools: Vec<String> },
    /// Deny specified tools (blacklist)
    DenyList { tools: Vec<String> },
}

/// Customization for a specific tool (e.g., custom description).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCustomization {
    /// Tool name (e.g., "mcp__context7__get-library-docs")
    pub tool_name: String,
    /// Custom description to override the original
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Record persisted for each detected project path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub path: PathBuf,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default, alias = "allowed_servers")]
    pub allowed_server_ids: Vec<String>,
    /// Tool-level permissions per server (takes precedence over allowed_server_ids)
    #[serde(default)]
    pub allowed_server_tools: HashMap<String, ToolPermission>,
    /// Custom tool descriptions and settings
    #[serde(default)]
    pub tool_customizations: Vec<ToolCustomization>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default = "current_timestamp")]
    pub created_at: u64,
    #[serde(default = "current_timestamp")]
    pub last_seen_at: u64,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl ProjectRecord {
    pub fn new(id: ProjectId, path: PathBuf) -> Self {
        let timestamp = current_timestamp();
        ProjectRecord {
            id: id.into(),
            path,
            display_name: None,
            allowed_server_ids: Vec::new(),
            allowed_server_tools: HashMap::new(),
            tool_customizations: Vec::new(),
            agent: None,
            created_at: timestamp,
            last_seen_at: timestamp,
            metadata: HashMap::new(),
        }
    }

    pub fn touch(&mut self) {
        self.last_seen_at = current_timestamp();
    }

    pub fn set_agent(&mut self, agent: Option<String>) {
        self.agent = agent;
    }
}

/// Helper to load and persist project records on disk.
#[derive(Debug, Clone)]
pub struct ProjectRegistry {
    root: PathBuf,
}

impl ProjectRegistry {
    pub fn new(layout: &Layout) -> Self {
        Self { root: layout.projects_dir().to_path_buf() }
    }

    pub fn ensure(&self) -> Result<()> {
        if !self.root.exists() {
            fs::create_dir_all(&self.root)
                .map_err(|source| CoreError::CreateDirectory { path: self.root.clone(), source })?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<ProjectRecord>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(|source| CoreError::ReadDirectory { path: self.root.clone(), source })?
        {
            let entry = entry
                .map_err(|source| CoreError::ReadDirectory { path: self.root.clone(), source })?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let record = self.load_from_path(&path)?;
            records.push(record);
        }
        Ok(records)
    }

    pub fn load(&self, id: &ProjectId) -> Result<ProjectRecord> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(CoreError::ProjectConfigNotFound { id: id.as_str().to_string() }.into());
        }
        self.load_from_path(&path)
    }

    pub fn load_from_id_str(&self, id_str: &str) -> Result<ProjectRecord> {
        let path = self.path_for_raw(id_str);
        if !path.exists() {
            return Err(CoreError::ProjectConfigNotFound { id: id_str.to_string() }.into());
        }
        self.load_from_path(&path)
    }

    pub fn find_by_path(&self, target: &Path) -> Result<Option<ProjectRecord>> {
        let canonical_target = target.to_path_buf();
        for record in self.list()? {
            if record.path == canonical_target {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    pub fn store(&self, record: &ProjectRecord) -> Result<()> {
        let path = self.path_for_raw(&record.id);
        let doc = toml_edit::ser::to_string_pretty(record)
            .map_err(|source| CoreError::ProjectSerialise { source })?;
        fs::write(&path, doc).map_err(|source| CoreError::ProjectWrite { path, source })?;
        Ok(())
    }

    pub fn delete(&self, id: &ProjectId) -> Result<()> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(CoreError::ProjectConfigNotFound { id: id.as_str().to_string() }.into());
        }
        fs::remove_file(&path)
            .map_err(|source| CoreError::RemoveFile { path: path.clone(), source })?;
        Ok(())
    }

    pub fn delete_by_id_str(&self, id: &str) -> Result<()> {
        let path = self.path_for_raw(id);
        if !path.exists() {
            return Err(CoreError::ProjectConfigNotFound { id: id.to_string() }.into());
        }
        fs::remove_file(&path)
            .map_err(|source| CoreError::RemoveFile { path: path.clone(), source })?;
        Ok(())
    }

    pub fn path_for(&self, id: &ProjectId) -> PathBuf {
        self.path_for_raw(id.as_str())
    }

    pub fn path_for_raw(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.toml"))
    }

    fn load_from_path(&self, path: &Path) -> Result<ProjectRecord> {
        let content = fs::read_to_string(path)
            .map_err(|source| CoreError::ProjectRead { path: path.to_path_buf(), source })?;
        let mut record: ProjectRecord = toml_edit::de::from_str(&content)
            .map_err(|source| CoreError::ProjectParse { path: path.to_path_buf(), source })?;
        if record.id.trim().is_empty() {
            let id =
                path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().to_string();
            record.id = id;
        }
        Ok(record)
    }
}
