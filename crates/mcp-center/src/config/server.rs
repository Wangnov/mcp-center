use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use url::Url;

use super::id_generator::generate_id;
use crate::error::CoreError;

/// Supported MCP server protocols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ServerProtocol {
    #[serde(alias = "stdio")]
    #[default]
    StdIo,
    #[serde(alias = "sse")]
    Sse,
    #[serde(alias = "http")]
    Http,
    #[serde(other)]
    Unknown,
}

/// Definition of a single MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerDefinition {
    /// Unique identifier used internally and for filenames.
    #[serde(default)]
    pub id: String,
    /// Optional human friendly name.
    #[serde(default)]
    pub name: Option<String>,
    /// Communication protocol. Defaults to `stdio`.
    #[serde(default)]
    pub protocol: ServerProtocol,
    /// Executable to launch.
    #[serde(default)]
    pub command: String,
    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables injected into the process.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Endpoint for remote MCP servers.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Extra headers for remote MCP servers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Whether the server is currently enabled.
    #[serde(default)]
    pub enabled: bool,
}

impl ServerDefinition {
    /// Validate invariants (non-empty id/command).
    pub fn validate(&self) -> Result<()> {
        if self.name.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            return Err(CoreError::ServerNameEmpty {
                id: (!self.id.trim().is_empty()).then(|| self.id.clone()),
            }
            .into());
        }
        if matches!(self.protocol, ServerProtocol::StdIo) && self.command.trim().is_empty() {
            return Err(CoreError::ServerCommandEmpty {
                id: (!self.id.trim().is_empty()).then(|| self.id.clone()),
            }
            .into());
        }
        if matches!(self.protocol, ServerProtocol::Unknown) {
            return Err(CoreError::UnsupportedProtocol {
                id: (!self.id.trim().is_empty()).then(|| self.id.clone()),
            }
            .into());
        }
        if matches!(self.protocol, ServerProtocol::Sse | ServerProtocol::Http) {
            let id = (!self.id.trim().is_empty()).then(|| self.id.clone());
            let endpoint = self
                .endpoint
                .as_ref()
                .and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                })
                .ok_or_else(|| CoreError::ServerEndpointMissing { id: id.clone() })?;
            Url::parse(endpoint).map_err(|source| CoreError::ServerEndpointInvalid {
                id,
                endpoint: endpoint.to_string(),
                source,
            })?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ServerConfigDocument {
    Wrapped { mcp_server: ServerDefinition },
    Direct(ServerDefinition),
}

impl ServerConfigDocument {
    fn into_definition(self) -> ServerDefinition {
        match self {
            ServerConfigDocument::Wrapped { mcp_server } => mcp_server,
            ServerConfigDocument::Direct(definition) => definition,
        }
    }
}

/// Wrapper around a server definition loaded from disk.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    definition: ServerDefinition,
    source: Option<PathBuf>,
}

impl ServerConfig {
    /// Load from a TOML or JSON file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|source| CoreError::ReadConfig { path: path.to_path_buf(), source })?;
        let mut definition = if is_json_path(path) {
            let doc: ServerConfigDocument = serde_json::from_str(&content)
                .map_err(|source| CoreError::ParseJson { path: path.to_path_buf(), source })?;
            doc.into_definition()
        } else {
            let doc: ServerConfigDocument = toml_edit::de::from_str(&content)
                .map_err(|source| CoreError::ParseToml { path: path.to_path_buf(), source })?;
            doc.into_definition()
        };

        if let Some(name) = definition.name.take() {
            let trimmed = name.trim().to_string();
            definition.name = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };
        }
        if let Some(endpoint) = definition.endpoint.take() {
            let trimmed = endpoint.trim().to_string();
            definition.endpoint = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };
        }

        let config = ServerConfig { definition, source: Some(path.to_path_buf()) };
        config.validate()?;
        Ok(config)
    }

    /// Construct from a definition (not yet persisted).
    pub fn new(definition: ServerDefinition) -> Result<Self> {
        let config = Self { definition, source: None };
        config.validate()?;
        Ok(config)
    }

    /// Persist to TOML format.
    pub fn to_toml_string(&self) -> Result<String> {
        let doc = ServerConfigDocument::Wrapped { mcp_server: self.definition.clone() };
        toml_edit::ser::to_string_pretty(&doc)
            .map_err(|source| CoreError::SerialiseToml { source }.into())
    }

    /// Access the inner definition.
    pub fn definition(&self) -> &ServerDefinition {
        &self.definition
    }

    /// Mutable access to the inner definition.
    pub fn definition_mut(&mut self) -> &mut ServerDefinition {
        &mut self.definition
    }

    /// Path on disk, if loaded from file.
    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    fn validate(&self) -> Result<()> {
        self.definition.validate()
    }

    pub fn assign_unique_id(&mut self, existing: &HashSet<String>) {
        if self.definition.id.trim().is_empty() {
            let candidate = generate_id(existing);
            self.definition.id = candidate;
        }
    }
}

fn is_json_path(path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("json"))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn parses_wrapped_toml_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("context7.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"
[mcp_server]
id = "context7"
name = "Context7"
command = "npx"
args = ["-y"]
"#
        )
        .unwrap();

        let config = ServerConfig::from_file(&path).unwrap();
        assert_eq!(config.definition().id, "context7");
        assert_eq!(config.definition().command, "npx");
        assert!(config.definition().args.contains(&"-y".to_string()));
    }

    #[test]
    fn rejects_missing_endpoint_for_remote() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"
[mcp_server]
id = "invalid"
name = "Invalid"
command = "npx"
protocol = "sse"
"#
        )
        .unwrap();

        let err = ServerConfig::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("endpoint"), "unexpected error: {err:?}");
    }

    #[test]
    fn rejects_invalid_endpoint_for_remote() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad_endpoint.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"
[mcp_server]
id = "invalid"
name = "Invalid"
protocol = "sse"
endpoint = "not a url"
"#
        )
        .unwrap();

        let err = ServerConfig::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("endpoint"), "unexpected error: {err:?}");
    }

    #[test]
    fn accepts_valid_remote_definition() {
        let definition = ServerDefinition {
            id: "deepwiki".into(),
            name: Some("DeepWiki".into()),
            protocol: ServerProtocol::Sse,
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            endpoint: Some("https://mcp.deepwiki.com/sse".into()),
            headers: BTreeMap::new(),
            enabled: false,
        };
        assert!(ServerConfig::new(definition).is_ok());
    }
}
