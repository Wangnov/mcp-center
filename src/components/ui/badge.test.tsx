import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Badge } from "./badge";

describe("Badge", () => {
  it("renders span by default", () => {
    render(<Badge>Default</Badge>);

    const element = screen.getByText("Default");
    expect(element.tagName).toBe("SPAN");
    expect(element.getAttribute("data-slot")).toBe("badge");
  });

  it("renders child element when asChild is true", () => {
    render(
      <Badge asChild>
        <a href="#" data-testid="badge-link">
          Link
        </a>
      </Badge>,
    );

    const element = screen.getByTestId("badge-link");
    expect(element.tagName).toBe("A");
    expect(element.getAttribute("data-slot")).toBe("badge");
  });
});
