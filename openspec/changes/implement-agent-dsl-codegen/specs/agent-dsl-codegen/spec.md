## ADDED Requirements

### Requirement: Agent DSL codegen module

The `ag-dsl-agent` crate SHALL have a `codegen.rs` module that generates SWC AST nodes from an `AgentTemplate`. The `generate()` function SHALL accept an `AgentTemplate`, a slice of type-erased capture references, and a `CodegenContext`, and return `Vec<swc::ModuleItem>`.

#### Scenario: Minimal agent with system role

- **WHEN** an `AgentTemplate` has name "Coder", one `PromptSection::Role { role: System, body: [Text("You are helpful.")] }`, and no other fields set
- **THEN** the generated JavaScript SHALL contain:
  - `import { AgentRuntime } from "@agentscript/runtime"`
  - `const Coder = new AgentRuntime({ messages: [{ role: "system", content: "You are helpful." }] })`

#### Scenario: Agent with model

- **WHEN** an `AgentTemplate` has `model: Some(ModelSpec { models: ["claude-sonnet", "gpt-4o"] })`
- **THEN** the output SHALL include `model: ["claude-sonnet", "gpt-4o"]` in the config object

#### Scenario: Agent with no model

- **WHEN** an `AgentTemplate` has `model: None`
- **THEN** the `model` property SHALL be omitted from the config object

### Requirement: Tools capture translation

The codegen SHALL translate `tools_capture` to a `tools` property on the config object by calling `ctx.translate_expr()` on the captured expression at the given index.

#### Scenario: Agent with tools capture

- **WHEN** an `AgentTemplate` has `tools_capture: Some(0)` and captures slice has an expression at index 0
- **THEN** the output SHALL include `tools: <translated_capture_0>` in the config object

#### Scenario: Agent with no tools

- **WHEN** an `AgentTemplate` has `tools_capture: None`
- **THEN** the `tools` property SHALL be omitted from the config object

### Requirement: Skills capture translation

The codegen SHALL translate `skills_capture` to a `skills` property on the config object by calling `ctx.translate_expr()` on the captured expression.

#### Scenario: Agent with skills capture

- **WHEN** an `AgentTemplate` has `skills_capture: Some(1)` and captures slice has an expression at index 1
- **THEN** the output SHALL include `skills: <translated_capture_1>` in the config object

### Requirement: Agents capture translation

The codegen SHALL translate `agents_capture` to an `agents` property on the config object by calling `ctx.translate_expr()` on the captured expression.

#### Scenario: Agent with agents capture

- **WHEN** an `AgentTemplate` has `agents_capture: Some(2)` and captures slice has an expression at index 2
- **THEN** the output SHALL include `agents: <translated_capture_2>` in the config object

### Requirement: Output schema compilation

When `AgentTemplate.output` is `Some(OutputSpec)`, the codegen SHALL produce an `outputSchema` property on the config object. The schema SHALL be a JSON Schema object with `type: "object"`, `properties`, and `required` fields. Type mapping SHALL match the prompt codegen behavior.

#### Scenario: Agent with inline output schema

- **WHEN** an `AgentTemplate` has `output: Some(OutputSpec { kind: Inline, fields: [("answer", "str"), ("confidence", "num")] })`
- **THEN** the output SHALL include `outputSchema: { type: "object", properties: { answer: { type: "string" }, confidence: { type: "number" } }, required: ["answer", "confidence"] }`

#### Scenario: Agent with no output

- **WHEN** an `AgentTemplate` has `output: None`
- **THEN** the `outputSchema` property SHALL be omitted from the config object

### Requirement: Constraints compilation

When `AgentTemplate.constraints` is `Some(Constraints)`, the codegen SHALL produce a `constraints` property on the config object with key-value pairs matching the constraint fields.

#### Scenario: Agent with constraints

- **WHEN** an `AgentTemplate` has `constraints: Some(Constraints { fields: [("temperature", Number(0.3)), ("max_tokens", Number(4096))] })`
- **THEN** the output SHALL include `constraints: { temperature: 0.3, max_tokens: 4096 }`

### Requirement: Lifecycle hooks compilation

The codegen SHALL translate `on_hooks` to a `hooks` property on the config object. Each `OnHook { event, capture_index }` SHALL produce a key-value pair where the key is the event name and the value is the translated capture expression.

#### Scenario: Agent with init and error hooks

- **WHEN** an `AgentTemplate` has `on_hooks: [OnHook { event: "init", capture_index: 3 }, OnHook { event: "error", capture_index: 4 }]`
- **THEN** the output SHALL include `hooks: { init: <translated_capture_3>, error: <translated_capture_4> }`

#### Scenario: Agent with no hooks

- **WHEN** an `AgentTemplate` has `on_hooks: []`
- **THEN** the `hooks` property SHALL be omitted from the config object

### Requirement: Messages with captures use template literals

When a role section body contains `PromptPart::Capture` entries, the codegen SHALL produce a `content` arrow function `(ctx) => \`...\${ctx.var}...\`` matching the prompt codegen behavior.

#### Scenario: Dynamic system prompt

- **WHEN** an agent has role system with body `[Text("Expert in "), Capture(0)]` and capture 0 translates to identifier `lang`
- **THEN** the output message SHALL be `{ role: "system", content: (ctx) => \`Expert in \${ctx.lang}\` }`

### Requirement: Examples compilation

When the agent template includes `PromptSection::Examples`, the codegen SHALL produce an `examples` property on the config object matching the prompt codegen behavior.

#### Scenario: Agent with examples

- **WHEN** an `AgentTemplate` has a `PromptSection::Examples` section with user/assistant pairs
- **THEN** the output SHALL include `examples: [{ role: "user", content: "..." }, { role: "assistant", content: "..." }]`

### Requirement: Import deduplication

The codegen SHALL emit exactly one import statement: `import { AgentRuntime } from "@agentscript/runtime"`. This SHALL NOT be duplicated regardless of config complexity.

#### Scenario: Single import

- **WHEN** an `AgentTemplate` with all fields populated is compiled
- **THEN** the output SHALL contain exactly one `import { AgentRuntime }` statement

### Requirement: Agent DSL handler

The `ag-dsl-agent` crate SHALL have a `handler.rs` module with an `AgentDslHandler` struct implementing `ag_dsl_core::DslHandler`. It SHALL follow the 5-step pipeline: lex → parse → validate → collect captures → codegen.

#### Scenario: Inline block handling

- **WHEN** `handle()` receives a `DslBlock` with `DslContent::Inline` containing agent directives
- **THEN** it SHALL return `Ok(Vec<ModuleItem>)` with the generated AgentRuntime JavaScript

#### Scenario: FileRef block rejection

- **WHEN** `handle()` receives a `DslBlock` with `DslContent::FileRef`
- **THEN** it SHALL return `Err(DslError)` with a message indicating file references are not supported for agent blocks

#### Scenario: Parse error propagation

- **WHEN** the agent parser returns diagnostics (e.g., empty template)
- **THEN** the handler SHALL return `Err(DslError)` with the diagnostic messages joined

### Requirement: Full agent compilation

The codegen SHALL correctly handle a complete agent definition with all directive types producing a valid `AgentRuntime` constructor call.

#### Scenario: Full agent block

- **WHEN** an AG module contains:
  ```
  @agent Coder ```
    @model claude-sonnet | gpt-4o
    @tools #{[read_file, write_file]}
    @role system
    You are an expert software engineer.
    @constraints { temperature: 0.3 }
    @on init #{initHandler}
  ```
- **THEN** the codegen pipeline SHALL produce JavaScript containing:
  - `import { AgentRuntime } from "@agentscript/runtime"`
  - `const Coder = new AgentRuntime({...})` with model, tools, messages, constraints, and hooks properties
