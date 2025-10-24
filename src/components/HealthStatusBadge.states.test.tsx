import { describe, expect, it, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";

const useQueryMock = vi.hoisted(() => vi.fn());

vi.mock("@tanstack/react-query", async () => {
  const actual = await vi.importActual<typeof import("@tanstack/react-query")>(
    "@tanstack/react-query",
  );
  return {
    ...actual,
    useQuery: useQueryMock,
  };
});

import { HealthStatusBadge } from "./HealthStatusBadge";

describe("HealthStatusBadge (mocked query states)", () => {
  beforeEach(() => {
    useQueryMock.mockReset();
  });

  it("renders error state when query errors", () => {
    useQueryMock.mockReturnValue({
      data: null,
      isError: true,
      isPending: false,
      isFetching: false,
      error: new Error("offline"),
    });

    render(<HealthStatusBadge />);

    const button = screen.getByRole("button", {
      name: "server_status: 离线",
    });
    expect(button).toBeInTheDocument();
  });

  it("dims indicator while fetching new data", () => {
    useQueryMock.mockReturnValue({
      data: { status: "ok" },
      isError: false,
      isPending: false,
      isFetching: true,
      error: null,
    });

    render(<HealthStatusBadge />);

    const button = screen.getByRole("button", {
      name: "server_status: 正常",
    });
    const indicator = button.querySelector("span");
    expect(indicator?.className).toContain("opacity-70");
  });
});
