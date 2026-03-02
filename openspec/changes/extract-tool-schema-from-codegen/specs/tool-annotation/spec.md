## MODIFIED Requirements

### Requirement: Checker registers @tool fn in tool registry

The checker SHALL register each `@tool`-annotated `FnDecl` in a tool registry within the `CheckResult`. The registry SHALL store the function name mapped to a `ToolSchemaInfo` (defined in `ag-ast`). When registering, the checker SHALL convert each parameter's resolved `Type` to a `JsonSchema` value. The `ToolSchemaInfo` SHALL include the tool description from the annotation and the parameter name/schema pairs.

#### Scenario: Tool function registered with schema info

- **WHEN** the checker processes `@tool("Search docs") fn search(query: str, limit: int) -> str { ... }`
- **THEN** `search` SHALL be registered in the tool registry as `ToolSchemaInfo { description: Some("Search docs"), params: [("query", JsonSchema::String), ("limit", JsonSchema::Integer)] }`

#### Scenario: Non-tool function not registered

- **WHEN** the checker processes `fn helper(x: int) -> int { x + 1 }`
- **THEN** `helper` SHALL NOT appear in the tool registry

#### Scenario: Struct parameter converted to JsonSchema::Object

- **WHEN** the checker processes `@tool fn run(config: Config) -> str { ... }` where `Config { timeout: int, verbose: bool }`
- **THEN** `config` SHALL be registered as `JsonSchema::Object { properties: [("timeout", Integer), ("verbose", Boolean)], required: ["timeout", "verbose"], additional_properties: None }`

#### Scenario: Nullable parameter converted to inner schema

- **WHEN** the checker processes `@tool fn fetch(url: str, timeout: int?) -> str { ... }`
- **THEN** `timeout` SHALL be registered as `JsonSchema::Integer` (the nullable wrapper is stripped; optionality is handled at the tool schema level by excluding from `required`)

#### Scenario: Array parameter converted to JsonSchema::Array

- **WHEN** the checker processes `@tool fn batch(items: [str]) -> [str] { ... }`
- **THEN** `items` SHALL be registered as `JsonSchema::Array(Box::new(JsonSchema::String))`

#### Scenario: Union parameter converted to JsonSchema::AnyOf

- **WHEN** the checker processes `@tool fn handle(input: str | int) -> str { ... }`
- **THEN** `input` SHALL be registered as `JsonSchema::AnyOf(vec![JsonSchema::String, JsonSchema::Integer])`

#### Scenario: Non-serializable type converted to JsonSchema::Any

- **WHEN** the checker processes `@tool fn apply(f: fn(int) -> int) -> int { ... }`
- **THEN** `f` SHALL be registered as `JsonSchema::Any` (function types have no JSON Schema representation)
