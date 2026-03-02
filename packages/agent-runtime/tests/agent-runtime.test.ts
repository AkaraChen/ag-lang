import { describe, it, expect } from "vitest";
import { AgentRuntime } from "../src/agent-runtime.js";

describe("AgentRuntime", () => {
  it("constructs with minimal config", () => {
    const agent = new AgentRuntime({
      messages: [
        { role: "system", content: "You are helpful." },
      ],
    });
    expect(agent).toBeDefined();
    expect(agent).toBeInstanceOf(AgentRuntime);
  });

  it("constructs with model array", () => {
    const agent = new AgentRuntime({
      model: ["claude-sonnet", "gpt-4o"],
      messages: [
        { role: "system", content: "You are helpful." },
      ],
    });
    expect(agent).toBeDefined();
  });

  it("constructs with constraints", () => {
    const agent = new AgentRuntime({
      messages: [
        { role: "system", content: "Be brief." },
      ],
      constraints: {
        temperature: 0.5,
        max_tokens: 100,
      },
    });
    expect(agent).toBeDefined();
  });

  it("fires init hook on construction", () => {
    let initCalled = false;
    const agent = new AgentRuntime({
      messages: [
        { role: "system", content: "Hi" },
      ],
      hooks: {
        init: () => { initCalled = true; },
      },
    });
    expect(agent).toBeDefined();
    expect(initCalled).toBe(true);
  });

  it("constructs with tools", () => {
    const lookup = Object.assign(
      (topic: string) => `Docs for: ${topic}`,
      {
        schema: {
          name: "lookup",
          description: "Look up docs",
          parameters: {
            type: "object" as const,
            properties: { topic: { type: "string" } },
            required: ["topic"],
            additionalProperties: false,
          },
        },
      }
    );

    const agent = new AgentRuntime({
      messages: [
        { role: "system", content: "Use tools." },
      ],
      tools: [lookup],
    });
    expect(agent).toBeDefined();
  });
});
