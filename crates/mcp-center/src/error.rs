use std::{
    io,
    path::{Path, PathBuf},
};

use serde_json::Error as JsonError;
use thiserror::Error;
use toml_edit::{de::Error as TomlDeError, ser::Error as TomlSerError};
use url::ParseError as UrlParseError;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("server name cannot be empty")]
    ServerNameEmpty { id: Option<String> },

    #[error("server command cannot be empty")]
    ServerCommandEmpty { id: Option<String> },

    #[error("unsupported protocol for server")]
    UnsupportedProtocol { id: Option<String> },

    #[error("server endpoint is required for remote protocols")]
    ServerEndpointMissing { id: Option<String> },

    #[error("invalid server endpoint '{endpoint}'")]
    ServerEndpointInvalid {
        id: Option<String>,
        endpoint: String,
        #[source]
        source: UrlParseError,
    },

    #[error("server configuration '{id}' not found")]
    ServerConfigNotFound { id: String },

    #[error("server '{name}' not found")]
    ServerConfigNotFoundByName { name: String },

    #[error("failed to create directory {path}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to read directory {path}")]
    ReadDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to read config file {path}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse JSON server config at {path}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: JsonError,
    },

    #[error("failed to parse TOML server config at {path}")]
    ParseToml {
        path: PathBuf,
        #[source]
        source: TomlDeError,
    },

    #[error("failed to serialise server definition to TOML")]
    SerialiseToml {
        #[source]
        source: TomlSerError,
    },

    #[error("project configuration '{id}' not found")]
    ProjectConfigNotFound { id: String },

    #[error("failed to read project config file {path}")]
    ProjectRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse project config file {path}")]
    ProjectParse {
        path: PathBuf,
        #[source]
        source: TomlDeError,
    },

    #[error("failed to serialise project record to TOML")]
    ProjectSerialise {
        #[source]
        source: TomlSerError,
    },

    #[error("failed to write project config file {path}")]
    ProjectWrite {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to remove {path}")]
    RemoveFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("unable to determine user home directory for MCP_CENTER_ROOT")]
    HomeDirectoryUnknown,
}

impl CoreError {
    pub fn message_key(&self) -> &'static str {
        match self {
            CoreError::ServerNameEmpty { id } => {
                if id.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
                    "core.server_name_empty_with_id"
                } else {
                    "core.server_name_empty"
                }
            }
            CoreError::ServerCommandEmpty { id } => {
                if id.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
                    "core.server_command_empty_with_id"
                } else {
                    "core.server_command_empty"
                }
            }
            CoreError::UnsupportedProtocol { id } => {
                if id.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
                    "core.unsupported_protocol_with_id"
                } else {
                    "core.unsupported_protocol"
                }
            }
            CoreError::ServerEndpointMissing { id } => {
                if id.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
                    "core.server_endpoint_missing_with_id"
                } else {
                    "core.server_endpoint_missing"
                }
            }
            CoreError::ServerEndpointInvalid { id, .. } => {
                if id.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
                    "core.server_endpoint_invalid_with_id"
                } else {
                    "core.server_endpoint_invalid"
                }
            }
            CoreError::ServerConfigNotFound { .. } => "core.server_config_not_found",
            CoreError::ServerConfigNotFoundByName { .. } => "core.server_config_not_found_name",
            CoreError::CreateDirectory { .. } => "core.create_dir_failed",
            CoreError::ReadDirectory { .. } => "core.read_dir_failed",
            CoreError::ReadConfig { .. } => "core.read_config_failed",
            CoreError::ParseJson { .. } => "core.parse_json_failed",
            CoreError::ParseToml { .. } => "core.parse_toml_failed",
            CoreError::SerialiseToml { .. } => "core.serialise_toml_failed",
            CoreError::ProjectConfigNotFound { .. } => "core.project_config_not_found",
            CoreError::ProjectRead { .. } => "core.project_read_failed",
            CoreError::ProjectParse { .. } => "core.project_parse_failed",
            CoreError::ProjectSerialise { .. } => "core.project_serialise_failed",
            CoreError::ProjectWrite { .. } => "core.project_write_failed",
            CoreError::RemoveFile { .. } => "core.remove_file_failed",
            CoreError::HomeDirectoryUnknown => "core.home_dir_unknown",
        }
    }

    pub fn placeholders(&self) -> Vec<(&'static str, String)> {
        match self {
            CoreError::ServerNameEmpty { id }
            | CoreError::ServerCommandEmpty { id }
            | CoreError::UnsupportedProtocol { id }
            | CoreError::ServerEndpointMissing { id } => id
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|id| vec![("id", id.clone())])
                .unwrap_or_default(),
            CoreError::ServerEndpointInvalid { id, endpoint, source } => {
                let mut placeholders = id
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .map(|id| vec![("id", id.clone())])
                    .unwrap_or_default();
                placeholders.push(("endpoint", endpoint.clone()));
                placeholders.push(("error", source.to_string()));
                placeholders
            }
            CoreError::ServerConfigNotFound { id } => vec![("id", id.clone())],
            CoreError::ServerConfigNotFoundByName { name } => {
                vec![("name", name.clone())]
            }
            CoreError::CreateDirectory { path, source }
            | CoreError::ReadDirectory { path, source }
            | CoreError::ReadConfig { path, source }
            | CoreError::RemoveFile { path, source }
            | CoreError::ProjectRead { path, source }
            | CoreError::ProjectWrite { path, source } => {
                vec![("path", display_path(path)), ("error", source.to_string())]
            }
            CoreError::ParseJson { path, source } => {
                vec![("path", display_path(path)), ("error", source.to_string())]
            }
            CoreError::ParseToml { path, source } | CoreError::ProjectParse { path, source } => {
                vec![("path", display_path(path)), ("error", source.to_string())]
            }
            CoreError::SerialiseToml { source } | CoreError::ProjectSerialise { source } => {
                vec![("error", source.to_string())]
            }
            CoreError::ProjectConfigNotFound { id } => vec![("id", id.clone())],
            CoreError::HomeDirectoryUnknown => Vec::new(),
        }
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}
