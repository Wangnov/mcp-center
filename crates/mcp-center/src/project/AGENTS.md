# Project 模块速览（AI 助手用）

## 职责

- 基于磁盘路径生成稳定的 `ProjectId`（Blake3 哈希，裁剪至 16 hex）。
- 维护项目权限/自定义数据的持久化记录（`ProjectRecord`），序列化为 TOML 存储在 `Layout::projects_dir()`。
- 提供 `ProjectRegistry` 封装读写逻辑（ensure/list/load/store/delete），并在错误时抛出 `CoreError` 变体。
- 支持工具级别权限（`ToolPermission`）与自定义描述（`ToolCustomization`），供 Web API 与 daemon 权限管控使用。

## 关键类型

- `ProjectId`：`String` newtype，`from_path` 根据平台字节序生成哈希；常以 `Arc<RwLock<ProjectId>>` 形式共享（见 daemon/control.rs → HostService）。
- `ProjectRecord`：序列化结构体，字段补充 `#[serde(default)]`，以便兼容旧记录；`touch()` 更新 `last_seen_at`。
- `ToolPermission`：enum（All/AllowList/DenyList），在 host 权限判断时优先于 `allowed_server_ids`。
- `ProjectRegistry`：封装 `.toml` 文件 CRUD；注意 `list()` 会遍历目录并过滤扩展名，`load()`/`store()` 会抛出带路径信息的 `CoreError`，调用方要处理。

## 与其他模块的关系

- `daemon/control.rs`：使用 `ProjectRegistry` 维护桥接会话记录，并在 Roots 获取后迁移/更新记录。
- `daemon/host.rs`：读取 `ProjectRegistry`，结合 `ToolPermission` / `allowed_server_ids` 做工具过滤，自定义描述输出给客户端。
- `web/http.rs`：CRUD 接口直接操作 `ProjectRegistry` 与 `ProjectRecord`；Specta 导出类型与这里字段保持一致。
- `paths.rs::Layout`：提供项目目录路径与基础 ensure。

## 使用注意

- 新增字段时确保 `#[serde(default)]` 或自定义默认，避免旧记录解析失败。
- 修改 `ProjectId` 生成算法会破坏已有记录，慎动。
- `ProjectRegistry` 在内存中缓存 `.toml` 记录，并通过文件指纹（mtime + size）检测外部变动；若底层文件系统不暴露这些元数据，会退化为每次强制重建缓存。
- `ProjectRegistry::find_by_path` 依赖缓存构建的路径索引，调用前务必保证传入路径已经过规范化/一致化处理。
