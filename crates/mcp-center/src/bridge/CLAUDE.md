# Bridge 模块速览（AI 助手用）

## 职责

- `mcp-center connect` 子命令实现，负责探测项目路径、与 daemon 控制 socket 握手、建立 STDIO ↔ daemon 管道。
- 定义控制协议消息体，供 bridge 与 daemon 共享。

## 子文件

- `connect.rs`：
  - CLI 参数 `ConnectArgs { root, daemon }`。
  - 主要流程：
    1. `resolve_layout`（可传 root 或 `default_root()`）。
    2. `detect_project_path()`：优先环境变量 → 标记文件 → Git 根 → CWD（全部 `tokio::fs::canonicalize`，大量 debug 日志）。
    3. `connect_or_launch`：尝试连接 control socket，不存在则自动 spawn daemon (`spawn_daemon`) 并等待 60 秒重试。
    4. `perform_handshake`：发送 `ControlMessage::BridgeHello`（含项目路径/agent/PID/metadata），等待 `BridgeReady`，打印 info。
    5. `tunnel_stdio`：使用 `tokio::io::copy` / `split` 建立 STDIO ↔ 本地 socket 双向管道，支持 Ctrl+C 中断。
  - `gather_metadata()` 生成 JSON（pid/cwd/exe）。
  - 默认跨平台执行；Windows 也会尝试自动拉起 daemon。
- `control.rs`：控制协议结构体
  - `BridgeHello`, `BridgeReady`, `ControlMessage`（与 daemon/control.rs 复用）。

## 与其他模块关系

- `daemon/control.rs` 读取 `BridgeHello`，写入 `ProjectRegistry` 并返回 `BridgeReady`；两端必须保持结构一致。
- `ServeArgs` 在 `bin/mcp-center.rs` 中暴露 `connect` 子命令，调用 `bridge::connect::run`.
- 项目路径探测逻辑与 daemon 侧创建记录联动：`BridgeHello` 初始路径用于生成临时 `ProjectId`，随后 daemon 尝试 roots 更正。

## 注意事项

- `detect_project_path` 会遍历环境变量和祖先目录，增加了大量 debug 日志；如需减少噪音可调节 `tracing` level。
- `spawn_daemon` 使用 `setsid` 与临时日志文件，保证 daemon 独立运行；如果单二进制部署需确保 `args.daemon` 正确指向当前可执行文件。
- 通过 `interprocess` 实现跨平台连接，Windows 不再提前退出；仅 Unix 分支执行 `setsid` 做终端脱离。
