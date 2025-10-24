import { describe, expect, it } from "vitest";
import {
  addServerFormSchema,
  buildAddServerPayload,
  type AddServerFormValues,
} from "./AddServerDialog";

const baseValues: AddServerFormValues = {
  name: "Demo",
  protocol: "stdio",
  command: "node server.js",
  endpoint: "",
};

describe("addServerFormSchema", () => {
  it("requires command for stdio protocol", () => {
    const result = addServerFormSchema.safeParse({
      ...baseValues,
      command: "",
    });

    expect(result.success).toBe(false);
  });

  it("requires endpoint for http/sse protocols", () => {
    const httpResult = addServerFormSchema.safeParse({
      ...baseValues,
      protocol: "http",
      command: undefined,
      endpoint: "",
    });
    expect(httpResult.success).toBe(false);

    const valid = addServerFormSchema.safeParse({
      ...baseValues,
      protocol: "http",
      command: undefined,
      endpoint: "https://example.com",
    });
    expect(valid.success).toBe(true);
  });
});

describe("buildAddServerPayload", () => {
  it("splits command and args for stdio protocol", () => {
    const payload = buildAddServerPayload(baseValues);

    expect(payload.command).toBe("node");
    expect(payload.args).toBe("server.js");
  });

  it("leaves endpoint untouched for http protocol", () => {
    const payload = buildAddServerPayload({
      ...baseValues,
      protocol: "http",
      command: undefined,
      endpoint: "https://api",
    });

    expect(payload.command).toBeUndefined();
    expect(payload.endpoint).toBe("https://api");
  });

  it("falls back to empty command when parsing whitespace", () => {
    const payload = buildAddServerPayload({
      ...baseValues,
      command: "   ",
    });

    expect(payload.command).toBe("");
    expect(payload.args).toBe("");
  });
});
