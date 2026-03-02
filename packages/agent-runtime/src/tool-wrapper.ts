import { tool } from "ai";
import { jsonSchema } from "ai";
import type { AgentTool } from "./types.js";

/**
 * Wrap an AgentScript tool function (with `.schema` property)
 * into an AI SDK `tool()` object.
 */
export function wrapTool(fn: AgentTool) {
  const schema = fn.schema;
  return tool({
    description: schema.description ?? schema.name,
    parameters: jsonSchema(schema.parameters),
    execute: async (...args: unknown[]) => {
      return fn(...args);
    },
  });
}

/**
 * Wrap an array of AgentScript tool functions into AI SDK tools map.
 */
export function wrapTools(
  fns: AgentTool[]
): Record<string, ReturnType<typeof tool>> {
  const tools: Record<string, ReturnType<typeof tool>> = {};
  for (const fn of fns) {
    tools[fn.schema.name] = wrapTool(fn);
  }
  return tools;
}
