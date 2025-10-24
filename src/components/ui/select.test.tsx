import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "./select";

describe("ui/select", () => {
  it("renders trigger with default size and value", () => {
    render(
      <Select defaultValue="one">
        <SelectTrigger>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="one">One</SelectItem>
        </SelectContent>
      </Select>,
    );

    const trigger = document.querySelector('[data-slot="select-trigger"]');
    expect(trigger).toHaveAttribute("data-size", "default");
  });

  it("supports custom trigger size", () => {
    render(
      <Select defaultValue="one">
        <SelectTrigger size="sm">
          <SelectValue />
        </SelectTrigger>
      </Select>,
    );

    const trigger = document.querySelector('[data-slot="select-trigger"]');
    expect(trigger).toHaveAttribute("data-size", "sm");
  });
});
