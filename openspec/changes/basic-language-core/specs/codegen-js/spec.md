## ADDED Requirements

### Requirement: Variable declaration translation

The codegen SHALL translate `let` to `const`, `mut` to `let`, and `const` to `const` in JavaScript output. Type annotations SHALL be erased (not emitted).

#### Scenario: Let binding

- **WHEN** the input is `let x = 42`
- **THEN** output is `const x = 42;`

#### Scenario: Mut binding

- **WHEN** the input is `mut counter = 0`
- **THEN** output is `let counter = 0;`

#### Scenario: Const binding

- **WHEN** the input is `const MAX = 100`
- **THEN** output is `const MAX = 100;`

### Requirement: Function declaration translation

The codegen SHALL translate `fn name(params) -> type { body }` to `function name(params) { body }`. Type annotations on parameters and return type SHALL be erased. The last expression in a function body (implicit return) SHALL be wrapped in a `return` statement. `pub fn` SHALL emit `export function`. `async fn` SHALL emit `async function`.

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

### Requirement: Arrow function translation

The codegen SHALL translate arrow functions to JavaScript arrow functions. Type annotations SHALL be erased.

#### Scenario: Expression arrow

- **WHEN** the input is `let double = (x: int) => x * 2`
- **THEN** output is `const double = (x)=>x * 2;`

#### Scenario: Block body arrow

- **WHEN** the input is `let f = (x: int) => { let y = x + 1; y }`
- **THEN** output is `const f = (x)=>{ const y = x + 1; return y; };`

### Requirement: Struct declaration translation

The codegen SHALL NOT emit runtime code for struct declarations. Structs are type-only and SHALL be erased during code generation.

#### Scenario: Struct is erased

- **WHEN** the input is `struct User { name: str, age: int }`
- **THEN** no output is emitted for this declaration

### Requirement: Enum declaration and construction translation

The codegen SHALL translate enum variant construction to tagged union objects. Each variant SHALL have a `tag` field with the variant name as a string. Associated data fields SHALL be spread into the object.

#### Scenario: Unit variant

- **WHEN** the input is `let s = Status::Pending`
- **THEN** output is `const s = { tag: "Pending" };`

#### Scenario: Variant with data

- **WHEN** the input is `let s = Status::Active(since: "2025-01-01")`
- **THEN** output is `const s = { tag: "Active", since: "2025-01-01" };`

#### Scenario: Enum declaration is erased

- **WHEN** the input is `enum Status { Pending, Active(since: str) }`
- **THEN** no runtime code is emitted for the enum declaration itself

### Requirement: Type alias translation

The codegen SHALL NOT emit runtime code for type alias declarations. They are type-only.

#### Scenario: Type alias is erased

- **WHEN** the input is `type ID = str`
- **THEN** no output is emitted

### Requirement: Match expression translation

The codegen SHALL translate `match` expressions to if-else chains. Each pattern SHALL be translated to a condition check. Pattern bindings SHALL be translated to variable declarations within the arm body. Guard expressions (`if` clause) SHALL be AND-ed with the pattern condition.

#### Scenario: Literal pattern match

- **WHEN** the input is `match x { 0 => "zero", 1 => "one", _ => "other" }`
- **THEN** output is equivalent to `x === 0 ? "zero" : x === 1 ? "one" : "other"` or an if-else chain

#### Scenario: Enum pattern match

- **WHEN** the input is `match s { Status::Active(since) => since, _ => "" }`
- **THEN** output checks `s.tag === "Active"` and binds `const since = s.since;`

#### Scenario: Match with guard

- **WHEN** the input is `match n { x if x > 100 => "big", _ => "small" }`
- **THEN** output is equivalent to `if (n > 100) { return "big"; } else { return "small"; }`

### Requirement: Pipe operator translation

The codegen SHALL translate `a |> f` to `f(a)`. When the right side is a call expression with `_` placeholder, `a |> f(_, x)` SHALL translate to `f(a, x)`, replacing `_` with the piped value.

#### Scenario: Simple pipe

- **WHEN** the input is `data |> parse |> validate`
- **THEN** output is `validate(parse(data))`

#### Scenario: Pipe with placeholder

- **WHEN** the input is `data |> transform(_, options)`
- **THEN** output is `transform(data, options)`

### Requirement: Optional chaining and nullish coalescing translation

The codegen SHALL translate `?.` to JavaScript optional chaining (`?.`) and `??` to JavaScript nullish coalescing (`??`), as these are natively supported in modern JS.

#### Scenario: Optional chaining

- **WHEN** the input is `user?.name`
- **THEN** output is `user?.name`

#### Scenario: Nullish coalescing

- **WHEN** the input is `x ?? "default"`
- **THEN** output is `x ?? "default"`

### Requirement: Error propagation operator translation

The codegen SHALL translate the `?` error propagation operator to an early return pattern. `expr?` SHALL be translated to a temporary variable check: if the result is an `Error`, return it immediately; otherwise unwrap the value.

#### Scenario: Error propagation

- **WHEN** the input is `let x = parse(input)?`
- **THEN** output is equivalent to `const _tmp = parse(input); if (_tmp instanceof Error) return _tmp; const x = _tmp;`

### Requirement: Try-catch translation

The codegen SHALL translate `try { ... } catch e { ... }` to JavaScript `try { ... } catch (e) { ... }`.

#### Scenario: Try-catch

- **WHEN** the input is `try { parse(input) } catch e { log(e) }`
- **THEN** output is `try { parse(input); } catch (e) { log(e); }`

### Requirement: Control flow translation

The codegen SHALL translate `if`/`else` to JavaScript `if`/`else`, `for x in iter` to `for (const x of iter)`, and `while cond` to `while (cond)`.

#### Scenario: If-else

- **WHEN** the input is `if x > 0 { "positive" } else { "non-positive" }`
- **THEN** output is `if (x > 0) { return "positive"; } else { return "non-positive"; }` (or ternary when used as expression)

#### Scenario: For-in loop

- **WHEN** the input is `for item in items { process(item) }`
- **THEN** output is `for (const item of items) { process(item); }`

#### Scenario: While loop

- **WHEN** the input is `while x > 0 { x = x - 1 }`
- **THEN** output is `while (x > 0) { x = x - 1; }`

### Requirement: Import/export translation

The codegen SHALL translate `import { x, y } from "path"` to `import { x, y } from "path";` and `import * as ns from "path"` to `import * as ns from "path";`. Module paths SHALL be preserved as-is. `pub` declarations SHALL be translated to `export`.

#### Scenario: Named imports

- **WHEN** the input is `import { read, write } from "./fs"`
- **THEN** output is `import { read, write } from "./fs";`

#### Scenario: Namespace import

- **WHEN** the input is `import * as fs from "./fs"`
- **THEN** output is `import * as fs from "./fs";`

### Requirement: Template string translation

The codegen SHALL translate template strings `` `hello ${name}` `` to JavaScript template literals `` `hello ${name}` ``.

#### Scenario: Template with interpolation

- **WHEN** the input is `` `Hello, ${name}!` ``
- **THEN** output is `` `Hello, ${name}!` ``

### Requirement: SWC-based emission

The codegen SHALL construct `swc_ecma_ast` nodes from the AgentScript AST and use `swc_ecma_codegen::Emitter` to produce the final JavaScript text. The output SHALL be valid ES2020+ JavaScript (ESM).

#### Scenario: Output is valid JS

- **WHEN** any valid AgentScript program is compiled
- **THEN** the output SHALL be parseable by a standard JavaScript parser without errors
