use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
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
use tracing::warn;

const PROJECT_ID_HEX_LEN: usize = 16;
const MAX_CACHE_REFRESH_ATTEMPTS: usize = 3;

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
    inner: Arc<ProjectRegistryInner>,
}

#[derive(Debug)]
struct ProjectRegistryInner {
    root: PathBuf,
    cache: RwLock<ProjectCache>,
}

/// Snapshot of files under the projects directory used to invalidate the in-memory cache.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DirectoryFingerprint {
    Missing,
    Known(HashMap<PathBuf, FileFingerprint>),
    Unknown,
}

impl DirectoryFingerprint {
    fn missing() -> Self {
        DirectoryFingerprint::Missing
    }

    fn unknown() -> Self {
        DirectoryFingerprint::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    modified: Option<SystemTime>,
    len: Option<u64>,
}

impl FileFingerprint {
    fn from_metadata(metadata: Option<fs::Metadata>) -> Self {
        let modified = metadata.as_ref().and_then(|meta| meta.modified().ok());
        let len = metadata.as_ref().map(|meta| meta.len());
        FileFingerprint { modified, len }
    }
}

/// Cached project records backed by the on-disk TOML files.
#[derive(Debug)]
struct ProjectCache {
    records_by_id: HashMap<String, ProjectRecord>,
    path_index: HashMap<PathBuf, String>,
    fingerprint: DirectoryFingerprint,
    loaded: bool,
}

impl Default for ProjectCache {
    fn default() -> Self {
        Self {
            records_by_id: HashMap::new(),
            path_index: HashMap::new(),
            fingerprint: DirectoryFingerprint::missing(),
            loaded: false,
        }
    }
}

impl ProjectCache {
    fn replace_with_snapshot(
        &mut self,
        snapshot: CacheSnapshot,
        fingerprint: DirectoryFingerprint,
    ) {
        self.records_by_id = snapshot.records_by_id;
        self.path_index = snapshot.path_index;
        self.fingerprint = fingerprint;
        self.loaded = true;
    }

    fn clear_to_missing(&mut self) {
        self.records_by_id.clear();
        self.path_index.clear();
        self.fingerprint = DirectoryFingerprint::missing();
        self.loaded = true;
    }

    fn record_vec(&self) -> Vec<ProjectRecord> {
        self.records_by_id.values().cloned().collect()
    }
}

struct CacheSnapshot {
    records_by_id: HashMap<String, ProjectRecord>,
    path_index: HashMap<PathBuf, String>,
    fingerprint: DirectoryFingerprint,
}

impl ProjectRegistry {
    pub fn new(layout: &Layout) -> Self {
        Self {
            inner: Arc::new(ProjectRegistryInner {
                root: layout.projects_dir().to_path_buf(),
                cache: RwLock::new(ProjectCache::default()),
            }),
        }
    }

    pub fn ensure(&self) -> Result<()> {
        if !self.root().exists() {
            fs::create_dir_all(self.root()).map_err(|source| CoreError::CreateDirectory {
                path: self.root().to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<ProjectRecord>> {
        self.ensure_cache_fresh()?;
        let cache = self.cache();
        let cache = cache.read().expect("project cache poisoned");
        Ok(cache.record_vec())
    }

    pub fn load(&self, id: &ProjectId) -> Result<ProjectRecord> {
        self.ensure_cache_fresh()?;
        let cache = self.cache();
        let cache = cache.read().expect("project cache poisoned");
        cache
            .records_by_id
            .get(id.as_str())
            .cloned()
            .ok_or_else(|| CoreError::ProjectConfigNotFound { id: id.as_str().to_string() }.into())
    }

    pub fn load_from_id_str(&self, id_str: &str) -> Result<ProjectRecord> {
        self.ensure_cache_fresh()?;
        let cache = self.cache();
        let cache = cache.read().expect("project cache poisoned");
        cache
            .records_by_id
            .get(id_str)
            .cloned()
            .ok_or_else(|| CoreError::ProjectConfigNotFound { id: id_str.to_string() }.into())
    }

    pub fn find_by_path(&self, target: &Path) -> Result<Option<ProjectRecord>> {
        self.ensure_cache_fresh()?;
        let cache = self.cache();
        let cache = cache.read().expect("project cache poisoned");
        if let Some(id) = cache.path_index.get(target) {
            Ok(cache.records_by_id.get(id).cloned())
        } else {
            Ok(None)
        }
    }

    pub fn store(&self, record: &ProjectRecord) -> Result<()> {
        let path = self.path_for_raw(&record.id);
        let doc = toml_edit::ser::to_string_pretty(record)
            .map_err(|source| CoreError::ProjectSerialise { source })?;
        fs::write(&path, doc).map_err(|source| CoreError::ProjectWrite { path, source })?;
        self.update_cache_after_store(record.clone());
        Ok(())
    }

    pub fn delete(&self, id: &ProjectId) -> Result<()> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(CoreError::ProjectConfigNotFound { id: id.as_str().to_string() }.into());
        }
        fs::remove_file(&path)
            .map_err(|source| CoreError::RemoveFile { path: path.clone(), source })?;
        self.update_cache_after_delete(id.as_str());
        Ok(())
    }

    pub fn delete_by_id_str(&self, id: &str) -> Result<()> {
        let path = self.path_for_raw(id);
        if !path.exists() {
            return Err(CoreError::ProjectConfigNotFound { id: id.to_string() }.into());
        }
        fs::remove_file(&path)
            .map_err(|source| CoreError::RemoveFile { path: path.clone(), source })?;
        self.update_cache_after_delete(id);
        Ok(())
    }

    pub fn path_for(&self, id: &ProjectId) -> PathBuf {
        self.path_for_raw(id.as_str())
    }

    pub fn path_for_raw(&self, id: &str) -> PathBuf {
        self.root().join(format!("{id}.toml"))
    }

    fn root(&self) -> &Path {
        &self.inner.root
    }

    fn cache(&self) -> &RwLock<ProjectCache> {
        &self.inner.cache
    }

    fn ensure_cache_fresh(&self) -> Result<()> {
        let fingerprint = self.collect_fingerprint()?;
        {
            let cache = self.cache().read().expect("project cache poisoned");
            if cache.loaded
                && cache.fingerprint == fingerprint
                && !matches!(fingerprint, DirectoryFingerprint::Unknown)
            {
                return Ok(());
            }
        }
        self.refresh_cache(fingerprint)
    }

    fn refresh_cache(&self, fingerprint: DirectoryFingerprint) -> Result<()> {
        if matches!(fingerprint, DirectoryFingerprint::Missing) {
            let mut cache = self.cache().write().expect("project cache poisoned");
            cache.clear_to_missing();
            return Ok(());
        }

        let mut attempts = 0usize;
        loop {
            attempts += 1;
            let snapshot = self.scan_records()?;
            let confirm = self.collect_fingerprint()?;

            if snapshot.fingerprint == confirm {
                let mut cache = self.cache().write().expect("project cache poisoned");
                cache.replace_with_snapshot(snapshot, confirm);
                return Ok(());
            }

            if attempts >= MAX_CACHE_REFRESH_ATTEMPTS {
                warn!(
                    "project registry directory mutated repeatedly during refresh; forcing reload next access"
                );
                let mut cache = self.cache().write().expect("project cache poisoned");
                cache.replace_with_snapshot(snapshot, DirectoryFingerprint::unknown());
                return Ok(());
            }
        }
    }

    fn update_cache_after_store(&self, record: ProjectRecord) {
        let file_path = self.path_for_raw(&record.id);
        let mut cache = self.cache().write().expect("project cache poisoned");
        if !cache.loaded {
            cache.fingerprint = DirectoryFingerprint::unknown();
            return;
        }
        if let Some(previous) = cache.records_by_id.insert(record.id.clone(), record.clone()) {
            cache.path_index.remove(&previous.path);
        }
        cache.path_index.insert(record.path.clone(), record.id.clone());
        match &mut cache.fingerprint {
            DirectoryFingerprint::Known(entries) => match fs::metadata(&file_path) {
                Ok(metadata) => {
                    entries.insert(file_path, FileFingerprint::from_metadata(Some(metadata)));
                }
                Err(_) => {
                    cache.fingerprint = DirectoryFingerprint::unknown();
                }
            },
            _ => {
                cache.fingerprint = DirectoryFingerprint::unknown();
            }
        }
    }

    fn update_cache_after_delete(&self, id: &str) {
        let file_path = self.path_for_raw(id);
        let mut cache = self.cache().write().expect("project cache poisoned");
        if !cache.loaded {
            cache.fingerprint = DirectoryFingerprint::unknown();
            return;
        }
        if let Some(previous) = cache.records_by_id.remove(id) {
            cache.path_index.remove(&previous.path);
        }
        match &mut cache.fingerprint {
            DirectoryFingerprint::Known(entries) => {
                entries.remove(&file_path);
            }
            _ => {
                cache.fingerprint = DirectoryFingerprint::unknown();
            }
        }
    }

    fn collect_fingerprint(&self) -> Result<DirectoryFingerprint> {
        let root = self.root();
        if !root.exists() {
            return Ok(DirectoryFingerprint::missing());
        }

        let mut entries = HashMap::new();
        for entry in fs::read_dir(root)
            .map_err(|source| CoreError::ReadDirectory { path: root.to_path_buf(), source })?
        {
            let entry = entry
                .map_err(|source| CoreError::ReadDirectory { path: root.to_path_buf(), source })?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let metadata = entry.metadata().ok();
            entries.insert(path, FileFingerprint::from_metadata(metadata));
        }

        Ok(DirectoryFingerprint::Known(entries))
    }

    fn scan_records(&self) -> Result<CacheSnapshot> {
        let mut records_by_id = HashMap::new();
        let mut path_index = HashMap::new();
        let mut entries = HashMap::new();

        let root = self.root();
        if !root.exists() {
            return Ok(CacheSnapshot {
                records_by_id,
                path_index,
                fingerprint: DirectoryFingerprint::missing(),
            });
        }

        for entry in fs::read_dir(root)
            .map_err(|source| CoreError::ReadDirectory { path: root.to_path_buf(), source })?
        {
            let entry = entry
                .map_err(|source| CoreError::ReadDirectory { path: root.to_path_buf(), source })?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let metadata = entry.metadata().ok();
            entries.insert(path.clone(), FileFingerprint::from_metadata(metadata));
            let record = self.load_from_path(&path)?;
            path_index.insert(record.path.clone(), record.id.clone());
            records_by_id.insert(record.id.clone(), record);
        }

        Ok(CacheSnapshot {
            records_by_id,
            path_index,
            fingerprint: DirectoryFingerprint::Known(entries),
        })
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
