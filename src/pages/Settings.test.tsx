import type { ReactNode } from "react";
import { describe, expect, it, beforeEach, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import { screen } from "@testing-library/react";
import { renderWithQueryClient } from "@/test/test-utils";
import { SettingsPage } from "./Settings";
import { getAppVersion } from "@/lib/api";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>(
    "@/lib/api",
  );
  return {
    ...actual,
    getAppVersion: vi.fn(),
  };
});

vi.mock("@/components/ui/dropdown-menu", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    DropdownMenu: Wrapper,
    DropdownMenuTrigger: Wrapper,
    DropdownMenuContent: Wrapper,
    DropdownMenuItem: ({ children, onSelect }: any) => (
      <button type="button" onClick={onSelect}>
        {children}
      </button>
    ),
  };
});

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, ...rest }: any) => <button {...rest}>{children}</button>,
}));

vi.mock("lucide-react", () => ({
  Globe: () => null,
}));

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.mocked(getAppVersion).mockResolvedValue("1.0.0");
  });

  it("renders version and handles language selection", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<SettingsPage />);

    await screen.findByText("version_label: 1.0.0");

    const trigger = screen.getAllByRole("button", { name: /English/ })[0];
    await user.click(trigger);
    await user.click(screen.getAllByRole("button", { name: "简体中文" })[0]);

    expect(screen.getByRole("button", { name: /简体中文/ })).toBeInTheDocument();
  });

  it("shows loading state when fetching version", () => {
    vi.mocked(getAppVersion).mockReturnValue(new Promise(() => undefined));

    renderWithQueryClient(<SettingsPage />);

    expect(screen.getByText("version_label: loading")).toBeInTheDocument();
  });

  it("falls back to unknown version when API returns empty", async () => {
    vi.mocked(getAppVersion).mockResolvedValueOnce("");

    renderWithQueryClient(<SettingsPage />);

    await screen.findByText("version_label: unknown_version");
  });
});
