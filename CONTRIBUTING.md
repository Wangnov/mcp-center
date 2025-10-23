# Contributing to MCP Center | 贡献指南

[中文](#中文) | [English](#english)

---

## 中文

感谢您对 MCP Center 项目的关注！本文档提供了贡献指南。

### 行为准则

- 保持尊重和包容
- 遵循"Linus 原则"：少说多做，用代码说话
- 保持小步快跑的迭代方式
- 确保代码始终可编译、可测试

### 快速开始

1. **Fork 并克隆**

   ```bash
   git clone https://github.com/yourusername/mcp-center.git
   cd mcp-center
   ```

2. **安装依赖**

   ```bash
   npm install
   cargo build
   ```

3. **运行测试**

   ```bash
   # 后端测试
   cargo test-all

   # 前端类型检查
   npm run type-check
   ```

### 开发流程

#### 后端 (Rust)

1. **代码规范**

   ```bash
   cargo fmt              # 格式化代码
   cargo lint-fix         # 修复 lint 问题
   cargo check-all        # 检查所有目标
   cargo test-all         # 运行所有测试
   ```

2. **开发指南**
   - 使用 `CoreError` 和 `ApiError` 处理错误
   - 共享逻辑放在 `Layout`、`ServerManager`、`ProjectRegistry`
   - 文件系统操作使用 `paths::Layout`
   - 所有字符串必须支持国际化（更新 `cli/i18n.rs`）

#### 前端 (TypeScript/React)

1. **代码规范**

   ```bash
   npm run lint:fix       # 修复 ESLint 问题
   npm run type-check     # TypeScript 类型检查
   npm run format         # Prettier 格式化
   ```

2. **开发指南**
   - 使用函数组件 + Hooks
   - 状态管理：Zustand（全局）+ React Query（服务端数据）
   - Tailwind v4：使用 CSS 变量，不使用 `@apply`
   - shadcn/ui 组件与官方保持同步

3. **类型生成**

   ```bash
   npm run export-types   # 从 Rust 生成 TypeScript 类型
   ```

   - 修改后端 API 结构后必须运行
   - 禁止手动编辑 `api-types.generated.ts`

### 提交信息格式

我们遵循 Conventional Commits 规范：

```text
<类型>(<范围>): <主题>

<正文>

<页脚>
```

**类型说明：**

- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档修改
- `style`: 代码格式（不影响功能）
- `refactor`: 重构
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 工具链、依赖、构建配置

**示例：**

```text
feat(backend): Add tool-level permission control
- Implement ToolPermission enum (AllowList/DenyList)
- Add tool filtering in HostService
- Support custom tool descriptions

Summary: Enables fine-grained control over which tools
projects can access from MCP servers.
```

```text
fix(frontend): Resolve emoji rendering in health status
- Replace emoji with Lucide Circle icons
- Add proper accessibility attributes
- Support theme-based coloring

修复 #123
```

### Pull Request 流程

1. **创建功能分支**

   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **进行修改**
   - 保持提交小而专注
   - 编写清晰的提交信息
   - 同步更新文档

3. **测试修改**

   ```bash
   # 后端
   cargo fmt && cargo lint-fix && cargo test-all

   # 前端
   npm run export-types && npm run lint:fix && npm run type-check
   ```

4. **提交 Pull Request**
   - 提供清晰的修改说明
   - 引用相关 issue
   - 确保 CI 检查通过

### 项目结构

```text
mcp-center/
├── crates/
│   ├── mcp-center/           # 后端：CLI、守护进程、HTTP API
│   └── mcp-center-test-client/  # MCP 测试客户端
├── src/                      # 前端：React + Tailwind v4
├── src-tauri/                # 桌面应用壳
└── docs/                     # 设计文档
```

### 核心技术栈

- **后端**: Rust, Tokio, Axum, rmcp SDK, Specta
- **前端**: React 19, Vite 6, Tailwind CSS v4, shadcn/ui
- **桌面**: Tauri 2.0
- **状态**: Zustand, React Query
- **国际化**: i18next（前端）, 自定义（后端）

### 测试

- 为新功能编写测试
- 提交 PR 前确保所有测试通过
- 后端：集成测试位于 `crates/mcp-center/tests/`
- 前端：TypeScript 类型安全

### 文档

- 用户可见的变更需更新 README.md
- AI 协助相关的变更需更新 CLAUDE.md/AGENTS.md
- 为复杂逻辑添加代码注释
- 保持 CHANGELOG.md 更新

### 需要帮助？

- 阅读 `README.md`、`CLAUDE.md` 中的项目文档
- 查看已有的 issues 和 PRs
- 在 GitHub Discussions 中提问

---

## English

Thank you for your interest in contributing to MCP Center! This document provides guidelines for contributing to the project.

### Code of Conduct

- Be respectful and inclusive
- Follow the "Linus principle": Talk is cheap, show me the code
- Keep iterations small and focused
- Ensure code is always compilable and testable

### Getting Started

1. **Fork and Clone**

   ```bash
   git clone https://github.com/yourusername/mcp-center.git
   cd mcp-center
   ```

2. **Install Dependencies**

   ```bash
   npm install
   cargo build
   ```

3. **Run Tests**

   ```bash
   # Backend tests
   cargo test-all

   # Frontend type check
   npm run type-check
   ```

### Development Workflow

#### Backend (Rust)

1. **Code Style**

   ```bash
   cargo fmt              # Format code
   cargo lint-fix         # Fix linting issues
   cargo check-all        # Check all targets
   cargo test-all         # Run all tests
   ```

2. **Guidelines**
   - Use `CoreError` and `ApiError` for error handling
   - Shared logic goes in `Layout`, `ServerManager`, `ProjectRegistry`
   - File system operations should use `paths::Layout`
   - All strings must support i18n (update `cli/i18n.rs`)

#### Frontend (TypeScript/React)

1. **Code Style**

   ```bash
   npm run lint:fix       # Fix ESLint issues
   npm run type-check     # TypeScript type checking
   npm run format         # Prettier formatting
   ```

2. **Guidelines**
   - Use function components with Hooks
   - State management: Zustand for global state, React Query for server data
   - Tailwind v4: Use CSS variables, no `@apply`
   - Keep shadcn/ui components in sync with official versions

3. **Type Generation**

   ```bash
   npm run export-types   # Generate TypeScript types from Rust
   ```

   - Run after changing backend API structures
   - Never manually edit `api-types.generated.ts`

### Commit Message Format

We follow Conventional Commits specification:

```text
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Tooling, dependencies, build config

**Examples:**

```text
feat(backend): Add tool-level permission control
- Implement ToolPermission enum (AllowList/DenyList)
- Add tool filtering in HostService
- Support custom tool descriptions

Summary: Enables fine-grained control over which tools
projects can access from MCP servers.
```

```text
fix(frontend): Resolve emoji rendering in health status
- Replace emoji with Lucide Circle icons
- Add proper accessibility attributes
- Support theme-based coloring

Fixes #123
```

### Pull Request Process

1. **Create a Feature Branch**

   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Make Your Changes**
   - Keep commits small and focused
   - Write descriptive commit messages
   - Update documentation as needed

3. **Test Your Changes**

   ```bash
   # Backend
   cargo fmt && cargo lint-fix && cargo test-all

   # Frontend
   npm run export-types && npm run lint:fix && npm run type-check
   ```

4. **Submit Pull Request**
   - Provide a clear description of changes
   - Reference related issues
   - Ensure CI checks pass

### Project Structure

```text
mcp-center/
├── crates/
│   ├── mcp-center/           # Backend: CLI, daemon, HTTP API
│   └── mcp-center-test-client/  # MCP test client
├── src/                      # Frontend: React + Tailwind v4
├── src-tauri/                # Desktop app shell
└── docs/                     # Design docs
```

### Key Technologies

- **Backend**: Rust, Tokio, Axum, rmcp SDK, Specta
- **Frontend**: React 19, Vite 6, Tailwind CSS v4, shadcn/ui
- **Desktop**: Tauri 2.0
- **State**: Zustand, React Query
- **i18n**: i18next (frontend), custom (backend)

### Testing

- Write tests for new features
- Ensure all tests pass before submitting PR
- Backend: Integration tests in `crates/mcp-center/tests/`
- Frontend: Type safety with TypeScript

### Documentation

- Update README.md for user-facing changes
- Update CLAUDE.md/AGENTS.md for AI assistance
- Add code comments for complex logic
- Keep CHANGELOG.md up to date

### Need Help?

- Read project documentation in `README.md`, `CLAUDE.md`
- Check existing issues and PRs
- Ask questions in GitHub Discussions

---

## License | 许可证

By contributing to MCP Center, you agree that your contributions will be licensed under the same license as the project.

贡献到 MCP Center 即表示您同意您的贡献将使用与项目相同的许可证。
