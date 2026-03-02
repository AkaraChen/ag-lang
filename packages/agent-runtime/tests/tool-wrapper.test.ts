import { describe, it, expect } from "vitest";
import { wrapTool, wrapTools } from "../src/tool-wrapper.js";
import type { AgentTool } from "../src/types.js";

function makeTool(name: string, fn: (...args: unknown[]) => unknown): AgentTool {
  const tool = fn as AgentTool;
  tool.schema = {
    name,
    description: `Tool: ${name}`,
    parameters: {
      type: "object",
      properties: {
        input: { type: "string" },
      },
      required: ["input"],
      additionalProperties: false,
    },
  };
  return tool;
}

describe("wrapTool", () => {
  it("wraps a function with schema into an AI SDK tool", () => {
    const fn = makeTool("lookup", (input: unknown) => `Result: ${input}`);
    const wrapped = wrapTool(fn);
    expect(wrapped).toBeDefined();
    expect(wrapped.type).toBe("function");
  });
});

describe("wrapTools", () => {
  it("wraps multiple tools into a map", () => {
    const fn1 = makeTool("tool_a", () => "a");
    const fn2 = makeTool("tool_b", () => "b");
    const map = wrapTools([fn1, fn2]);
    expect(Object.keys(map)).toEqual(["tool_a", "tool_b"]);
  });

  it("returns empty map for empty array", () => {
    const map = wrapTools([]);
    expect(Object.keys(map)).toEqual([]);
  });
});
