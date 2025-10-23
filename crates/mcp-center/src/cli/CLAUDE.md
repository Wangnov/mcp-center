# CLI 模块速览（AI 助手用）

## 角色

- 提供 i18n 文本访问器（`Messages`）与语言自动侦测，供 CLI 命令行输出使用。
- 仅在 Rust 端维护语言字符串，最终在 `bin/mcp-center.rs` 调用 `cli_i18n`。

## 关键内容

- `Language` 枚举：`English` / `SimplifiedChinese` / `TraditionalChinese` / `Japanese`。
- `detect_language()`：
  - 环境变量 `MCP_CENTER_LANG` 优先；
  - 其次使用 `locale_config::Locale::user_default()` 扫描系统 locale。
- `Messages`：
  - 缓存于 `OnceLock`。
  - 提供大量便捷方法（如 `workspace_initialized`, `registered_server`）包装翻译模板并执行 `interpolate`。
  - `text(key)` 根据当前语言调度至 `english_text/zh_hans_text/...`。
- 翻译表：
  - `english_text`, `zh_hans_text`, `zh_hant_text`, `japanese_text`（位于文件后半部分），返回 `Option<&'static str>`。
  - 若 key 不存在，默认回退英文/占位提示。
- 错误转换：
  - `translate_core_error(&CoreError)`/`translate_anyhow(&AnyhowError)` 将核心错误映射至本地化消息。
- Clap 集成：
  - `apply_command_translations(Command)` / `apply_arg_translations(Arg)` 用翻译的 `about`/`help` 更新 CLI 定义。

## 依赖/使用

- `bin/mcp-center.rs`：
  - 构建 `Cli` 时调用 `i18n::messages()` 获取文本。
  - 在输出/错误提示处，集中调用 `messages()` 方法。
- 错误处理：
  - CLI 执行过程中捕获 `CoreError`，可通过 `i18n::translate_anyhow` 生成用户友好的多语言提示。

## 注意点

- 新增 CLI 命令或 `CoreError` 变体时，务必同步在四种语言表中添加 key，否则 `text()` 返回默认占位并在日志中打印警告。
- `Messages` 使用 `OnceLock`；测试若需要不同语言需在每个测试进程前设置环境变量（无法在同一进程内重设）。
