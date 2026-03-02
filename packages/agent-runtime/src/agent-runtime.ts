import { generateText } from "ai";
import type { LanguageModelV1 } from "ai";
import { resolveModel } from "./model-resolver.js";
import { wrapTools } from "./tool-wrapper.js";
import type {
  AgentRuntimeConfig,
  GenerateOptions,
  GenerateResult,
  Message,
} from "./types.js";

export class AgentRuntime {
  private model: LanguageModelV1;
  private messages: Message[];
  private tools: ReturnType<typeof wrapTools>;
  private outputSchema?: Record<string, unknown>;
  private constraints?: Record<string, unknown>;
  private hooks?: Record<string, (...args: unknown[]) => unknown>;
  private maxSteps: number;

  constructor(config: AgentRuntimeConfig) {
    // Resolve model — use first from list, or default
    const modelName = config.model?.[0] ?? "claude-sonnet";
    this.model = resolveModel(modelName);

    this.messages = config.messages;
    this.tools = wrapTools(config.tools ?? []);
    this.outputSchema = config.outputSchema;
    this.constraints = config.constraints;
    this.hooks = config.hooks;
    this.maxSteps =
      (this.constraints?.["max_steps"] as number | undefined) ?? 10;

    // Fire init hook
    if (this.hooks?.["init"]) {
      this.hooks["init"]();
    }
  }

  async generate(options?: GenerateOptions): Promise<GenerateResult> {
    const messages = [
      ...this.messages,
      ...(options?.messages ?? []),
    ];

    // Fire pre-generate hook
    if (this.hooks?.["before_generate"]) {
      this.hooks["before_generate"](messages);
    }

    const result = await generateText({
      model: this.model,
      messages: messages.map((m) => ({
        role: m.role as "system" | "user" | "assistant",
        content: m.content,
      })),
      tools: this.tools,
      maxSteps: options?.maxSteps ?? this.maxSteps,
      ...(this.constraints?.["temperature"] != null && {
        temperature: this.constraints["temperature"] as number,
      }),
      ...(this.constraints?.["max_tokens"] != null && {
        maxTokens: this.constraints["max_tokens"] as number,
      }),
    });

    const generateResult: GenerateResult = {
      text: result.text,
      messages: result.response.messages.map((m) => ({
        role: m.role,
        content: typeof m.content === "string"
          ? m.content
          : JSON.stringify(m.content),
      })),
      steps: result.steps.length,
    };

    // Fire post-generate hook
    if (this.hooks?.["after_generate"]) {
      this.hooks["after_generate"](generateResult);
    }

    return generateResult;
  }
}
