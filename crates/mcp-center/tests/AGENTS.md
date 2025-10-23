# Tests 目录速览（AI 助手用）

## 目标

这组集成测试验证 CLI、HTTP API、RPC 等高层功能是否按预期协同。所有测试均使用临时目录 / 进程，隔离真实用户数据。

## 文件概览

### `init_cli.rs`

- 测试 `mcp-center init`：
  - 在临时 root 下运行，检查 `config/servers` 目录生成并包含示例配置。
  - 验证示例 `ServerDefinition` 已分配 id、名称非空、默认禁用。
  - 再次运行 `init` 应保持幂等，输出包含 “Added sample config” 或 “already initialized”。
- 使用 `Command::cargo_bin("mcp-center")` 直接调用 CLI。

### `mcp_cli.rs`

- 覆盖 MCP 子命令主要流程：
  1. `mcp add` 添加 HTTP 服务器，确认配置写入、字段匹配、默认禁用。
  2. `mcp list` 输出检查。
  3. `mcp info` 返回 JSON，校验 id/name。
  4. `mcp enable/disable` 切换状态，并读回配置确认。
  5. 启动守护进程：spawn `mcp-center serve`（后台），等待 RPC socket，就绪后调用 `mcp list-tools`。
  6. `mcp remove --yes` 删除配置，确认文件不存在。
- 辅助函数：
  - `cli_with_root(&Path, args)` 统一附加 `--root`。
  - `load_server_configs(&Path)` 使用 `Layout` 读取配置。
- 注意：测试中会 spawn daemon 子进程；使用完毕后 `kill` + `wait`。

### `project_cli.rs`

- 重点验证项目权限 CLI：
  - 先创建 dummy server (`mcp add`)，再 `project add` 工作目录。
  - `project allow` → 检查 `allowed_server_ids` 更新。
  - `project allow-tools`/`deny-tools` → 检查 `ToolPermission` 切换 AllowList/DenyList。
  - `project set-tool-desc` / `reset-tool-desc` → 验证 `tool_customizations` 增删。
  - `project deny` → 移除服务器授权。
  - `project remove --yes` → 删除记录。
- 使用 `ProjectRegistry` 直接读取磁盘记录，确保 CLI 与核心逻辑一致。

### `http_api.rs`

- 端到端测试 HTTP Router（Axum）：
  - `make_router(layout)` 启动 `ServerManager` + `ProjectRegistry` + `HttpState`，返回 `Router`。
  - `write_server_config` 写入初始服务器定义。
  - `/api/project/allow` → `/api/project` → `/api/project/deny` 流程；检查 JSON 字段。
  - `/api/mcp/:id/enabled`、`/api/mcp/:id`、`/api/mcp/:id/tools`、`POST /api/mcp`、`DELETE /api/mcp/:id` 等 REST 行为。
  - `http_api_requires_auth_token` 确认缺少 Bearer token 返回 401。
- 使用 `tower::ServiceExt::oneshot` 直接调 `Router`，无需网络。

## 通用模式

- 所有测试使用 `tempdir()`/`Layout::new(tmp.path())`，避免污染真实 `~/.mcp-center`。
- 依赖 `assert_cmd` 检查 CLI 返回值、stdout/stderr。
- JSON 解析使用 `serde_json::Value`，断言对接口契约敏感（字段名 `snake_case`）。
- 若测试启动 daemon，记得清理子进程并等待结束，防止悬挂。

## 扩展建议

- 添加新 CLI 或 HTTP 功能时，复制现有模式：使用临时目录、现成 `cli_with_root` / `make_router`。
- 对 Specta 生成类型的断言应在单独脚本中处理（当前测试未覆盖）。
- 若未来支持 Windows/非 Unix，需要为依赖 Unix socket 的测试添加条件编译或替代实现。
- 更复杂的端到端场景可直接复用 `mcp-center-test-client::TestClient`，或调用 `cargo run -p mcp-center-test-client` 手动驱动工具调用。
