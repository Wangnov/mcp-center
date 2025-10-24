import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { ToolDetailDialog } from "./ToolDetailDialog";

const tool = {
  name: "resolve-library-id",
  serverName: "Alpha",
  serverId: "srv-1",
  description: "Resolve library metadata",
};

describe("ToolDetailDialog", () => {
  it("returns null when no tool is provided", () => {
    const { container } = render(
      <ToolDetailDialog tool={null} open onOpenChange={() => {}} />,
    );

    expect(container.firstChild).toBeNull();
  });

  it("renders tool information when open", () => {
    render(<ToolDetailDialog tool={tool} open onOpenChange={() => {}} />);

    expect(
      screen.getByRole("heading", { name: /resolve-library-id/i }),
    ).toBeInTheDocument();
    expect(screen.getByText("工具描述")).toBeInTheDocument();
    expect(screen.getByText("Resolve library metadata")).toBeInTheDocument();
  });

  it("renders fallback when description is missing", () => {
    render(
      <ToolDetailDialog
        tool={{ ...tool, description: null }}
        open
        onOpenChange={() => {}}
      />,
    );

    expect(screen.getByText("暂无描述")).toBeInTheDocument();
  });
});
