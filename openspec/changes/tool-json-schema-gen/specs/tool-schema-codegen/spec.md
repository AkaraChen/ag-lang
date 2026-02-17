## ADDED Requirements

### Requirement: Schema property emitted for @tool functions

The codegen SHALL emit a `.schema` property assignment statement immediately after the function declaration for every `@tool`-annotated `FnDecl`. The schema object SHALL contain `name` (the function name as a string), `description` (from the `ToolAnnotation`, or omitted if `None`), and `parameters` (a JSON Schema object describing the function parameters).

#### Scenario: @tool with description

- **WHEN** the input is `@tool("Search the web") fn search(query: str, limit: int = 5) -> str { query }`
- **THEN** output SHALL contain `function search(query, limit = 5) { return query; }` followed by `search.schema = { name: "search", description: "Search the web", parameters: { type: "object", properties: { query: { type: "string" }, limit: { type: "integer" } }, required: ["query"] } };`

#### Scenario: @tool without description

- **WHEN** the input is `@tool fn add(a: int, b: int) -> int { a + b }`
- **THEN** output SHALL contain `add.schema = { name: "add", parameters: { ... } }` with no `description` field

#### Scenario: Non-tool function has no schema

- **WHEN** the input is `fn helper(x: int) -> int { x + 1 }`
- **THEN** output SHALL NOT contain any `.schema` assignment

### Requirement: Type-to-JSON-Schema mapping for primitives

The codegen SHALL map AG primitive types to JSON Schema type strings: `str` → `"string"`, `num` → `"number"`, `int` → `"integer"`, `bool` → `"boolean"`.

#### Scenario: String parameter

- **WHEN** a `@tool` function has parameter `query: str`
- **THEN** the schema property for `query` SHALL be `{ "type": "string" }`

#### Scenario: Number parameter

- **WHEN** a `@tool` function has parameter `score: num`
- **THEN** the schema property for `score` SHALL be `{ "type": "number" }`

#### Scenario: Integer parameter

- **WHEN** a `@tool` function has parameter `count: int`
- **THEN** the schema property for `count` SHALL be `{ "type": "integer" }`

#### Scenario: Boolean parameter

- **WHEN** a `@tool` function has parameter `verbose: bool`
- **THEN** the schema property for `verbose` SHALL be `{ "type": "boolean" }`

### Requirement: Type-to-JSON-Schema mapping for arrays

The codegen SHALL map AG array types `[T]` to JSON Schema `{ "type": "array", "items": <T-schema> }`, recursively mapping the element type.

#### Scenario: Array of strings

- **WHEN** a `@tool` function has parameter `tags: [str]`
- **THEN** the schema property for `tags` SHALL be `{ "type": "array", "items": { "type": "string" } }`

#### Scenario: Nested array

- **WHEN** a `@tool` function has parameter `matrix: [[int]]`
- **THEN** the schema property for `matrix` SHALL be `{ "type": "array", "items": { "type": "array", "items": { "type": "integer" } } }`

### Requirement: Type-to-JSON-Schema mapping for objects and structs

The codegen SHALL map AG struct types `Struct(name, fields)` to JSON Schema `{ "type": "object", "properties": { ... }, "required": [...] }` where each field becomes a property with its type mapped recursively. AG map types `{str: V}` SHALL be mapped to `{ "type": "object", "additionalProperties": <V-schema> }`. Inline object types `{ field: Type }` SHALL be mapped the same as structs.

#### Scenario: Struct parameter

- **WHEN** `struct Config { timeout: int, verbose: bool }` is defined and a `@tool` function has parameter `config: Config`
- **THEN** the schema property for `config` SHALL be `{ "type": "object", "properties": { "timeout": { "type": "integer" }, "verbose": { "type": "boolean" } }, "required": ["timeout", "verbose"] }`

#### Scenario: Inline object type parameter

- **WHEN** a `@tool` function has parameter `dims: { w: num, h: num }`
- **THEN** the schema property for `dims` SHALL be `{ "type": "object", "properties": { "w": { "type": "number" }, "h": { "type": "number" } }, "required": ["w", "h"] }`

#### Scenario: Map type parameter

- **WHEN** a `@tool` function has parameter `headers: {str: str}`
- **THEN** the schema property for `headers` SHALL be `{ "type": "object", "additionalProperties": { "type": "string" } }`

### Requirement: Type-to-JSON-Schema mapping for nullable types

The codegen SHALL map AG nullable types `T?` by including the base type schema. Nullable fields SHALL be excluded from the `required` array of their parent object.

#### Scenario: Nullable primitive

- **WHEN** a `@tool` function has parameter `name: str?`
- **THEN** the schema property for `name` SHALL be `{ "type": "string" }` and `name` SHALL NOT appear in the `required` array

#### Scenario: Nullable field in struct

- **WHEN** a struct has `{ w: num?, h: num }` and is used in a `@tool` function
- **THEN** the schema SHALL have `"required": ["h"]` (excluding `w` since it's nullable)

### Requirement: Type-to-JSON-Schema mapping for union types

The codegen SHALL map AG union types `T | U` to JSON Schema `{ "anyOf": [<T-schema>, <U-schema>] }`.

#### Scenario: Union parameter

- **WHEN** a `@tool` function has parameter `id: str | int`
- **THEN** the schema property for `id` SHALL be `{ "anyOf": [{ "type": "string" }, { "type": "integer" }] }`

### Requirement: Required parameters array

The `parameters` JSON Schema object SHALL include a `required` array listing names of parameters that do not have default values. Parameters with defaults SHALL be excluded from `required`.

#### Scenario: Mix of required and optional parameters

- **WHEN** a `@tool` function has `fn search(query: str, limit: int = 10, verbose: bool = false)`
- **THEN** the schema SHALL have `"required": ["query"]`

#### Scenario: All parameters required

- **WHEN** a `@tool` function has `fn add(a: int, b: int)`
- **THEN** the schema SHALL have `"required": ["a", "b"]`

### Requirement: Codegen accepts tool registry

The codegen entry point SHALL accept an optional tool registry (`HashMap<String, ToolInfo>`) in addition to the AST module. When provided, the registry SHALL be used to resolve parameter types for schema generation. When not provided (empty map), `@tool` functions SHALL still emit schemas using AST type information where possible.

#### Scenario: Codegen with tool registry

- **WHEN** codegen is called with a module containing `@tool fn foo(x: str)` and a tool registry containing `foo`'s resolved types
- **THEN** the schema SHALL be generated using the resolved types from the registry

### Requirement: Any and unknown types map to empty schema

The codegen SHALL map `any` and `unknown` types to an empty JSON Schema object `{}`, which accepts any value.

#### Scenario: Any parameter

- **WHEN** a `@tool` function has parameter `data: any`
- **THEN** the schema property for `data` SHALL be `{}`
