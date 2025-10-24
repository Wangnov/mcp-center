import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { screen } from "@testing-library/react";
import { render } from "@testing-library/react";
import { Layout } from "./Layout";

vi.mock("@/components/icons/McpCenterLogo", () => ({
  McpCenterLogo: () => <div data-testid="logo" />,
}));

vi.mock("@/components/HealthStatusBadge", () => ({
  HealthStatusBadge: () => <div data-testid="health-badge" />,
}));

vi.mock("@/components/ui/tooltip", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    Tooltip: Wrapper,
    TooltipTrigger: Wrapper,
    TooltipContent: Wrapper,
    TooltipProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
  };
});

describe("Layout", () => {
  it("renders navigation and outlet content", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route element={<Layout />}>
            <Route path="settings" element={<div>Settings Page</div>} />
            <Route path="mcp" element={<div>MCP Page</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByTestId("logo")).toBeInTheDocument();
    expect(screen.getByTestId("health-badge")).toBeInTheDocument();
    expect(screen.getByText("Settings Page")).toBeInTheDocument();
  });
});
