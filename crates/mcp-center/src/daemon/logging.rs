use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use rmcp::{
    model::{CallToolResult, JsonObject, LoggingLevel, LoggingMessageNotification},
    service::ServiceError,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value, json};
use specta::Type;
use time::{Date, OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::{Mutex, broadcast},
};
use tokio_stream::wrappers::BroadcastStream;

use crate::Layout;

/// Append-only JSONL log sink with simple daily file rotation.
#[derive(Clone)]
pub struct LogSink {
    inner: Arc<LogSinkInner>,
}

struct LogSinkInner {
    directory: PathBuf,
    state: Mutex<SinkState>,
}

struct SinkState {
    current_date: Date,
    file: tokio::fs::File,
}

impl LogSink {
    pub async fn new(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory)
            .await
            .with_context(|| format!("failed to create log directory {}", directory.display()))?;

        let today = current_utc_date();
        let file = open_daily_file(&directory, today).await?;
        let state = SinkState { current_date: today, file };

        Ok(Self { inner: Arc::new(LogSinkInner { directory, state: Mutex::new(state) }) })
    }

    pub fn directory(&self) -> &Path {
        &self.inner.directory
    }

    pub async fn append(&self, entry: &LogEntry) -> Result<()> {
        let mut buffer = serde_json::to_vec(entry).context("failed to serialise log record")?;
        buffer.push(b'\n');

        let mut state = self.inner.state.lock().await;
        let today = current_utc_date();
        if state.current_date != today {
            state.file = open_daily_file(&self.inner.directory, today).await?;
            state.current_date = today;
        }
        state.file.write_all(&buffer).await.context("failed to write log record")
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
    stream: broadcast::Sender<Arc<LogEntry>>,
}

impl ServerLogHandle {
    pub async fn new(server_id: String, server_name: String, log_dir: PathBuf) -> Result<Self> {
        let sink = LogSink::new(log_dir).await?;
        let (stream, _) = broadcast::channel(512);
        Ok(Self { inner: Arc::new(ServerLogHandleInner { server_id, server_name, sink, stream }) })
    }

    pub fn server_id(&self) -> &str {
        &self.inner.server_id
    }

    pub fn server_name(&self) -> &str {
        &self.inner.server_name
    }

    pub fn log_dir(&self) -> PathBuf {
        self.inner.sink.directory().to_path_buf()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<LogEntry>> {
        self.inner.stream.subscribe()
    }

    async fn record(&self, entry: LogEntry) -> Result<()> {
        let entry = Arc::new(entry);
        self.inner.sink.append(&entry).await?;
        let _ = self.inner.stream.send(entry);
        Ok(())
    }

    pub async fn log_mcp_message(&self, notification: &LoggingMessageNotification) -> Result<()> {
        let payload_summary = summarise_payload(notification.params.data.clone());
        let mut detail_map = JsonMap::new();
        if let Some(logger) = notification.params.logger.clone() {
            detail_map.insert("logger".to_string(), Value::String(logger));
        }
        detail_map.insert("payload".to_string(), notification.params.data.clone());

        self.record(LogEntry {
            timestamp: now_timestamp(),
            level: map_logging_level(notification.params.level),
            category: LogCategory::McpMessage,
            message: payload_summary,
            server: Some(ServerContext::new(self.server_id(), self.server_name())),
            tool: None,
            duration_ms: None,
            details: Some(Value::Object(detail_map)),
        })
        .await
    }

    pub async fn log_tool_request(
        &self,
        call_id: &str,
        tool_name: &str,
        arguments: Option<&JsonObject>,
    ) -> Result<()> {
        self.record(LogEntry {
            timestamp: now_timestamp(),
            level: LogLevel::Info,
            category: LogCategory::ToolRequest,
            message: format!("tool call started: {tool_name}"),
            server: Some(ServerContext::new(self.server_id(), self.server_name())),
            tool: Some(ToolContext::new(tool_name, call_id)),
            duration_ms: None,
            details: arguments.map(|args| Value::Object(args.clone())),
        })
        .await
    }

    pub async fn log_tool_response(
        &self,
        call_id: &str,
        tool_name: &str,
        duration: Duration,
        result: &CallToolResult,
    ) -> Result<()> {
        let is_error = result.is_error.unwrap_or(false);
        self.record(LogEntry {
            timestamp: now_timestamp(),
            level: if is_error {
                LogLevel::Error
            } else {
                LogLevel::Info
            },
            category: if is_error {
                LogCategory::ToolError
            } else {
                LogCategory::ToolResponse
            },
            message: if is_error {
                format!("tool call failed: {tool_name}")
            } else {
                format!("tool call completed: {tool_name}")
            },
            server: Some(ServerContext::new(self.server_id(), self.server_name())),
            tool: Some(ToolContext::new(tool_name, call_id)),
            duration_ms: Some(duration.as_millis()),
            details: serde_json::to_value(result).ok(),
        })
        .await
    }

    pub async fn log_tool_error(
        &self,
        call_id: &str,
        tool_name: &str,
        duration: Duration,
        error: &ServiceError,
    ) -> Result<()> {
        self.record(LogEntry {
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
        })
        .await
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LogCategory {
    McpMessage,
    ToolRequest,
    ToolResponse,
    ToolError,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
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

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
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

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
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

#[derive(Debug, Clone)]
pub struct LogFileMeta {
    pub server_id: String,
    pub file_name: String,
    pub path: PathBuf,
    pub file_size: u64,
    pub line_count: u64,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogReadPage {
    pub entries: Vec<LogEntry>,
    pub next_offset: Option<u64>,
    pub has_more: bool,
}

pub async fn list_server_log_files(layout: &Layout, server_id: &str) -> Result<Vec<LogFileMeta>> {
    let dir = layout.server_log_dir(server_id);
    let mut result = Vec::new();

    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read log dir {}", dir.display()));
        }
    };

    while let Some(entry) = entries.next_entry().await.context("failed to iterate log directory")? {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("log") {
            continue;
        }
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;
        let summary = summarise_log_file(&path).await?;
        result.push(LogFileMeta {
            server_id: server_id.to_string(),
            file_name,
            path,
            file_size: metadata.len(),
            line_count: summary.line_count,
            first_timestamp: summary.first_timestamp,
            last_timestamp: summary.last_timestamp,
        });
    }

    result.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(result)
}

pub async fn read_log_entries(file_path: &Path, offset: u64, limit: usize) -> Result<LogReadPage> {
    ensure!(limit > 0, "limit must be greater than zero");

    let file = OpenOptions::new()
        .read(true)
        .open(file_path)
        .await
        .with_context(|| format!("failed to open log file {}", file_path.display()))?;

    let mut reader = BufReader::new(file).lines();
    let mut skipped = 0u64;
    while skipped < offset {
        if reader.next_line().await?.is_none() {
            return Ok(LogReadPage { entries: Vec::new(), next_offset: None, has_more: false });
        }
        skipped += 1;
    }

    let mut entries = Vec::with_capacity(limit);
    while entries.len() < limit {
        match reader.next_line().await? {
            Some(line) => match serde_json::from_str::<LogEntry>(&line) {
                Ok(entry) => entries.push(entry),
                Err(err) => {
                    tracing::warn!(error = ?err, "failed to parse log line");
                }
            },
            None => break,
        }
    }

    let has_more = reader.next_line().await?.is_some();
    let next_offset = if has_more {
        Some(offset + entries.len() as u64)
    } else {
        None
    };
    Ok(LogReadPage { entries, next_offset, has_more })
}

pub fn stream_server_logs(handle: &ServerLogHandle) -> BroadcastStream<Arc<LogEntry>> {
    BroadcastStream::new(handle.subscribe())
}

struct FileSummary {
    line_count: u64,
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
}

async fn summarise_log_file(path: &Path) -> Result<FileSummary> {
    let file = match OpenOptions::new().read(true).open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileSummary { line_count: 0, first_timestamp: None, last_timestamp: None });
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to open log file {}", path.display()));
        }
    };

    let mut reader = BufReader::new(file).lines();
    let mut line_count = 0u64;
    let mut first = None;
    let mut last = None;

    while let Some(line) = reader.next_line().await? {
        match serde_json::from_str::<LogEntry>(&line) {
            Ok(entry) => {
                if first.is_none() {
                    first = Some(entry.timestamp.clone());
                }
                last = Some(entry.timestamp);
            }
            Err(err) => {
                tracing::warn!(error = ?err, "failed to parse log line while summarising");
            }
        }
        line_count += 1;
    }

    Ok(FileSummary { line_count, first_timestamp: first, last_timestamp: last })
}

fn now_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn current_utc_date() -> Date {
    OffsetDateTime::now_utc().date()
}

async fn open_daily_file(directory: &Path, date: Date) -> Result<tokio::fs::File> {
    let filename = format!("{:04}{:02}{:02}", date.year(), u8::from(date.month()), date.day());
    let path = directory.join(format!("{filename}.log"));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .with_context(|| format!("failed to open log file {}", path.display()))
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
