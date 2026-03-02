## ADDED Requirements

### Requirement: Tool schema emission for @tool functions

The codegen SHALL emit a `fnName.schema = { ... }` assignment statement immediately after each `@tool`-annotated function declaration. The schema object SHALL follow the JSON Schema structure with fields `name` (string), `description` (string, if provided), and `parameters` (object with `type: "object"`, `properties`, `required`, `additionalProperties: false`).

#### Scenario: Tool schema with description

- **WHEN** the input is `@tool("Look up docs") fn lookup(topic: str) -> str { ... }`
- **THEN** output contains `function lookup(topic) { ... }` followed by `lookup.schema = { name: "lookup", description: "Look up docs", parameters: { type: "object", properties: { topic: { type: "string" } }, required: ["topic"], additionalProperties: false } };`

#### Scenario: Tool schema without description

- **WHEN** the input is `@tool fn add(a: int, b: int) -> int { a + b }`
- **THEN** output contains `function add(a, b) { ... }` followed by `add.schema = { name: "add", parameters: { type: "object", properties: { a: { type: "integer" }, b: { type: "integer" } }, required: ["a", "b"], additionalProperties: false } };` and the schema object SHALL NOT contain a `description` field

#### Scenario: Tool schema with optional parameter

- **WHEN** the input is `@tool fn search(query: str, limit: int?) -> str { ... }`
- **THEN** the schema `required` array SHALL contain `"query"` but SHALL NOT contain `"limit"`

#### Scenario: Tool schema with struct parameter

- **WHEN** the input is `@tool fn run(config: Config) -> str { ... }` where `Config { timeout: int, verbose: bool }`
- **THEN** the schema `properties.config` SHALL be `{ type: "object", properties: { timeout: { type: "integer" }, verbose: { type: "boolean" } }, required: ["timeout", "verbose"] }`

### Requirement: Codegen does not depend on ag-checker

The `ag-codegen` crate SHALL NOT have a dependency on `ag-checker`. Tool schema information SHALL be received as `ToolSchemaInfo` (from `ag-ast`) which contains `JsonSchema` values, not checker `Type` values.

#### Scenario: ag-codegen compiles without ag-checker

- **WHEN** `ag-codegen` is compiled
- **THEN** it SHALL NOT have `ag-checker` in its dependency tree (verified by Cargo.toml)
