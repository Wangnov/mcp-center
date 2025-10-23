# hooks 目录速览

当前仅包含 `use-keyboard-shortcuts.ts`：

- `useKeyboardShortcuts(shortcuts)`：注册/销毁 `keydown` 事件，根据 `ctrl/meta/alt/shift/key` 匹配执行回调。
- `getModifierSymbol()`：根据系统平台返回 `⌘` 或 `Ctrl`，用于提示快捷键。

使用建议：

- 将 Hook 放在组件顶部，并确保 `shortcuts` 数组引用稳定（必要时用 `useMemo`）。
- 快捷键涉及焦点管理时需手动聚焦目标元素，如 `ServersPage` 中的搜索框逻辑。
