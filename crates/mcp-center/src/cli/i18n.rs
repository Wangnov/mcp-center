use std::{env, path::Path, sync::OnceLock};

use crate::CoreError;
use anyhow::Error as AnyhowError;
use clap::{Command, builder::Arg};
use locale_config::Locale;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    SimplifiedChinese,
    TraditionalChinese,
    Japanese,
}

const PLACEHOLDER_PREFIX: &str = "i18n:";

static LANGUAGE: OnceLock<Language> = OnceLock::new();
static MESSAGES: OnceLock<Messages> = OnceLock::new();

fn interpolate(template: &str, values: &[(&str, String)]) -> String {
    let mut result = template.to_owned();
    for (key, value) in values {
        let placeholder = format!("{{{key}}}");
        result = result.replace(&placeholder, value);
    }
    result
}

pub fn language() -> Language {
    *LANGUAGE.get_or_init(detect_language)
}

pub fn messages() -> &'static Messages {
    MESSAGES.get_or_init(|| Messages { language: language() })
}

fn detect_language() -> Language {
    if let Ok(value) = env::var("MCP_CENTER_LANG") {
        if let Some(lang) = parse_language_tag(&value) {
            return lang;
        }
    }

    let locale = Locale::user_default();
    for (_category, tag) in locale.tags() {
        if let Some(lang) = parse_language_tag(tag.as_ref()) {
            return lang;
        }
    }

    Language::English
}

fn parse_language_tag(raw: &str) -> Option<Language> {
    let mut normalized = raw
        .trim()
        .split('=')
        .next_back()
        .unwrap_or(raw)
        .replace('_', "-")
        .to_ascii_lowercase();

    if let Some(idx) = normalized.find('@') {
        normalized.truncate(idx);
    }
    if let Some(idx) = normalized.find('.') {
        normalized.truncate(idx);
    }

    if normalized.is_empty() {
        return None;
    }

    if matches!(normalized.as_str(), "ja" | "ja-jp" | "ja-jpan" | "ja-jp-u-ca-japanese")
        || normalized.starts_with("ja-")
    {
        return Some(Language::Japanese);
    }

    if normalized.starts_with("zh-hant")
        || normalized.starts_with("zh-tw")
        || normalized.starts_with("zh-hk")
        || normalized.starts_with("zh-mo")
    {
        return Some(Language::TraditionalChinese);
    }

    if normalized.starts_with("zh-hans")
        || normalized.starts_with("zh-cn")
        || normalized.starts_with("zh-sg")
        || normalized == "zh"
    {
        return Some(Language::SimplifiedChinese);
    }

    if normalized.starts_with("zh") {
        return Some(Language::TraditionalChinese);
    }

    if normalized.starts_with("en") {
        return Some(Language::English);
    }

    None
}

pub struct Messages {
    language: Language,
}

impl Messages {
    pub fn error_prefix(&self) -> &'static str {
        self.text("errors.prefix")
    }

    pub fn workspace_already_initialized(&self, root: &Path) -> String {
        interpolate(self.text("init.workspace_exists"), &[("root", root.display().to_string())])
    }

    pub fn workspace_initialized(&self, root: &Path, sample: &Path) -> String {
        interpolate(
            self.text("init.workspace_created"),
            &[("root", root.display().to_string()), ("sample", sample.display().to_string())],
        )
    }

    pub fn sample_write_failed(&self, path: &Path) -> String {
        interpolate(
            self.text("errors.sample_write_failed"),
            &[("path", path.display().to_string())],
        )
    }

    pub fn sample_added(&self, path: &Path) -> String {
        interpolate(self.text("init.sample_added"), &[("path", path.display().to_string())])
    }

    pub fn copy_definition_failed(&self, path: &Path) -> String {
        interpolate(
            self.text("errors.copy_definition_failed"),
            &[("path", path.display().to_string())],
        )
    }

    pub fn write_definition_failed(&self, path: &Path) -> String {
        interpolate(
            self.text("errors.write_definition_failed"),
            &[("path", path.display().to_string())],
        )
    }

    pub fn update_definition_failed(&self, path: &Path) -> String {
        interpolate(
            self.text("errors.update_definition_failed"),
            &[("path", path.display().to_string())],
        )
    }

    pub fn registered_server(&self, name: &str, id: &str, path: &Path) -> String {
        interpolate(
            self.text("add.registered"),
            &[
                ("name", name.to_string()),
                ("id", id.to_string()),
                ("path", path.display().to_string()),
            ],
        )
    }

    pub fn added_server(&self, name: &str, id: &str, path: &Path) -> String {
        interpolate(
            self.text("add.added"),
            &[
                ("name", name.to_string()),
                ("id", id.to_string()),
                ("path", path.display().to_string()),
            ],
        )
    }

    pub fn no_servers_registered(&self) -> &'static str {
        self.text("list.empty")
    }

    pub fn list_headers(
        &self,
    ) -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        (
            self.text("list.header.name"),
            self.text("list.header.enabled"),
            self.text("list.header.proto"),
            self.text("list.header.endpoint"),
            self.text("list.header.id"),
            self.text("list.header.command"),
        )
    }

    pub fn enabled_label(&self, enabled: bool) -> &'static str {
        if enabled {
            self.text("list.enabled.yes")
        } else {
            self.text("list.enabled.no")
        }
    }

    pub fn server_already_enabled(&self, name: &str) -> String {
        interpolate(self.text("enable.already"), &[("name", name.to_string())])
    }

    pub fn server_enabled(&self, name: &str) -> String {
        interpolate(self.text("enable.done"), &[("name", name.to_string())])
    }

    pub fn server_already_disabled(&self, name: &str) -> String {
        interpolate(self.text("disable.already"), &[("name", name.to_string())])
    }

    pub fn server_disabled(&self, name: &str) -> String {
        interpolate(self.text("disable.done"), &[("name", name.to_string())])
    }

    pub fn confirm_removal_prompt(&self, name: &str) -> String {
        interpolate(self.text("remove.prompt"), &[("name", name.to_string())])
    }

    pub fn removal_aborted(&self) -> &'static str {
        self.text("remove.aborted")
    }

    pub fn removal_done(&self, name: &str, id: &str) -> String {
        interpolate(self.text("remove.done"), &[("name", name.to_string()), ("id", id.to_string())])
    }

    pub fn inline_command_required(&self) -> &'static str {
        self.text("errors.inline_command_required")
    }

    pub fn inline_url_required(&self) -> &'static str {
        self.text("errors.url_required")
    }

    pub fn url_not_allowed_for_stdio(&self) -> &'static str {
        self.text("errors.url_not_allowed_stdio")
    }

    pub fn remote_command_forbidden(&self) -> &'static str {
        self.text("errors.remote_command_forbidden")
    }

    pub fn env_pair_format(&self) -> &'static str {
        self.text("errors.env_pair_format")
    }

    pub fn env_key_empty(&self) -> &'static str {
        self.text("errors.env_key_empty")
    }

    pub fn server_name_empty(&self) -> &'static str {
        self.text("errors.name_empty")
    }

    pub fn server_name_duplicate(&self, name: &str) -> String {
        interpolate(self.text("errors.name_duplicate"), &[("name", name.to_string())])
    }

    pub fn config_id_exists(&self, id: &str, path: &Path) -> String {
        interpolate(
            self.text("errors.config_id_exists"),
            &[("id", id.to_string()), ("path", path.display().to_string())],
        )
    }

    pub fn persist_path_unknown(&self) -> &'static str {
        self.text("errors.persist_path_unknown")
    }

    pub fn definition_missing_display_name(&self) -> &'static str {
        self.text("errors.definition_missing_display_name")
    }

    pub fn expand_home_missing(&self) -> &'static str {
        self.text("errors.expand_home_missing")
    }

    pub fn project_empty(&self) -> &'static str {
        self.text("project.list.empty")
    }

    pub fn project_headers(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        (
            self.text("project.list.header.project"),
            self.text("project.list.header.agent"),
            self.text("project.list.header.servers"),
            self.text("project.list.header.last_seen"),
        )
    }

    pub fn project_allow_done(&self, path: &Path, servers: &[String]) -> String {
        let placeholders = [("path", path.display().to_string()), ("servers", join_list(servers))];
        interpolate(self.text("project.allow.done"), &placeholders)
    }

    pub fn project_allow_unchanged(&self, path: &Path) -> String {
        interpolate(self.text("project.allow.unchanged"), &[("path", path.display().to_string())])
    }

    pub fn project_deny_done(&self, path: &Path, servers: &[String]) -> String {
        let placeholders = [("path", path.display().to_string()), ("servers", join_list(servers))];
        interpolate(self.text("project.deny.done"), &placeholders)
    }

    pub fn project_deny_unchanged(&self, path: &Path) -> String {
        interpolate(self.text("project.deny.unchanged"), &[("path", path.display().to_string())])
    }

    pub fn project_deny_missing(&self, path: &Path, servers: &[String]) -> String {
        interpolate(
            self.text("project.deny.missing"),
            &[("path", path.display().to_string()), ("servers", join_list(servers))],
        )
    }

    pub fn project_server_unknown(&self, name: &str) -> String {
        interpolate(self.text("project.server_unknown"), &[("name", name.to_string())])
    }

    pub fn project_record_missing(&self, path: &Path) -> String {
        interpolate(self.text("project.record_missing"), &[("path", path.display().to_string())])
    }

    pub fn render_anyhow(&self, err: &AnyhowError) -> String {
        if let Some(core) = err.downcast_ref::<CoreError>() {
            return self.render_core_error(core);
        }
        for cause in err.chain().skip(1) {
            if let Some(core) = cause.downcast_ref::<CoreError>() {
                return self.render_core_error(core);
            }
        }
        err.to_string()
    }

    // ========== 新增的翻译方法 ==========

    pub fn daemon_not_running(&self) -> &'static str {
        self.text("daemon.not_running")
    }

    pub fn no_tools_found(&self) -> &'static str {
        self.text("tools.none_found")
    }

    pub fn tools_from_server(&self, server: &str) -> String {
        interpolate(self.text("tools.from_server"), &[("server", server.to_string())])
    }

    pub fn all_tools(&self) -> &'static str {
        self.text("tools.all")
    }

    pub fn unexpected_response(&self) -> &'static str {
        self.text("rpc.unexpected_response")
    }

    pub fn rpc_error(&self) -> &'static str {
        self.text("rpc.error")
    }

    pub fn invalid_tool_spec(&self, spec: &str) -> String {
        interpolate(self.text("tools.invalid_spec"), &[("spec", spec.to_string())])
    }

    pub fn project_tools_allowed(&self, server: &str, tools: &str) -> String {
        interpolate(
            self.text("project.tools.allowed"),
            &[("server", server.to_string()), ("tools", tools.to_string())],
        )
    }

    pub fn project_tools_denied(&self, server: &str, tools: &str) -> String {
        interpolate(
            self.text("project.tools.denied"),
            &[("server", server.to_string()), ("tools", tools.to_string())],
        )
    }

    pub fn project_config_updated(&self, path: &str) -> String {
        interpolate(self.text("project.config_updated"), &[("path", path.to_string())])
    }

    pub fn tool_desc_set(&self, tool: &str) -> String {
        interpolate(self.text("tools.desc.set"), &[("tool", tool.to_string())])
    }

    pub fn tool_desc_reset(&self, tool: &str) -> String {
        interpolate(self.text("tools.desc.reset"), &[("tool", tool.to_string())])
    }

    pub fn tool_desc_not_customized(&self, tool: &str) -> String {
        interpolate(self.text("tools.desc.not_customized"), &[("tool", tool.to_string())])
    }

    pub fn project_not_found(&self, target: &str) -> String {
        interpolate(self.text("project.not_found"), &[("target", target.to_string())])
    }

    fn render_core_error(&self, error: &CoreError) -> String {
        let message = self.text(error.message_key());
        let placeholders = error.placeholders();
        interpolate(message, &placeholders)
    }

    fn text(&self, key: &str) -> &'static str {
        match self.language {
            Language::English => english_text(key),
            Language::SimplifiedChinese => zh_hans_text(key).unwrap_or_else(|| english_text(key)),
            Language::TraditionalChinese => zh_hant_text(key).unwrap_or_else(|| english_text(key)),
            Language::Japanese => ja_text(key).unwrap_or_else(|| english_text(key)),
        }
    }

    pub fn translate_placeholder(&self, candidate: &str) -> Option<&'static str> {
        let key = candidate.trim().strip_prefix(PLACEHOLDER_PREFIX)?;
        Some(self.text(key))
    }
}

pub fn localize_command(mut command: Command, messages: &Messages) -> Command {
    if let Some(about) = command
        .get_about()
        .and_then(|styled| messages.translate_placeholder(&styled.to_string()))
    {
        command = command.about(about);
    }
    if let Some(long_about) = command
        .get_long_about()
        .and_then(|styled| messages.translate_placeholder(&styled.to_string()))
    {
        command = command.long_about(long_about);
    }

    command = command.mut_args(|arg| localize_arg(arg, messages));
    command = command.mut_subcommands(|sub| localize_command(sub, messages));
    command
}

fn localize_arg(mut arg: Arg, messages: &Messages) -> Arg {
    if let Some(help) = arg
        .get_help()
        .and_then(|styled| messages.translate_placeholder(&styled.to_string()))
    {
        arg = arg.help(help);
    }

    if let Some(long_help) = arg
        .get_long_help()
        .and_then(|styled| messages.translate_placeholder(&styled.to_string()))
    {
        arg = arg.long_help(long_help);
    }

    arg
}

fn join_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

fn english_text(key: &str) -> &'static str {
    match key {
        "cli.about" => "MCP Center CLI",
        "cli.version_flag_help" => "Show version information and exit.",
        "cli.root_help" => "Override the root directory for all data.",
        "command.init.about" => "Initialize the workspace structure and create sample configs.",
        "command.serve.about" => "Start the MCP server management daemon",
        "command.connect.about" => "Connect AI agents to the daemon (stdin/stdout bridge)",
        "command.mcp.about" => "Manage MCP server definitions.",
        "command.mcp.add.about" => "Add a server definition from file or inline command.",
        "command.mcp.list.about" => "List registered MCP servers.",
        "command.mcp.info.about" => "Show details for a server.",
        "command.mcp.remove.about" => "Remove a server definition.",
        "command.mcp.enable.about" => "Enable a server by name.",
        "command.mcp.disable.about" => "Disable a server by name.",
        "command.project.about" => "Manage project-to-server mappings.",
        "command.project.add.about" => "Add a new project.",
        "command.project.remove.about" => "Remove a project.",
        "command.project.list.about" => "List known projects and their allowed servers.",
        "command.project.allow.about" => "Allow a project to use specific servers.",
        "command.project.deny.about" => "Remove servers from a project's allow list.",
        "args.serve.http_bind" => {
            "Bind the HTTP API to the specified address (e.g. 127.0.0.1:8787)."
        }
        "args.serve.http_auth_token" => {
            "Authentication token required for the HTTP API (or set MCP_CENTER_HTTP_TOKEN)."
        }
        "args.mcp_add.name_or_path" => {
            "Server config path (toml/json) or server display name when using inline command form."
        }
        "args.mcp_add.name" => "Optional friendly display name when using inline command form.",
        "args.mcp_add.protocol" => {
            "MCP protocol when using inline command form. Defaults to 'stdio'."
        }
        "args.mcp_add.url" => "Remote endpoint URL (required for 'sse' or 'http' protocols).",
        "args.mcp_add.env" => {
            "Environment variables in KEY=VALUE form (only for inline command form)."
        }
        "args.mcp_add.command" => "Command to execute, specified after '--' (only inline form).",
        "args.mcp_name" => "MCP server display name.",
        "args.mcp_remove.name" => "MCP server display name to remove.",
        "args.mcp_remove.yes" => "Remove without prompting for confirmation.",
        "args.project.target" => "Project path (recommended) or identifier.",
        "args.project.servers" => "One or more MCP server IDs.",
        "args.project_add.path" => "Project path (optional, defaults to current directory).",
        "args.project_remove.target" => "Project path or ID to remove.",
        "args.project_remove.yes" => "Remove without prompting for confirmation.",
        "init.workspace_exists" => "Workspace initialized at {root}",
        "init.workspace_created" => "Workspace initialized at {root}\nSample config: {sample}",
        "init.sample_added" => "Added sample config: {path}",
        "list.empty" => "No MCP servers registered. Use `mcp-center mcp add` to add one.",
        "list.header.name" => "NAME",
        "list.header.enabled" => "ENABLED",
        "list.header.proto" => "PROTO",
        "list.header.endpoint" => "ENDPOINT",
        "list.header.id" => "ID",
        "list.header.command" => "COMMAND",
        "project.list.empty" => "No projects recorded yet. Run mcp-center-bridge to register one.",
        "project.list.header.project" => "PROJECT",
        "project.list.header.agent" => "AGENT",
        "project.list.header.servers" => "ALLOWED MCP SERVER IDS",
        "project.list.header.last_seen" => "LAST SEEN",
        "project.allow.done" => "Updated {path} allowed MCP server IDs → {servers}.",
        "project.allow.unchanged" => "No changes for {path}; MCP server IDs already allowed.",
        "project.deny.done" => "Removed {servers} from {path} allowed MCP server IDs.",
        "project.deny.unchanged" => "No changes for {path}; no MCP server IDs to remove.",
        "project.deny.missing" => {
            "Skipping removal for {path}; MCP server IDs not allowed: {servers}."
        }
        "project.server_unknown" => {
            "Unknown MCP server ID '{name}'. Add it with `mcp-center mcp add` first."
        }
        "project.record_missing" => {
            "Project not found for path {path}. Run mcp-center-bridge once to register it."
        }
        "list.enabled.yes" => "yes",
        "list.enabled.no" => "no",
        "enable.already" => "MCP server '{name}' is already enabled.",
        "enable.done" => "Enabled MCP server '{name}'.",
        "disable.already" => "MCP server '{name}' is already disabled.",
        "disable.done" => "Disabled MCP server '{name}'.",
        "remove.prompt" => "Remove MCP server '{name}'? [y/N]: ",
        "remove.aborted" => "Aborted.",
        "remove.done" => "Removed MCP server '{name}' (id {id}).",
        "add.registered" => "Registered MCP server '{name}' (id {id}) -> {path}",
        "add.added" => "Added MCP server '{name}' (id {id}) -> {path}",
        "errors.prefix" => "Error:",
        "errors.inline_command_required" => "Inline form requires a command after `--`.",
        "errors.url_required" => "Remote protocols require specifying --url.",
        "errors.url_not_allowed_stdio" => "--url cannot be used with the 'stdio' protocol.",
        "errors.remote_command_forbidden" => {
            "Command arguments are not supported for remote protocols."
        }
        "errors.env_pair_format" => "Environment variables must be in KEY=VALUE form.",
        "errors.env_key_empty" => "Environment variable key cannot be empty.",
        "errors.name_empty" => "MCP server name cannot be empty.",
        "errors.name_duplicate" => "MCP server name '{name}' already exists.",
        "errors.config_id_exists" => "MCP server config for id '{id}' already exists at {path}",
        "errors.copy_definition_failed" => "Failed to copy MCP server definition to {path}",
        "errors.write_definition_failed" => "Failed to write MCP server definition to {path}",
        "errors.update_definition_failed" => "Failed to update MCP server definition at {path}",
        "errors.sample_write_failed" => "Failed to write sample config to {path}",
        "errors.persist_path_unknown" => "MCP server configuration path is unknown",
        "errors.definition_missing_display_name" => "MCP server definition missing display name",
        "errors.expand_home_missing" => "cannot expand '~', HOME is not set",
        "core.server_name_empty" => "MCP server name cannot be empty",
        "core.server_name_empty_with_id" => "MCP server name cannot be empty (id {id})",
        "core.server_command_empty" => "MCP server command cannot be empty",
        "core.server_command_empty_with_id" => "MCP server command cannot be empty (id {id})",
        "core.unsupported_protocol" => "Unsupported protocol for MCP server",
        "core.unsupported_protocol_with_id" => "Unsupported protocol for MCP server (id {id})",
        "core.server_endpoint_missing" => "MCP server endpoint is required for remote protocols",
        "core.server_endpoint_missing_with_id" => {
            "MCP server endpoint is required for remote protocols (id {id})"
        }
        "core.server_endpoint_invalid" => "Invalid MCP server endpoint '{endpoint}': {error}",
        "core.server_endpoint_invalid_with_id" => {
            "Invalid MCP server endpoint for id {id} ('{endpoint}'): {error}"
        }
        "core.server_config_not_found" => "MCP server configuration '{id}' not found",
        "core.server_config_not_found_name" => "MCP server '{name}' not found",
        "core.create_dir_failed" => "Failed to create directory {path}: {error}",
        "core.read_dir_failed" => "Failed to read directory {path}: {error}",
        "core.read_config_failed" => "Failed to read config file {path}: {error}",
        "core.parse_json_failed" => "Failed to parse JSON server config at {path}: {error}",
        "core.parse_toml_failed" => "Failed to parse TOML server config at {path}: {error}",
        "core.serialise_toml_failed" => "Failed to serialise server definition to TOML: {error}",
        "core.remove_file_failed" => "Failed to remove {path}: {error}",
        "core.home_dir_unknown" => "Unable to determine user home directory for MCP_CENTER_ROOT",
        // New translations for tool-level permissions
        "command.mcp.list_tools.about" => "List all tools from MCP servers",
        "command.project.allow_tools.about" => "Allow specific tools for a project",
        "command.project.deny_tools.about" => "Deny specific tools for a project",
        "command.project.set_tool_desc.about" => "Set custom description for a tool",
        "command.project.reset_tool_desc.about" => "Reset tool description to default",
        "args.mcp_list_tools.server" => "Optional server name to filter tools",
        "args.project_tools.target" => "Project path or ID",
        "args.project_tools.tools" => "Tools in SERVER::TOOL format",
        "args.project_tool_desc.target" => "Project path or ID",
        "args.project_tool_desc.tool_name" => "Tool name",
        "args.project_tool_desc.description" => "Custom description",
        "args.project_reset_tool_desc.target" => "Project path or ID",
        "args.project_reset_tool_desc.tool_name" => "Tool name to reset",
        "daemon.not_running" => "Daemon is not running. Start it with 'mcp-center serve'",
        "tools.none_found" => "No tools found",
        "tools.from_server" => "Tools from server '{server}':",
        "tools.all" => "All available tools:",
        "rpc.unexpected_response" => "Unexpected response from daemon",
        "rpc.error" => "RPC error",
        "tools.invalid_spec" => "Invalid tool spec '{spec}'. Expected format: SERVER::TOOL",
        "project.tools.allowed" => "Allowed tools from '{server}': {tools}",
        "project.tools.denied" => "Denied tools from '{server}': {tools}",
        "project.config_updated" => "Project configuration updated: {path}",
        "tools.desc.set" => "Custom description set for tool '{tool}'",
        "tools.desc.reset" => "Description reset to default for tool '{tool}'",
        "tools.desc.not_customized" => "Tool '{tool}' has no custom description",
        "project.not_found" => "Project not found: {target}",
        _ => panic!("missing English text for key '{key}'"),
    }
}

fn zh_hans_text(key: &str) -> Option<&'static str> {
    Some(match key {
        "cli.about" => "MCP Center CLI",
        "cli.version_flag_help" => "显示版本信息并退出。",
        "cli.root_help" => "覆盖所有数据的根目录。",
        "command.init.about" => "初始化工作区结构并创建示例配置。",
        "command.serve.about" => "启动 MCP 服务器管理守护进程",
        "command.connect.about" => "将 AI 代理连接到守护进程（stdin/stdout 桥接）",
        "command.mcp.about" => "管理 MCP 服务器定义。",
        "command.mcp.add.about" => "通过文件或命令行添加 MCP 服务器定义。",
        "command.mcp.list.about" => "列出已注册的 MCP 服务器。",
        "command.mcp.info.about" => "查看 MCP 服务器的详细信息。",
        "command.mcp.remove.about" => "移除 MCP 服务器定义。",
        "command.mcp.enable.about" => "按名称启用 MCP 服务器。",
        "command.mcp.disable.about" => "按名称禁用 MCP 服务器。",
        "command.project.about" => "管理项目与 MCP 服务器的关联。",
        "command.project.add.about" => "添加新项目。",
        "command.project.remove.about" => "移除项目。",
        "command.project.list.about" => "列出已记录的项目及其允许使用的 MCP 服务器。",
        "command.project.allow.about" => "为项目允许使用指定 MCP 服务器。",
        "command.project.deny.about" => "从项目的允许列表中移除 MCP 服务器。",
        "args.serve.http_bind" => "绑定 HTTP API 的监听地址（例如 127.0.0.1:8787）。",
        "args.serve.http_auth_token" => {
            "设置 HTTP API 鉴权 Token（或使用 MCP_CENTER_HTTP_TOKEN）。"
        }
        "args.mcp_add.name_or_path" => {
            "使用文件时为配置路径（toml/json），使用命令行时为 MCP 服务器显示名称。"
        }
        "args.mcp_add.name" => "使用命令行形式时的可选显示名称。",
        "args.mcp_add.protocol" => "使用命令行形式时的 MCP 协议，默认为 'stdio'。",
        "args.mcp_add.url" => "远程端点 URL（使用 'sse' 或 'http' 协议时必填）。",
        "args.mcp_add.env" => "仅在命令行形式下使用的环境变量（KEY=VALUE）。",
        "args.mcp_add.command" => "仅在命令行形式下，需在 `--` 之后指定的命令。",
        "args.mcp_name" => "MCP 服务器显示名称。",
        "args.mcp_remove.name" => "要移除的 MCP 服务器显示名称。",
        "args.mcp_remove.yes" => "跳过确认直接移除。",
        "args.project.target" => "项目路径（推荐）或标识符。",
        "args.project.servers" => "一个或多个 MCP 服务器 ID。",
        "args.project_add.path" => "项目路径（可选，默认为当前目录）。",
        "args.project_remove.target" => "要移除的项目路径或 ID。",
        "args.project_remove.yes" => "跳过确认直接移除。",
        "init.workspace_exists" => "工作区已初始化：{root}",
        "init.workspace_created" => "工作区已初始化：{root}\n示例配置：{sample}",
        "init.sample_added" => "已添加示例配置：{path}",
        "list.empty" => "当前没有注册的 MCP 服务器，可使用 `mcp-center mcp add` 添加。",
        "list.header.name" => "名称",
        "list.header.enabled" => "启用",
        "list.header.proto" => "协议",
        "list.header.endpoint" => "端点",
        "list.header.id" => "ID",
        "list.header.command" => "命令",
        "project.list.empty" => "当前没有记录项目，可运行 mcp-center-bridge 注册一个。",
        "project.list.header.project" => "项目",
        "project.list.header.agent" => "代理",
        "project.list.header.servers" => "允许的 MCP 服务器 ID",
        "project.list.header.last_seen" => "最近活跃",
        "project.allow.done" => "已更新 {path} 的 MCP 服务器 ID 允许列表 → {servers}。",
        "project.allow.unchanged" => "{path} 的 MCP 服务器 ID 允许列表未改变，目标已在列表中。",
        "project.deny.done" => "已从 {path} 的 MCP 服务器 ID 允许列表移除 {servers}。",
        "project.deny.unchanged" => "{path} 的 MCP 服务器 ID 允许列表未改变，无需移除。",
        "project.deny.missing" => {
            "忽略 {path} 的移除操作，以下 MCP 服务器 ID 不在列表中：{servers}。"
        }
        "project.server_unknown" => {
            "未知的 MCP 服务器 ID \"{name}\"。请先使用 `mcp-center mcp add` 添加。"
        }
        "project.record_missing" => {
            "未找到路径 {path} 对应的项目。请先运行 mcp-center-bridge 注册。"
        }
        "list.enabled.yes" => "是",
        "list.enabled.no" => "否",
        "enable.already" => "MCP 服务器“{name}”已处于启用状态。",
        "enable.done" => "已启用 MCP 服务器“{name}”。",
        "disable.already" => "MCP 服务器“{name}”已处于禁用状态。",
        "disable.done" => "已禁用 MCP 服务器“{name}”。",
        "remove.prompt" => "确定移除 MCP 服务器“{name}”？[y/N]：",
        "remove.aborted" => "已取消。",
        "remove.done" => "已移除 MCP 服务器“{name}”（ID {id}）。",
        "add.registered" => "已注册 MCP 服务器“{name}”（ID {id}）-> {path}",
        "add.added" => "已添加 MCP 服务器“{name}”（ID {id}）-> {path}",
        "errors.prefix" => "错误：",
        "errors.inline_command_required" => "命令行形式需要在 `--` 之后提供命令。",
        "errors.url_required" => "远程协议必须提供 --url 参数。",
        "errors.url_not_allowed_stdio" => "使用 'stdio' 协议时不能指定 --url。",
        "errors.remote_command_forbidden" => "远程协议不支持附加命令。",
        "errors.env_pair_format" => "环境变量需要采用 KEY=VALUE 格式。",
        "errors.env_key_empty" => "环境变量的键不能为空。",
        "errors.name_empty" => "MCP 服务器名称不能为空。",
        "errors.name_duplicate" => "MCP 服务器名称“{name}”已存在。",
        "errors.config_id_exists" => "ID 为“{id}”的 MCP 服务器配置已存在：{path}",
        "errors.copy_definition_failed" => "复制 MCP 服务器定义到 {path} 失败",
        "errors.write_definition_failed" => "写入 MCP 服务器定义到 {path} 失败",
        "errors.update_definition_failed" => "更新 MCP 服务器定义失败：{path}",
        "errors.sample_write_failed" => "写入示例配置到 {path} 失败",
        "errors.persist_path_unknown" => "无法确定 MCP 服务器配置文件路径",
        "errors.definition_missing_display_name" => "MCP 服务器定义缺少显示名称",
        "errors.expand_home_missing" => "无法展开 '~'，未设置 HOME 环境变量",
        "core.server_name_empty" => "MCP 服务器名称不能为空。",
        "core.server_name_empty_with_id" => "MCP 服务器名称不能为空（ID {id}）。",
        "core.server_command_empty" => "MCP 服务器命令不能为空。",
        "core.server_command_empty_with_id" => "MCP 服务器命令不能为空（ID {id}）。",
        "core.unsupported_protocol" => "不支持的 MCP 服务器协议。",
        "core.unsupported_protocol_with_id" => "不支持的 MCP 服务器协议（ID {id}）。",
        "core.server_endpoint_missing" => "远程协议需要配置 MCP 服务器端点。",
        "core.server_endpoint_missing_with_id" => "远程协议需要配置 MCP 服务器端点（ID {id}）。",
        "core.server_endpoint_invalid" => "MCP 服务器端点“{endpoint}”无效：{error}",
        "core.server_endpoint_invalid_with_id" => {
            "MCP 服务器端点无效（ID {id}，“{endpoint}”）：{error}"
        }
        "core.server_config_not_found" => "未找到 ID 为“{id}”的 MCP 服务器配置。",
        "core.server_config_not_found_name" => "未找到名称为“{name}”的 MCP 服务器。",
        "core.create_dir_failed" => "创建目录 {path} 失败：{error}",
        "core.read_dir_failed" => "读取目录 {path} 失败：{error}",
        "core.read_config_failed" => "读取配置文件 {path} 失败：{error}",
        "core.parse_json_failed" => "解析 {path} 的 JSON MCP 服务器配置失败：{error}",
        "core.parse_toml_failed" => "解析 {path} 的 TOML MCP 服务器配置失败：{error}",
        "core.serialise_toml_failed" => "序列化 MCP 服务器定义到 TOML 失败：{error}",
        "core.remove_file_failed" => "删除 {path} 失败：{error}",
        "core.home_dir_unknown" => "无法确定用户主目录（MCP_CENTER_ROOT）",
        // 工具级权限控制新增翻译
        "command.mcp.list_tools.about" => "列出 MCP 服务器的所有工具",
        "command.project.allow_tools.about" => "允许项目使用特定工具",
        "command.project.deny_tools.about" => "禁止项目使用特定工具",
        "command.project.set_tool_desc.about" => "为工具设置自定义描述",
        "command.project.reset_tool_desc.about" => "重置工具描述为默认值",
        "args.mcp_list_tools.server" => "可选的服务器名称（用于过滤）",
        "args.project_tools.target" => "项目路径或 ID",
        "args.project_tools.tools" => "工具列表（格式：SERVER::TOOL）",
        "args.project_tool_desc.target" => "项目路径或 ID",
        "args.project_tool_desc.tool_name" => "工具名称",
        "args.project_tool_desc.description" => "自定义描述",
        "args.project_reset_tool_desc.target" => "项目路径或 ID",
        "args.project_reset_tool_desc.tool_name" => "要重置的工具名称",
        "daemon.not_running" => "守护进程未运行。请使用 'mcp-center serve' 启动",
        "tools.none_found" => "未找到任何工具",
        "tools.from_server" => "来自服务器 '{server}' 的工具：",
        "tools.all" => "所有可用工具：",
        "rpc.unexpected_response" => "守护进程返回意外响应",
        "rpc.error" => "RPC 错误",
        "tools.invalid_spec" => "无效的工具规格 '{spec}'。期望格式：SERVER::TOOL",
        "project.tools.allowed" => "已允许来自 '{server}' 的工具：{tools}",
        "project.tools.denied" => "已禁止来自 '{server}' 的工具：{tools}",
        "project.config_updated" => "项目配置已更新：{path}",
        "tools.desc.set" => "已为工具 '{tool}' 设置自定义描述",
        "tools.desc.reset" => "已将工具 '{tool}' 的描述重置为默认值",
        "tools.desc.not_customized" => "工具 '{tool}' 没有自定义描述",
        "project.not_found" => "未找到项目：{target}",
        other => return Some(english_text(other)),
    })
}

fn zh_hant_text(key: &str) -> Option<&'static str> {
    Some(match key {
        "cli.about" => "MCP Center CLI",
        "cli.version_flag_help" => "顯示版本資訊並退出。",
        "cli.root_help" => "覆寫所有資料的根目錄。",
        "command.init.about" => "初始化工作區結構並建立範例設定。",
        "command.serve.about" => "啟動 MCP 伺服器管理守護行程",
        "command.connect.about" => "將 AI 代理連接到守護行程（stdin/stdout 橋接）",
        "command.mcp.about" => "管理 MCP 伺服器定義。",
        "command.mcp.add.about" => "透過檔案或命令列加入伺服器定義。",
        "command.mcp.list.about" => "列出已註冊的 MCP 伺服器。",
        "command.mcp.info.about" => "檢視伺服器的詳細資訊。",
        "command.mcp.remove.about" => "移除伺服器定義。",
        "command.mcp.enable.about" => "依名稱啟用伺服器。",
        "command.mcp.disable.about" => "依名稱停用伺服器。",
        "command.project.about" => "管理專案與伺服器的對應關係。",
        "command.project.add.about" => "新增專案。",
        "command.project.remove.about" => "移除專案。",
        "command.project.list.about" => "列出已記錄的專案與允許使用的伺服器。",
        "command.project.allow.about" => "將指定伺服器加入專案的允許清單。",
        "command.project.deny.about" => "從專案的允許清單移除伺服器。",
        "args.serve.http_bind" => "綁定 HTTP API 的監聽位址（例如 127.0.0.1:8787）。",
        "args.serve.http_auth_token" => {
            "設定 HTTP API 鑑權 Token（或使用 MCP_CENTER_HTTP_TOKEN）。"
        }
        "args.mcp_add.name_or_path" => {
            "使用檔案時為設定路徑（toml/json），使用命令列時為伺服器顯示名稱。"
        }
        "args.mcp_add.name" => "命令列形式的可選顯示名稱。",
        "args.mcp_add.protocol" => "命令列形式的 MCP 協定，預設為 'stdio'。",
        "args.mcp_add.url" => "遠端端點 URL（使用 'sse' 或 'http' 協定時必填）。",
        "args.mcp_add.env" => "僅用於命令列形式的環境變數（KEY=VALUE）。",
        "args.mcp_add.command" => "命令列形式下必須在 `--` 後提供的指令。",
        "args.mcp_name" => "MCP 伺服器顯示名稱。",
        "args.mcp_remove.name" => "要移除的 MCP 伺服器顯示名稱。",
        "args.mcp_remove.yes" => "略過確認直接移除。",
        "args.project.target" => "專案路徑（建議）或識別碼。",
        "args.project.servers" => "一個或多個 MCP 伺服器 ID。",
        "args.project_add.path" => "專案路徑（可選，預設為目前目錄）。",
        "args.project_remove.target" => "要移除的專案路徑或 ID。",
        "args.project_remove.yes" => "略過確認直接移除。",
        "init.workspace_exists" => "工作區已初始化：{root}",
        "init.workspace_created" => "工作區已初始化：{root}\n範例設定：{sample}",
        "init.sample_added" => "已新增範例設定：{path}",
        "list.empty" => "目前沒有註冊的 MCP 伺服器，可使用 `mcp-center mcp add` 加入。",
        "list.header.name" => "名稱",
        "list.header.enabled" => "啟用",
        "list.header.proto" => "協定",
        "list.header.endpoint" => "端點",
        "list.header.id" => "ID",
        "list.header.command" => "指令",
        "project.list.empty" => "目前沒有記錄任何專案，可先執行 mcp-center-bridge 進行註冊。",
        "project.list.header.project" => "專案",
        "project.list.header.agent" => "代理",
        "project.list.header.servers" => "允許的 MCP 伺服器 ID",
        "project.list.header.last_seen" => "最近使用",
        "project.allow.done" => "已更新 {path} 的 MCP 伺服器 ID 允許清單 → {servers}。",
        "project.allow.unchanged" => "{path} 的 MCP 伺服器 ID 允許清單未變更，目標已在名單中。",
        "project.deny.done" => "已從 {path} 的 MCP 伺服器 ID 允許清單移除 {servers}。",
        "project.deny.unchanged" => "{path} 的 MCP 伺服器 ID 允許清單未變更，無需移除。",
        "project.deny.missing" => {
            "略過 {path} 的移除操作，下列 MCP 伺服器 ID 不在清單中：{servers}。"
        }
        "project.server_unknown" => {
            "未知的 MCP 伺服器 ID「{name}」。請先執行 `mcp-center mcp add`。"
        }
        "project.record_missing" => {
            "找不到路徑 {path} 對應的專案，請先執行 mcp-center-bridge 註冊。"
        }
        "list.enabled.yes" => "是",
        "list.enabled.no" => "否",
        "enable.already" => "MCP 伺服器「{name}」已經啟用。",
        "enable.done" => "已啟用 MCP 伺服器「{name}」。",
        "disable.already" => "MCP 伺服器「{name}」已經停用。",
        "disable.done" => "已停用 MCP 伺服器「{name}」。",
        "remove.prompt" => "確定要移除 MCP 伺服器「{name}」？[y/N]：",
        "remove.aborted" => "已取消。",
        "remove.done" => "已移除 MCP 伺服器「{name}」（ID {id}）。",
        "add.registered" => "已註冊 MCP 伺服器「{name}」（ID {id}）-> {path}",
        "add.added" => "已加入 MCP 伺服器「{name}」（ID {id}）-> {path}",
        "errors.prefix" => "錯誤：",
        "errors.inline_command_required" => "命令列形式須在 `--` 後提供指令。",
        "errors.url_required" => "遠端協定必須提供 --url 參數。",
        "errors.url_not_allowed_stdio" => "使用 'stdio' 協定時不可指定 --url。",
        "errors.remote_command_forbidden" => "遠端協定不支援額外指令。",
        "errors.env_pair_format" => "環境變數必須採用 KEY=VALUE 格式。",
        "errors.env_key_empty" => "環境變數的鍵不可為空。",
        "errors.name_empty" => "MCP 伺服器名稱不可為空。",
        "errors.name_duplicate" => "MCP 伺服器名稱「{name}」已存在。",
        "errors.config_id_exists" => "ID 為「{id}」的 MCP 伺服器設定已存在：{path}",
        "errors.copy_definition_failed" => "複製 MCP 伺服器定義到 {path} 失敗",
        "errors.write_definition_failed" => "寫入 MCP 伺服器定義到 {path} 失敗",
        "errors.update_definition_failed" => "更新 MCP 伺服器定義失敗：{path}",
        "errors.sample_write_failed" => "寫入範例設定到 {path} 失敗",
        "errors.persist_path_unknown" => "無法取得 MCP 伺服器設定檔路徑",
        "errors.definition_missing_display_name" => "MCP 伺服器定義缺少顯示名稱",
        "errors.expand_home_missing" => "無法展開 '~'，未設定 HOME 環境變數",
        "core.server_name_empty" => "MCP 伺服器名稱不可為空。",
        "core.server_name_empty_with_id" => "MCP 伺服器名稱不可為空（ID {id}）。",
        "core.server_command_empty" => "MCP 伺服器指令不可為空。",
        "core.server_command_empty_with_id" => "MCP 伺服器指令不可為空（ID {id}）。",
        "core.unsupported_protocol" => "不支援的 MCP 伺服器通訊協定。",
        "core.unsupported_protocol_with_id" => "不支援的 MCP 伺服器通訊協定（ID {id}）。",
        "core.server_endpoint_missing" => "遠端協定需要設定 MCP 伺服器端點。",
        "core.server_endpoint_missing_with_id" => "遠端協定需要設定 MCP 伺服器端點（ID {id}）。",
        "core.server_endpoint_invalid" => "MCP 伺服器端點「{endpoint}」無效：{error}",
        "core.server_endpoint_invalid_with_id" => {
            "MCP 伺服器端點無效（ID {id}，「{endpoint}」）：{error}"
        }
        "core.server_config_not_found" => "找不到 ID 為「{id}」的 MCP 伺服器設定。",
        "core.server_config_not_found_name" => "找不到名稱為「{name}」的 MCP 伺服器。",
        "core.create_dir_failed" => "建立目錄 {path} 失敗：{error}",
        "core.read_dir_failed" => "讀取目錄 {path} 失敗：{error}",
        "core.read_config_failed" => "讀取設定檔 {path} 失敗：{error}",
        "core.parse_json_failed" => "解析 {path} 的 JSON 伺服器設定失敗：{error}",
        "core.parse_toml_failed" => "解析 {path} 的 TOML 伺服器設定失敗：{error}",
        "core.serialise_toml_failed" => "序列化伺服器定義到 TOML 失敗：{error}",
        "core.remove_file_failed" => "刪除 {path} 失敗：{error}",
        "core.home_dir_unknown" => "無法判定使用者家目錄（MCP_CENTER_ROOT）",
        other => return Some(english_text(other)),
    })
}

fn ja_text(key: &str) -> Option<&'static str> {
    Some(match key {
        "cli.about" => "MCP Center CLI",
        "cli.version_flag_help" => "バージョン情報を表示して終了します。",
        "cli.root_help" => "すべてのデータのルートディレクトリを上書きします。",
        "command.init.about" => "ワークスペース構成を初期化し、サンプル設定を作成します。",
        "command.serve.about" => "MCP サーバー管理デーモンを起動します",
        "command.connect.about" => "AI エージェントをデーモンに接続します（stdin/stdout ブリッジ）",
        "command.mcp.about" => "MCP サーバー定義を管理します。",
        "command.mcp.add.about" => "ファイルまたはコマンドラインからサーバー定義を追加します。",
        "command.mcp.list.about" => "登録済みの MCP サーバーを一覧表示します。",
        "command.mcp.info.about" => "サーバーの詳細情報を表示します。",
        "command.mcp.remove.about" => "サーバー定義を削除します。",
        "command.mcp.enable.about" => "名前でサーバーを有効化します。",
        "command.mcp.disable.about" => "名前でサーバーを無効化します。",
        "command.project.about" => "プロジェクトとサーバーの対応を管理します。",
        "command.project.add.about" => "新しいプロジェクトを追加します。",
        "command.project.remove.about" => "プロジェクトを削除します。",
        "command.project.list.about" => {
            "登録済みプロジェクトと許可されたサーバーを一覧表示します。"
        }
        "command.project.allow.about" => "プロジェクトに利用可能なサーバーを追加します。",
        "command.project.deny.about" => "プロジェクトの許可リストからサーバーを削除します。",
        "args.serve.http_bind" => {
            "HTTP API をバインドするアドレスを指定します（例: 127.0.0.1:8787）。"
        }
        "args.serve.http_auth_token" => {
            "HTTP API 用の認証トークンを設定します（または MCP_CENTER_HTTP_TOKEN を使用）。"
        }
        "args.mcp_add.name_or_path" => {
            "ファイル形式では設定ファイル（toml/json）のパス、コマンド形式ではサーバー表示名を指定します。"
        }
        "args.mcp_add.name" => "コマンド形式で使用する任意の表示名。",
        "args.mcp_add.protocol" => "コマンド形式で使用する MCP プロトコル。既定値は 'stdio'。",
        "args.mcp_add.url" => {
            "リモートエンドポイント URL（'sse' または 'http' プロトコルでは必須）。"
        }
        "args.mcp_add.env" => "コマンド形式でのみ使用する環境変数（KEY=VALUE）。",
        "args.mcp_add.command" => "コマンド形式では `--` の後に実行コマンドを指定します。",
        "args.mcp_name" => "MCP サーバーの表示名。",
        "args.mcp_remove.name" => "削除する MCP サーバーの表示名。",
        "args.mcp_remove.yes" => "確認を省略して削除します。",
        "args.project.target" => "プロジェクトのパス（推奨）または ID。",
        "args.project.servers" => "1 個以上の MCP サーバー ID。",
        "args.project_add.path" => "プロジェクトのパス（オプション、既定値は現在のディレクトリ）。",
        "args.project_remove.target" => "削除するプロジェクトのパスまたは ID。",
        "args.project_remove.yes" => "確認を省略して削除します。",
        "init.workspace_exists" => "ワークスペースはすでに初期化されています: {root}",
        "init.workspace_created" => {
            "ワークスペースを初期化しました: {root}\nサンプル設定: {sample}"
        }
        "init.sample_added" => "サンプル設定を追加しました: {path}",
        "list.empty" => {
            "登録済みの MCP サーバーはありません。`mcp-center mcp add` で追加できます。"
        }
        "list.header.name" => "名称",
        "list.header.enabled" => "有効",
        "list.header.proto" => "プロトコル",
        "list.header.endpoint" => "エンドポイント",
        "list.header.id" => "ID",
        "list.header.command" => "コマンド",
        "project.list.empty" => {
            "まだプロジェクトが登録されていません。mcp-center-bridge を実行してください。"
        }
        "project.list.header.project" => "プロジェクト",
        "project.list.header.agent" => "エージェント",
        "project.list.header.servers" => "許可済み MCP サーバー ID",
        "project.list.header.last_seen" => "最終更新",
        "project.allow.done" => "{path} の MCP サーバー ID 許可リストを更新しました → {servers}。",
        "project.allow.unchanged" => {
            "{path} の MCP サーバー ID 許可リストは変更されませんでした。すでに登録済みです。"
        }
        "project.deny.done" => {
            "{path} の MCP サーバー ID 許可リストから {servers} を削除しました。"
        }
        "project.deny.unchanged" => {
            "{path} の MCP サーバー ID 許可リストは変更されませんでした。削除対象がありません。"
        }
        "project.deny.missing" => {
            "{path} の削除をスキップしました。リストにない MCP サーバー ID: {servers}。"
        }
        "project.server_unknown" => {
            "MCP サーバー ID『{name}』が見つかりません。先に `mcp-center mcp add` で登録してください。"
        }
        "project.record_missing" => {
            "パス {path} に対応するプロジェクトが見つかりません。mcp-center-bridge を実行してください。"
        }
        "list.enabled.yes" => "有効",
        "list.enabled.no" => "無効",
        "enable.already" => "MCP サーバー「{name}」は既に有効です。",
        "enable.done" => "MCP サーバー「{name}」を有効化しました。",
        "disable.already" => "MCP サーバー「{name}」は既に無効です。",
        "disable.done" => "MCP サーバー「{name}」を無効化しました。",
        "remove.prompt" => "MCP サーバー「{name}」を削除しますか？[y/N]: ",
        "remove.aborted" => "キャンセルしました。",
        "remove.done" => "MCP サーバー「{name}」（ID {id}）を削除しました。",
        "add.registered" => "MCP サーバー「{name}」（ID {id}）を登録しました -> {path}",
        "add.added" => "MCP サーバー「{name}」（ID {id}）を追加しました -> {path}",
        "errors.prefix" => "エラー:",
        "errors.inline_command_required" => "コマンド形式では `--` の後にコマンドが必要です。",
        "errors.url_required" => "リモートプロトコルでは --url の指定が必要です。",
        "errors.url_not_allowed_stdio" => "'stdio' プロトコルでは --url を指定できません。",
        "errors.remote_command_forbidden" => {
            "リモートプロトコルでは追加のコマンドは使用できません。"
        }
        "errors.env_pair_format" => "環境変数は KEY=VALUE 形式で指定してください。",
        "errors.env_key_empty" => "環境変数のキーは空にできません。",
        "errors.name_empty" => "MCP サーバー名は必須です。",
        "errors.name_duplicate" => "MCP サーバー名「{name}」は既に存在します。",
        "errors.config_id_exists" => "ID「{id}」の MCP サーバー設定は既に存在します: {path}",
        "errors.copy_definition_failed" => "MCP サーバー定義のコピーに失敗しました: {path}",
        "errors.write_definition_failed" => "MCP サーバー定義の書き込みに失敗しました: {path}",
        "errors.update_definition_failed" => "MCP サーバー定義の更新に失敗しました: {path}",
        "errors.sample_write_failed" => "サンプル設定の書き込みに失敗しました: {path}",
        "errors.persist_path_unknown" => "MCP サーバー設定ファイルの場所を特定できません",
        "errors.definition_missing_display_name" => "MCP サーバー定義に表示名がありません",
        "errors.expand_home_missing" => "'~' を展開できません。HOME が設定されていません",
        "core.server_name_empty" => "MCP サーバー名は必須です。",
        "core.server_name_empty_with_id" => "MCP サーバー名は必須です（ID {id}）。",
        "core.server_command_empty" => "MCP サーバーのコマンドは必須です。",
        "core.server_command_empty_with_id" => "MCP サーバーのコマンドは必須です（ID {id}）。",
        "core.unsupported_protocol" => "MCP サーバーのプロトコルがサポートされていません。",
        "core.unsupported_protocol_with_id" => {
            "MCP サーバーのプロトコルがサポートされていません（ID {id}）。"
        }
        "core.server_endpoint_missing" => {
            "リモートプロトコルには MCP サーバーのエンドポイントが必要です。"
        }
        "core.server_endpoint_missing_with_id" => {
            "リモートプロトコルには MCP サーバーのエンドポイントが必要です（ID {id}）。"
        }
        "core.server_endpoint_invalid" => {
            "MCP サーバーのエンドポイント '{endpoint}' が無効です: {error}"
        }
        "core.server_endpoint_invalid_with_id" => {
            "MCP サーバーのエンドポイントが無効です（ID {id}、'{endpoint}'）：{error}"
        }
        "core.server_config_not_found" => "ID '{id}' の MCP サーバー設定が見つかりません。",
        "core.server_config_not_found_name" => "MCP サーバー「{name}」が見つかりません。",
        "core.create_dir_failed" => "ディレクトリ {path} の作成に失敗しました: {error}",
        "core.read_dir_failed" => "ディレクトリ {path} の読み取りに失敗しました: {error}",
        "core.read_config_failed" => "設定ファイル {path} の読み取りに失敗しました: {error}",
        "core.parse_json_failed" => "{path} の JSON サーバー設定を解析できませんでした: {error}",
        "core.parse_toml_failed" => "{path} の TOML サーバー設定を解析できませんでした: {error}",
        "core.serialise_toml_failed" => {
            "サーバー定義を TOML にシリアライズできませんでした: {error}"
        }
        "core.remove_file_failed" => "{path} の削除に失敗しました: {error}",
        "core.home_dir_unknown" => {
            "ユーザーのホームディレクトリ（MCP_CENTER_ROOT）を判別できません"
        }
        other => return Some(english_text(other)),
    })
}
