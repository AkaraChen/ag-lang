## ADDED Requirements

### Requirement: Implements DslHandler trait

The `ag-dsl-prompt` crate SHALL export a struct `PromptDslHandler` that implements `DslHandler`. When registered with kind `"prompt"`, it SHALL handle all `DslBlock` nodes with `kind == "prompt"`.

#### Scenario: Handler registration

- **WHEN** `PromptDslHandler` is registered with `translator.register_dsl_handler("prompt", Box::new(PromptDslHandler))`
- **THEN** all `@prompt` blocks in the AST are processed by this handler

### Requirement: Inline block codegen produces PromptTemplate constructor

For `DslContent::Inline`, the handler SHALL produce a JavaScript `const <name> = new PromptTemplate({...});` declaration. The PromptTemplate config object SHALL contain `messages`, and optionally `model`, `examples`, `outputSchema`, `constraints`, and `messagesPlaceholder`.

#### Scenario: Simple prompt

- **WHEN** input is `@prompt greeting ``` @role system\nHello! ```\n`
- **THEN** output is `const greeting = new PromptTemplate({ messages: [{ role: "system", content: "Hello!" }] });`

### Requirement: Role sections compile to messages array

Each `PromptSection::Role` SHALL compile to an object in the `messages` array with `role` and `content` fields. If the role body contains only `Text` parts, `content` SHALL be a string literal. If the body contains `Capture` parts, `content` SHALL be an arrow function `(ctx) => \`...\${ctx.varname}...\`` using a template literal.

#### Scenario: Static role content

- **WHEN** a role section has body `[Text("You are a helpful assistant.")]`
- **THEN** output message is `{ role: "system", content: "You are a helpful assistant." }`

#### Scenario: Dynamic role content with captures

- **WHEN** a role section has body `[Text("You are "), Capture(role_expr), Text(", expert in "), Capture(domain_expr)]`
- **THEN** output message is `{ role: "system", content: (ctx) => \`You are \${ctx.role}, expert in \${ctx.domain}\` }`

### Requirement: Capture variable naming in content functions

When a capture expression is a simple identifier (e.g., `#{role}`), the codegen SHALL use that identifier name as the property on `ctx` (e.g., `ctx.role`). When a capture is a complex expression (e.g., `#{a + b}`), the codegen SHALL generate a synthetic name `ctx.__capture_<index>` and document the mapping.

#### Scenario: Simple identifier capture

- **WHEN** capture is `Ident("role")`
- **THEN** codegen emits `ctx.role`

#### Scenario: Complex expression capture

- **WHEN** capture is `Binary(Add, Ident("a"), Ident("b"))`
- **THEN** codegen emits `ctx.__capture_0` (or similar synthetic name)

### Requirement: Model spec compilation

`ModelSpec` SHALL compile to a `model` property on the PromptTemplate config: an array of string literals in fallback order.

#### Scenario: Model with fallback

- **WHEN** `ModelSpec { models: ["claude-sonnet", "gpt-4o"] }`
- **THEN** output includes `model: ["claude-sonnet", "gpt-4o"]`

#### Scenario: No model specified

- **WHEN** `PromptTemplate.model` is `None`
- **THEN** the `model` property is omitted from the config object

### Requirement: Examples compilation

`PromptSection::Examples` SHALL compile to an `examples` property: an array of `{ role: string, content: string }` objects in declaration order.

#### Scenario: Two example pairs

- **WHEN** examples contain `[{user: "hello", assistant: "hi"}, {user: "bye", assistant: "goodbye"}]`
- **THEN** output includes `examples: [{ role: "user", content: "hello" }, { role: "assistant", content: "hi" }, { role: "user", content: "bye" }, { role: "assistant", content: "goodbye" }]`

### Requirement: Messages placeholder compilation

`PromptSection::Messages` SHALL compile to a `messagesPlaceholder` property on the config. The value SHALL be the capture variable name (for simple identifiers) or the translated expression.

#### Scenario: Messages with identifier capture

- **WHEN** `@messages #{history}` with capture being `Ident("history")`
- **THEN** output includes `messagesPlaceholder: "history"`

### Requirement: Output schema compilation — inline

When `@output` has an inline schema (`OutputKind::Inline`), the codegen SHALL produce a `outputSchema` property containing a JSON Schema-like object with `type: "object"` and `properties`.

#### Scenario: Inline output schema

- **WHEN** `@output { answer: str, confidence: num, sources: [str] }`
- **THEN** output includes `outputSchema: { type: "object", properties: { answer: { type: "string" }, confidence: { type: "number" }, sources: { type: "array", items: { type: "string" } } }, required: ["answer", "confidence", "sources"] }`

### Requirement: Output schema compilation — capture reference

When `@output` has a capture reference (`OutputKind::CaptureRef`), the codegen SHALL use `CodegenContext::translate_expr` to produce a JavaScript expression representing the schema. The exact translation depends on how the host compiler represents type-to-schema conversion.

#### Scenario: Output with capture ref

- **WHEN** `@output #{ResponseSchema}` with capture referencing a struct type
- **THEN** output includes `outputSchema: ctx.translate_expr(capture)` result as a JS expression

### Requirement: Constraints compilation

`Constraints` SHALL compile to a `constraints` property: an object literal with key-value pairs. Number values SHALL be JS numbers, string values SHALL be JS strings, array values SHALL be JS arrays, boolean values SHALL be JS booleans.

#### Scenario: Constraints object

- **WHEN** `@constraints { temperature: 0.7, max_tokens: 4096, stop: ["\n\n"] }`
- **THEN** output includes `constraints: { temperature: 0.7, max_tokens: 4096, stop: ["\n\n"] }`

### Requirement: FileRef codegen

For `DslContent::FileRef`, the handler SHALL produce a JavaScript statement that reads the file content. The output SHALL be `const <name> = new PromptTemplate({ messages: [{ role: "system", content: await readFile("<path>", "utf-8") }] });` or equivalent. The path SHALL be preserved as-is from the source.

#### Scenario: File reference prompt

- **WHEN** `@prompt system from "./system-prompt.txt"`
- **THEN** output produces JavaScript that creates a PromptTemplate loading content from the file

### Requirement: Runtime import generation

When the handler produces `PromptTemplate` constructor calls, it SHALL also emit an import statement: `import { PromptTemplate } from "@agentscript/prompt-runtime";`. The import SHALL be emitted once per module, not once per prompt block.

#### Scenario: Single import for multiple prompts

- **WHEN** a module contains two `@prompt` blocks
- **THEN** output contains exactly one `import { PromptTemplate } from "@agentscript/prompt-runtime";` at the top

### Requirement: Standalone testability

The `ag-dsl-prompt` crate SHALL be testable without the host compiler. Tests SHALL be able to construct `DslBlock` with `DslPart::Text` and mock `DslPart::Capture` values, run the full pipeline (lex → parse → validate → codegen), and verify the JavaScript output string.

#### Scenario: Independent unit test

- **WHEN** a test constructs `DslBlock { kind: "prompt", name: "test", content: Inline { parts: [Text("@role system\nHello!")] } }` and calls the handler with a mock `CodegenContext`
- **THEN** the handler returns valid `ModuleItem` nodes without depending on any host compiler crate
