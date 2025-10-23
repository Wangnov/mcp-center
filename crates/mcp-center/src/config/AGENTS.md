# Config 模块速览（AI 助手用）

## 职责

- `ServerDefinition` / `ServerConfig`：描述 MCP 服务器并负责 TOML/JSON 读写。
- `ServerProtocol`：与 CLI/Daemon/Web 共享的协议枚举（支持 Specta）。
- `id_generator::generate_id`：生成 8 字符小写 ID，避免与现有配置冲突。

## 关键接口

- `ServerDefinition::validate()`：校验名称、命令、协议合法性；远程协议需提供有效 `endpoint` URL。
- `ServerConfig::{from_file,new,to_toml_string,assign_unique_id}`：文件加载、创建、持久化。

## 与其他模块

- `paths::Layout` 通过这些类型实现 `load_server_config`、`remove_server_config` 等操作。
- `daemon::server_manager` 在启动时读取 `ServerDefinition` 列表并据此拉起 MCP 服务。
- CLI/Web 直接复用同一模型输出到用户或 Specta。

## 注意

- 新增字段请加 `#[serde(default)]`，以兼容旧配置。
- 远程协议参数更新时务必同步 CLI 参数提示与文档。
