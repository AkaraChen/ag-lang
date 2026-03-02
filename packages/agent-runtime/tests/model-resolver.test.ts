import { describe, it, expect } from "vitest";
import { resolveModel } from "../src/model-resolver.js";

describe("resolveModel", () => {
  it("resolves short name 'claude-sonnet'", () => {
    const model = resolveModel("claude-sonnet");
    expect(model).toBeDefined();
    expect(model.modelId).toContain("claude");
  });

  it("resolves short name 'gpt-4o'", () => {
    const model = resolveModel("gpt-4o");
    expect(model).toBeDefined();
    expect(model.modelId).toBe("gpt-4o");
  });

  it("resolves provider/model format 'anthropic/claude-3-haiku-20240307'", () => {
    const model = resolveModel("anthropic/claude-3-haiku-20240307");
    expect(model).toBeDefined();
    expect(model.modelId).toContain("claude-3-haiku");
  });

  it("resolves provider/model format 'openai/gpt-4-turbo'", () => {
    const model = resolveModel("openai/gpt-4-turbo");
    expect(model).toBeDefined();
    expect(model.modelId).toBe("gpt-4-turbo");
  });

  it("resolves raw claude model IDs", () => {
    const model = resolveModel("claude-3-opus-20240229");
    expect(model).toBeDefined();
  });

  it("throws for unknown provider", () => {
    expect(() => resolveModel("unknown/model")).toThrow("Unknown model provider");
  });

  it("throws for completely unknown model", () => {
    expect(() => resolveModel("llama-70b")).toThrow("Cannot resolve model");
  });
});
