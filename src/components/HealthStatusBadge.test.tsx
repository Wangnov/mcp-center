import { describe, expect, it, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import { HealthStatusBadge } from "./HealthStatusBadge";
import { renderWithQueryClient } from "@/test/test-utils";
import { getHealth } from "@/lib/api";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>(
    "@/lib/api",
  );
  return {
    ...actual,
    getHealth: vi.fn(),
  };
});

describe("HealthStatusBadge", () => {
  beforeEach(() => {
    vi.mocked(getHealth).mockReset();
  });

  it("renders loading indicator while pending", () => {
    vi.mocked(getHealth).mockImplementation(
      () => new Promise(() => undefined),
    );

    renderWithQueryClient(<HealthStatusBadge />);

    expect(document.querySelector(".animate-pulse")).not.toBeNull();
  });

  it("shows healthy state when API returns ok", async () => {
    vi.mocked(getHealth).mockResolvedValueOnce({ status: "ok" });

    renderWithQueryClient(<HealthStatusBadge />);

    const indicator = await screen.findByRole("button", {
      name: "server_status: 正常",
    });
    expect(indicator).toBeInTheDocument();
  });

  it("shows warning state when server reports degraded", async () => {
    vi.mocked(getHealth).mockResolvedValueOnce({ status: "degraded" });

    renderWithQueryClient(<HealthStatusBadge />);

    const indicator = await screen.findByRole("button", {
      name: "server_status: 警告",
    });
    expect(indicator).toBeInTheDocument();
  });
});
