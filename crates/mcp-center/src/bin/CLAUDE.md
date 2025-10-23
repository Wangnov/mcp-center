# Bin 目录速览（AI 助手用）

## 概览

此目录包含两个可执行入口：

- `mcp-center.rs`：主 CLI，可执行 `init / serve / connect / mcp / project` 子命令。
- `export-types.rs`：Specta TypeScript 类型导出脚本，用于同步前端契约。

## mcp-center.rs

- 基于 Clap (`Parser`, `Subcommand`) 定义命令树：
  - `Serve` → 调用 `daemon::serve::run`.
  - `Connect` → 调用 `bridge::connect::run`.
  - `Mcp` 子命令：
    - `add/list/list-tools/info/remove/enable/disable`：读写 `ServerConfig`，操作 `Layout`。
    - `list-tools/info` 依赖 `daemon::rpc` 通过 Unix Socket 查询正在运行的 daemon。
  - `Project` 子命令：
    - `add/remove/list/allow/deny/...` 直接调用 `ProjectRegistry`，变更权限或自定义描述。
  - `Init`：创建基础目录 + 示例配置。
- 输出文本全部通过 `cli_i18n::messages()` 获取，以保证多语言一致。
- 常见辅助函数：
  - `resolve_layout(cli.root?)`，内部调用 `default_root`.
  - `print_table` / `prompt_confirm` 等交互辅助。
  - 与 daemon RPC 的交互使用 tokio runtime + `UnixStream`。

## export-types.rs

- 定义 `EXPORT_TARGETS` 常量数组（标签 + 导出函数指针）。
- main 流程：
  1. 配置 `ExportConfiguration`（`BigIntExportBehavior::Number`）。
  2. 迭代 `EXPORT_TARGETS`，拼接 TS 类型字符串。
  3. 写入 `../../src/lib/api-types.generated.ts` 并打印生成的类型列表。
- 运行方式：
  - `npm run export-types`（package.json 调用该二进制）。

## 注意事项

- CLI 中所有路径相关操作都使用 `Layout`，避免散落的 `PathBuf` 拼接。若新增命令，优先复用现有工具。
- RPC 交互假设 daemon 已运行，否则命令会提示启动；需要的话可自动调用 `connect` 逻辑或给出友好错误。
- `export-types.rs` 与 Specta 导出的类型必须与 `web/http.rs` 定义同步；新增 API 时记得在 `EXPORT_TARGETS` 中注册。
