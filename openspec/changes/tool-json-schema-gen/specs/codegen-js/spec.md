## MODIFIED Requirements

### Requirement: Function declaration translation

The codegen SHALL translate `fn name(params) -> type { body }` to `function name(params) { body }`. Type annotations on parameters and return type SHALL be erased. The last expression in a function body (implicit return) SHALL be wrapped in a `return` statement. `pub fn` SHALL emit `export function`. `async fn` SHALL emit `async function`. When the function has a `@tool` annotation, the codegen SHALL additionally emit a `.schema` property assignment statement immediately after the function declaration.

#### Scenario: Simple function

- **WHEN** the input is `fn add(a: int, b: int) -> int { a + b }`
- **THEN** output is `function add(a, b) { return a + b; }`

#### Scenario: Pub function

- **WHEN** the input is `pub fn greet(name: str) -> str { "Hello" }`
- **THEN** output is `export function greet(name) { return "Hello"; }`

#### Scenario: Function with explicit ret

- **WHEN** the input is `fn foo(x: int) -> int { if x > 0 { ret x }; 0 }`
- **THEN** output contains `if (x > 0) { return x; }` and ends with `return 0;`

#### Scenario: Default parameters

- **WHEN** the input is `fn greet(name: str, loud: bool = false) -> str { name }`
- **THEN** output is `function greet(name, loud = false) { return name; }`

#### Scenario: @tool function emits schema

- **WHEN** the input is `@tool("Add numbers") fn add(a: int, b: int) -> int { a + b }`
- **THEN** output contains `function add(a, b) { return a + b; }` followed by `add.schema = { name: "add", description: "Add numbers", parameters: { type: "object", properties: { a: { type: "integer" }, b: { type: "integer" } }, required: ["a", "b"] } };`
