import { anthropic } from "@ai-sdk/anthropic";
import { openai } from "@ai-sdk/openai";
import type { LanguageModelV1 } from "ai";

const MODEL_MAP: Record<string, () => LanguageModelV1> = {
  "claude-sonnet": () => anthropic("claude-sonnet-4-20250514"),
  "claude-haiku": () => anthropic("claude-haiku-4-20250514"),
  "claude-opus": () => anthropic("claude-opus-4-20250514"),
  "gpt-4o": () => openai("gpt-4o"),
  "gpt-4o-mini": () => openai("gpt-4o-mini"),
  "gpt-4.1": () => openai("gpt-4.1"),
  "gpt-4.1-mini": () => openai("gpt-4.1-mini"),
};

export function resolveModel(name: string): LanguageModelV1 {
  // Check short-name map
  const factory = MODEL_MAP[name];
  if (factory) return factory();

  // Try provider/model format: "anthropic/claude-3-sonnet"
  const slashIdx = name.indexOf("/");
  if (slashIdx > 0) {
    const provider = name.substring(0, slashIdx);
    const model = name.substring(slashIdx + 1);
    switch (provider) {
      case "anthropic":
        return anthropic(model);
      case "openai":
        return openai(model);
      default:
        throw new Error(`Unknown model provider: ${provider}`);
    }
  }

  // Try as raw Anthropic model ID
  if (name.startsWith("claude-")) {
    return anthropic(name);
  }

  // Try as raw OpenAI model ID
  if (name.startsWith("gpt-") || name.startsWith("o1") || name.startsWith("o3")) {
    return openai(name);
  }

  throw new Error(`Cannot resolve model: ${name}. Use "provider/model" format or a known short name.`);
}
