import type { ReactNode } from "react";
import { vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import "whatwg-fetch";

// 统一模拟 react-i18next，默认返回 key 或 defaultValue
vi.mock("react-i18next", async () => {
  const actual = await vi.importActual<typeof import("react-i18next")>(
    "react-i18next",
  );

  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, options?: Record<string, unknown>) =>
        (options?.defaultValue as string | undefined) ?? key,
      i18n: {
        language: "en",
        languages: ["en"],
        changeLanguage: vi.fn(),
        exists: () => true,
      },
    }),
    Trans: ({ children }: { children: ReactNode }) => children,
  };
});

// 某些组件依赖该标记判断是否在 Tauri 环境
Object.defineProperty(window, "__TAURI_IPC__", {
  value: undefined,
  writable: true,
});
