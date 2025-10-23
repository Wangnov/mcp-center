# Tauri 桌面壳

Tauri 2.0 打包入口，用于将 WebUI 封装成跨平台桌面应用。当前实现仅提供基础命令，尚未接入后端核心逻辑。

## 目录结构

- `src/main.rs`：初始化日志、注册命令、启动 Tauri Builder
- `src/commands.rs`：对前端暴露的命令（目前大部分返回占位错误）
- `tauri.conf.json`：Tauri 配置
- `icons/`：应用图标
- `build.rs`：构建时生成元数据

## 开发调试

```bash
# 确保后端 HTTP 服务已启动
cargo run --bin mcp-center serve &

# 启动 Tauri（开发模式）
npm run tauri:dev

# 生产构建
npm run tauri:build
```

## TODO

- 在 `commands.rs` 中接入真实的守护进程调用（如启动/停止 `mcp-center serve`）。
- 根据需求添加窗口、托盘、更新、日志等桌面端特性。
- 与 WebUI 协同：`main.tsx` 会调用 `get_backend_base_url` / `get_backend_auth_token` 注入 HTTP 地址。

更多约定可参考 `AGENTS.md` / `CLAUDE.md`。
