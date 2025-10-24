use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolResult, JsonObject, LoggingLevel, LoggingMessageNotification},
    service::ServiceError,
};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::Mutex};

/// Non-blocking append-only log sink backed by a single file.
#[derive(Clone)]
pub struct LogSink {
    inner: Arc<LogSinkInner>,
}

struct LogSinkInner {
    file: Mutex<tokio::fs::File>,
}

impl LogSink {
    pub async fn new(path: PathBuf, truncate: bool) -> Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create log directory {}", parent.display()))?;
        }
        if truncate {
            tokio::fs::File::create(&path)
                .await
                .with_context(|| format!("failed to truncate log file {}", path.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .with_context(|| format!("failed to open log file {}", path.display()))?;
        Ok(Self { inner: Arc::new(LogSinkInner { file: Mutex::new(file) }) })
    }

    pub async fn append<T: Serialize>(&self, record: &T) -> Result<()> {
        let mut buffer = serde_json::to_vec(record).context("failed to serialise log record")?;
        buffer.push(b'\n');
        let mut file = self.inner.file.lock().await;
        file.write_all(&buffer).await.context("failed to write log record")?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ServerLogHandle {
    inner: Arc<ServerLogHandleInner>,
}

struct ServerLogHandleInner {
    server_id: String,
    server_name: String,
    sink: LogSink,
}

impl ServerLogHandle {
    pub async fn new(server_id: String, server_name: String, log_path: PathBuf) -> Result<Self> {
        let sink = LogSink::new(log_path, true).await?;
        Ok(Self { inner: Arc::new(ServerLogHandleInner { server_id, server_name, sink }) })
    }

    pub fn server_id(&self) -> &str {
        &self.inner.server_id
    }

    pub fn server_name(&self) -> &str {
        &self.inner.server_name
    }

    pub async fn log_mcp_message(&self, notification: &LoggingMessageNotification) -> Result<()> {
        let payload_summary = summarise_payload(notification.params.data.clone());
        let mut detail_map = JsonMap::new();
        if let Some(logger) = notification.params.logger.clone() {
            detail_map.insert("logger".to_string(), Value::String(logger));
        }
        detail_map.insert("payload".to_string(), notification.params.data.clone());

        let entry = LogEntry {
            timestamp: now_timestamp(),
            level: map_logging_level(notification.params.level),
            category: LogCategory::McpMessage,
            message: payload_summary,
            server: Some(ServerContext::new(self.server_id(), self.server_name())),
            tool: None,
            duration_ms: None,
            details: Some(Value::Object(detail_map)),
        };
        self.inner.sink.append(&entry).await
    }

    pub async fn log_tool_request(
        &self,
        call_id: &str,
        tool_name: &str,
        arguments: Option<&JsonObject>,
    ) -> Result<()> {
        let details = arguments.map(|args| Value::Object(args.clone()));
        let entry = LogEntry {
            timestamp: now_timestamp(),
            level: LogLevel::Info,
            category: LogCategory::ToolRequest,
            message: format!("tool call started: {tool_name}"),
            server: Some(ServerContext::new(self.server_id(), self.server_name())),
            tool: Some(ToolContext::new(tool_name, call_id)),
            duration_ms: None,
            details,
        };
        self.inner.sink.append(&entry).await
    }

    pub async fn log_tool_response(
        &self,
        call_id: &str,
        tool_name: &str,
        duration: Duration,
        result: &CallToolResult,
    ) -> Result<()> {
        let details = serde_json::to_value(result).ok();
        let entry = LogEntry {
            timestamp: now_timestamp(),
            level: if result.is_error.unwrap_or(false) {
                LogLevel::Error
            } else {
                LogLevel::Info
            },
            category: if result.is_error.unwrap_or(false) {
                LogCategory::ToolError
            } else {
                LogCategory::ToolResponse
            },
            message: if result.is_error.unwrap_or(false) {
                format!("tool call failed: {tool_name}")
            } else {
                format!("tool call completed: {tool_name}")
            },
            server: Some(ServerContext::new(self.server_id(), self.server_name())),
            tool: Some(ToolContext::new(tool_name, call_id)),
            duration_ms: Some(duration.as_millis()),
            details,
        };
        self.inner.sink.append(&entry).await
    }

    pub async fn log_tool_error(
        &self,
        call_id: &str,
        tool_name: &str,
        duration: Duration,
        error: &ServiceError,
    ) -> Result<()> {
        let entry = LogEntry {
            timestamp: now_timestamp(),
            level: LogLevel::Error,
            category: LogCategory::ToolError,
            message: format!("tool call failed: {tool_name}"),
            server: Some(ServerContext::new(self.server_id(), self.server_name())),
            tool: Some(ToolContext::new(tool_name, call_id)),
            duration_ms: Some(duration.as_millis()),
            details: Some(json!({
                "error": format!("{error:?}")
            })),
        };
        self.inner.sink.append(&entry).await
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogCategory {
    McpMessage,
    ToolRequest,
    ToolResponse,
    ToolError,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerContext {
    pub id: String,
    pub name: String,
}

impl ServerContext {
    fn new(id: &str, name: &str) -> Self {
        Self { id: id.to_string(), name: name.to_string() }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolContext {
    pub name: String,
    pub call_id: String,
}

impl ToolContext {
    fn new(name: &str, call_id: &str) -> Self {
        Self { name: name.to_string(), call_id: call_id.to_string() }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub category: LogCategory,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

fn now_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn summarise_payload(payload: Value) -> String {
    match payload {
        Value::String(text) => text,
        Value::Object(mut map) => map
            .remove("message")
            .and_then(|value| value.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| Value::Object(map).to_string()),
        other => other.to_string(),
    }
}

fn map_logging_level(level: LoggingLevel) -> LogLevel {
    match level {
        LoggingLevel::Debug => LogLevel::Debug,
        LoggingLevel::Info | LoggingLevel::Notice => LogLevel::Info,
        LoggingLevel::Warning => LogLevel::Warn,
        LoggingLevel::Error
        | LoggingLevel::Critical
        | LoggingLevel::Alert
        | LoggingLevel::Emergency => LogLevel::Error,
    }
}
