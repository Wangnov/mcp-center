import { describe, expect, it } from "vitest";
import { parseToolInput } from "./Projects";

describe("parseToolInput", () => {
  it("splits by newline and comma and trims whitespace", () => {
    const input = "Server::ToolA,Server::ToolB\n  Other::ToolC  ";
    const result = parseToolInput(input);

    expect(result).toEqual(["Server::ToolA", "Server::ToolB", "Other::ToolC"]);
  });

  it("filters out empty tokens", () => {
    const input = "Server::ToolA\n\n,   ,";
    const result = parseToolInput(input);

    expect(result).toEqual(["Server::ToolA"]);
  });
});
