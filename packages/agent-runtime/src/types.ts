export interface Message {
  role: string;
  content: string;
}

export interface ToolSchema {
  name: string;
  description?: string;
  parameters: {
    type: "object";
    properties: Record<string, unknown>;
    required: string[];
    additionalProperties: boolean;
  };
}

export interface AgentTool {
  (...args: unknown[]): unknown;
  schema: ToolSchema;
}

export interface AgentRuntimeConfig {
  model?: string[];
  messages: Message[];
  examples?: Message[];
  tools?: AgentTool[];
  skills?: AgentTool[];
  agents?: AgentRuntime[];
  outputSchema?: Record<string, unknown>;
  constraints?: Record<string, unknown>;
  hooks?: Record<string, (...args: unknown[]) => unknown>;
}

export interface GenerateOptions {
  messages?: Message[];
  maxSteps?: number;
}

export interface GenerateResult {
  text: string;
  messages: Message[];
  steps: number;
}

export interface StreamOptions extends GenerateOptions {
  onChunk?: (chunk: string) => void;
}

// Forward declare to avoid circular dependency
export interface AgentRuntime {
  generate(options?: GenerateOptions): Promise<GenerateResult>;
}
