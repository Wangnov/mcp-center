import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Button } from "./button";

describe("Button", () => {
  it("renders native button by default", () => {
    render(<Button>Click</Button>);

    const element = screen.getByRole("button");
    expect(element.tagName).toBe("BUTTON");
    expect(element.getAttribute("data-slot")).toBe("button");
  });

  it("renders child element when asChild is true", () => {
    render(
      <Button asChild>
        <span role="button" data-testid="button-span">
          Acting Button
        </span>
      </Button>,
    );

    const element = screen.getByTestId("button-span");
    expect(element.tagName).toBe("SPAN");
    expect(element.getAttribute("data-slot")).toBe("button");
  });
});
