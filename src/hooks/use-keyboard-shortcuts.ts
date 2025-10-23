import { useEffect } from "react";

interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  alt?: boolean;
  shift?: boolean;
  callback: (e: KeyboardEvent) => void;
}

export function useKeyboardShortcuts(shortcuts: KeyboardShortcut[]) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      for (const shortcut of shortcuts) {
        const isCtrlMatch =
          shortcut.ctrl === undefined || shortcut.ctrl === e.ctrlKey;
        const isMetaMatch =
          shortcut.meta === undefined || shortcut.meta === e.metaKey;
        const isAltMatch =
          shortcut.alt === undefined || shortcut.alt === e.altKey;
        const isShiftMatch =
          shortcut.shift === undefined || shortcut.shift === e.shiftKey;
        const isKeyMatch = shortcut.key.toLowerCase() === e.key.toLowerCase();

        if (
          isCtrlMatch &&
          isMetaMatch &&
          isAltMatch &&
          isShiftMatch &&
          isKeyMatch
        ) {
          e.preventDefault();
          shortcut.callback(e);
          break;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [shortcuts]);
}

export function getModifierSymbol(): string {
  return navigator.platform.toLowerCase().includes("mac") ? "⌘" : "Ctrl";
}
