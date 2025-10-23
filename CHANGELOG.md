# Changelog

All notable changes to this project will be documented in this file.

## [0.0.1] - 2025-10-23

### Added

- 初版 MCP Center 后端（`crates/mcp-center`）：CLI、守护进程、HTTP API、桥接层、Specta 类型导出。
- 轻量 MCP 测试客户端（`crates/mcp-center-test-client`），支持 stdio / SSE / streaming HTTP。
- React + Tailwind v4 WebUI，集成 shadcn/ui、React Query、Zustand。
- Tauri 2.0 桌面壳（占位命令，准备接入守护进程）。
- AI 协助文档体系：各目录 `AGENTS.md`/`CLAUDE.md`，以及面向人类开发者的 `README.md`。

### Notes

- 当前版本仍处于早期探索阶段，功能以开发与验证为主，生产部署前请务必评估。
- 版本号在 Rust crates、Tauri、前端 `package.json` 均统一为 `0.0.1`，尚未发布 Git tag。
