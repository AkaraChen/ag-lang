## ADDED Requirements

### Requirement: Parser recognizes @tool annotation syntax

The parser SHALL recognize the `@tool` annotation appearing before a `fn` declaration. The annotation SHALL accept an optional string literal as the tool description. The parsed annotation SHALL be stored as a `ToolAnnotation` attached to the subsequent `FnDecl`.

#### Scenario: @tool with no arguments

- **WHEN** the parser encounters `@tool fn read_file(path: str) -> str { ... }`
- **THEN** it SHALL produce a `FnDecl` with a `ToolAnnotation { description: None }` and `name = "read_file"`

#### Scenario: @tool with description string

- **WHEN** the parser encounters `@tool("Read a file from disk") fn read_file(path: str) -> str { ... }`
- **THEN** it SHALL produce a `FnDecl` with a `ToolAnnotation { description: Some("Read a file from disk") }`

#### Scenario: @tool without parens is bare annotation

- **WHEN** the parser encounters `@tool fn foo() { ... }` (no parentheses after `@tool`)
- **THEN** it SHALL produce a `FnDecl` with `ToolAnnotation { description: None }` (no error for missing parens)

### Requirement: @tool annotation AST representation

The `ToolAnnotation` AST node SHALL contain: `description: Option<String>` (the tool description, `None` if no argument provided) and `span: Span`.

#### Scenario: ToolAnnotation fields for bare @tool

- **WHEN** `@tool` is parsed (no parentheses)
- **THEN** the `ToolAnnotation` SHALL have `description = None` and a valid `span` covering the `@tool` token

#### Scenario: ToolAnnotation fields for @tool with description

- **WHEN** `@tool("Search the web")` is parsed
- **THEN** the `ToolAnnotation` SHALL have `description = Some("Search the web")` and a valid `span` covering from `@` through `)`

### Requirement: FnDecl includes tool_annotation field

The `FnDecl` AST struct SHALL include a `tool_annotation: Option<ToolAnnotation>` field. When no `@tool` annotation is present, this field SHALL be `None`.

#### Scenario: FnDecl without @tool

- **WHEN** the parser encounters `fn add(a: int, b: int) -> int { a + b }`
- **THEN** the resulting `FnDecl` SHALL have `tool_annotation: None`

#### Scenario: FnDecl with @tool

- **WHEN** the parser encounters `@tool("Add two numbers") fn add(a: int, b: int) -> int { a + b }`
- **THEN** the resulting `FnDecl` SHALL have `tool_annotation: Some(ToolAnnotation { description: Some("Add two numbers"), ... })`

### Requirement: @tool only valid before fn declarations

The parser SHALL produce a diagnostic error if `@tool` appears before a non-fn declaration (e.g., before `struct`, `extern`, `let`, `enum`, or `type`).

#### Scenario: @tool before struct is error

- **WHEN** the parser encounters `@tool struct Foo { x: int }`
- **THEN** it SHALL produce a diagnostic error: `@tool annotation can only be applied to fn declarations`

#### Scenario: @tool before extern is error

- **WHEN** the parser encounters `@tool extern fn fetch(url: str) -> str`
- **THEN** it SHALL produce a diagnostic error: `@tool annotation can only be applied to fn declarations`

#### Scenario: @tool before let is error

- **WHEN** the parser encounters `@tool let x = 5`
- **THEN** it SHALL produce a diagnostic error: `@tool annotation can only be applied to fn declarations`

#### Scenario: @tool before enum is error

- **WHEN** the parser encounters `@tool enum Color { Red, Green, Blue }`
- **THEN** it SHALL produce a diagnostic error: `@tool annotation can only be applied to fn declarations`

### Requirement: @tool with pub modifier works in both orderings

The parser SHALL accept both `@tool pub fn ...` and `pub @tool fn ...` orderings. Both SHALL produce a `FnDecl` with `is_pub: true` and a `ToolAnnotation` attached.

#### Scenario: @tool before pub fn

- **WHEN** the parser encounters `@tool pub fn search(query: str) -> str { ... }`
- **THEN** it SHALL produce a `FnDecl` with `is_pub: true` and `tool_annotation: Some(ToolAnnotation { description: None })`

#### Scenario: pub before @tool fn

- **WHEN** the parser encounters `pub @tool fn search(query: str) -> str { ... }`
- **THEN** it SHALL produce a `FnDecl` with `is_pub: true` and `tool_annotation: Some(ToolAnnotation { description: None })`

#### Scenario: @tool("desc") pub async fn

- **WHEN** the parser encounters `@tool("Search") pub async fn search(query: str) -> str { ... }`
- **THEN** it SHALL produce a `FnDecl` with `is_pub: true`, `is_async: true`, and `tool_annotation: Some(ToolAnnotation { description: Some("Search") })`

### Requirement: Checker registers @tool fn in tool registry

The checker SHALL register each `@tool`-annotated `FnDecl` in a tool registry within the symbol table. The registry SHALL store the function name and its `ToolAnnotation` metadata, making tool functions discoverable for future codegen passes.

#### Scenario: Tool function registered

- **WHEN** the checker processes `@tool fn read_file(path: str) -> str { ... }`
- **THEN** `read_file` SHALL be registered in the tool registry with its annotation metadata

#### Scenario: Non-tool function not registered

- **WHEN** the checker processes `fn helper(x: int) -> int { x + 1 }`
- **THEN** `helper` SHALL NOT appear in the tool registry

### Requirement: Checker warns on non-serializable param types

The checker SHALL emit a warning (not a hard error) if a `@tool`-annotated function has parameters whose types cannot be mapped to JSON Schema. Serializable types are: `str`, `num`, `int`, `bool`, `[T]` where T is serializable, `{K: V}` where K is `str` and V is serializable, and struct types whose fields are all serializable.

#### Scenario: All params serializable - no warning

- **WHEN** the checker processes `@tool fn search(query: str, limit: int) -> str { ... }`
- **THEN** it SHALL NOT emit any serializability warning

#### Scenario: Function type param - warning

- **WHEN** the checker processes `@tool fn apply(callback: fn(int) -> int) -> int { ... }`
- **THEN** it SHALL emit a warning that `callback` has type `fn(int) -> int` which is not serializable for tool calling

#### Scenario: Array of serializable type - no warning

- **WHEN** the checker processes `@tool fn batch(items: [str]) -> [str] { ... }`
- **THEN** it SHALL NOT emit any serializability warning (arrays of serializable types are serializable)

#### Scenario: Nested struct with serializable fields - no warning

- **WHEN** `struct Config { timeout: int, verbose: bool }` is defined and the checker processes `@tool fn run(config: Config) -> str { ... }`
- **THEN** it SHALL NOT emit any serializability warning (Config fields are all serializable)
