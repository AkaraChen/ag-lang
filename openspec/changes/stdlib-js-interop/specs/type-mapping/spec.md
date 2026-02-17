## ADDED Requirements

### Requirement: Primitive type pass-through

AG primitive types SHALL map directly to their JS equivalents with zero transformation in codegen. `str` maps to JS `string`, `num` maps to JS `number`, `int` maps to JS `number`, `bool` maps to JS `boolean`, `nil` maps to JS `null`, and `any` passes through without constraint.

#### Scenario: str pass-through

- **WHEN** an AG `str` value is passed to an extern fn expecting `str`
- **THEN** the codegen SHALL emit the value directly with no conversion

#### Scenario: int is JS number

- **WHEN** an AG `int` value is used in extern interop
- **THEN** the codegen SHALL emit it as a JS `number` with no conversion (no `Math.trunc`, no runtime guard)

#### Scenario: nil maps to null

- **WHEN** AG code passes `nil` to an extern fn
- **THEN** the codegen SHALL emit JS `null`

### Requirement: Optional type mapping

AG optional type `T?` SHALL map to JS `T | null`. The codegen SHALL not wrap or unwrap optional values — they pass through directly.

#### Scenario: Optional parameter

- **WHEN** an extern fn declares a parameter `init: RequestInit?` and AG code passes `nil`
- **THEN** the codegen SHALL emit `null` at that argument position

#### Scenario: Optional return type

- **WHEN** an extern fn returns `str?` and the result is `nil`
- **THEN** the AG-side value SHALL be `nil` (which is JS `null`)

### Requirement: Array type mapping

AG array type `[T]` SHALL map directly to JS `Array<T>`. No conversion is needed — AG arrays and JS arrays are the same runtime representation.

#### Scenario: Array pass-through

- **WHEN** an AG `[str]` value is passed to an extern fn expecting `[str]`
- **THEN** the codegen SHALL emit the array value directly

#### Scenario: Nested array

- **WHEN** an AG `[[int]]` value is used in extern interop
- **THEN** the codegen SHALL emit a nested JS array with no conversion

### Requirement: Map/object type mapping

AG map type `{K: V}` SHALL map to a JS object / `Record<K, V>`. No conversion is needed at the codegen level.

#### Scenario: Map pass-through

- **WHEN** an AG `{str: int}` value is passed to an extern fn
- **THEN** the codegen SHALL emit the object directly

### Requirement: Function type mapping

AG function type `fn(A) -> B` SHALL map to JS arrow function `(a) => b`. Function references pass through directly between AG and JS.

#### Scenario: Callback parameter

- **WHEN** an extern fn declares `callback: fn(str) -> nil` and AG code passes a function
- **THEN** the codegen SHALL emit the function reference directly

#### Scenario: Extern fn accepting function type

- **WHEN** `extern fn setTimeout(callback: () -> nil, ms: int) -> int` is declared and called with an AG function literal
- **THEN** the codegen SHALL emit `setTimeout(() => { ... }, ms)` where the AG function compiles to its JS equivalent

### Requirement: Struct to JS object structural mapping

AG `struct` types SHALL map to plain JS objects with matching field names. When an AG struct value is passed to JS (e.g., as an argument to extern fn), the codegen SHALL emit an object literal with the same field names. When a JS object is received (e.g., return value from extern fn), the checker SHALL validate field access based on the extern struct declaration.

#### Scenario: AG struct as extern fn argument

- **WHEN** a regular AG struct `struct Config { host: str, port: int }` is passed to an extern fn
- **THEN** the codegen SHALL emit `{ host: "...", port: 42 }` (the normal AG struct compilation)

#### Scenario: Extern struct field access

- **WHEN** an extern fn returns a value of extern struct type and AG code accesses its fields
- **THEN** the checker SHALL validate field names and types against the extern struct declaration, and codegen SHALL emit direct property access

### Requirement: Enum to tagged union mapping

AG `enum` types SHALL map to JS tagged unions with the format `{ tag: "VariantName", ...fields }`. Unit variants produce `{ tag: "Name" }`. Variants with fields produce `{ tag: "Name", field1: value1, ... }`.

#### Scenario: Unit enum variant

- **WHEN** AG code creates an enum value `Status.Active`
- **THEN** the codegen SHALL emit `{ tag: "Active" }`

#### Scenario: Enum variant with fields

- **WHEN** AG code creates `Result.Error(message: "fail")`
- **THEN** the codegen SHALL emit `{ tag: "Error", message: "fail" }`

### Requirement: Promise<T> as built-in generic type

`Promise<T>` SHALL be a built-in generic type in the AG type system. It is the only generic type — no general-purpose generics mechanism is introduced. The parser SHALL recognize `Promise<T>` in type position. The checker SHALL track the inner type `T`.

#### Scenario: Promise in return type

- **WHEN** the parser encounters `-> Promise<Response>` in a function signature
- **THEN** it SHALL parse this as a `Promise` type parameterized by `Response`

#### Scenario: Promise with primitive inner type

- **WHEN** the parser encounters `-> Promise<str>`
- **THEN** it SHALL parse this as a `Promise` parameterized by `str`

#### Scenario: Nested Promise is valid

- **WHEN** the parser encounters `-> Promise<Promise<str>>`
- **THEN** it SHALL parse this as a `Promise` parameterized by `Promise<str>` (no special flattening)

### Requirement: await unwraps Promise<T> to T

The `await` expression SHALL only be valid on expressions of type `Promise<T>`. The result type of `await expr` SHALL be `T` (the inner type of the Promise).

#### Scenario: await on Promise<Response>

- **WHEN** the checker processes `let resp = await fetch("url")` where `fetch` returns `Promise<Response>`
- **THEN** it SHALL infer `resp` as type `Response`

#### Scenario: await on non-Promise is error

- **WHEN** the checker processes `await 42`
- **THEN** it SHALL produce a type error: `await` requires a `Promise<T>` type, got `int`

#### Scenario: await in non-async context is error

- **WHEN** `await fetch("url")` appears inside a non-async function
- **THEN** the checker SHALL produce a diagnostic error: `await` is only valid inside `async` functions

### Requirement: async fn return type wraps as Promise

When a function is declared as `async fn foo() -> T`, the checker SHALL treat the function's external return type as `Promise<T>`. Callers see `Promise<T>`; inside the function body, return statements return `T` directly.

#### Scenario: async fn return type

- **WHEN** `async fn getData() -> str { return "hello" }` is declared and called
- **THEN** the checker SHALL infer the call expression `getData()` as type `Promise<str>`

#### Scenario: async fn with await inside

- **WHEN** `async fn getData() -> str { let r = await fetch("url"); return await r.text() }` is processed
- **THEN** the checker SHALL accept `await` inside the async function body and validate inner types correctly

### Requirement: Promise codegen is direct pass-through

The codegen SHALL not transform `Promise<T>` in any way. `await expr` compiles to JS `await expr`. `async fn` compiles to JS `async function`. Promise is a native JS concept.

#### Scenario: await codegen

- **WHEN** codegen processes `let resp = await fetch("url")`
- **THEN** the output SHALL be `const resp = await fetch("url");` (or equivalent JS)

#### Scenario: async fn codegen

- **WHEN** codegen processes `async fn getData() -> str { ... }`
- **THEN** the output SHALL be `async function getData() { ... }`

### Requirement: Checker validates extern parameter types against AG types

When an AG expression is passed as an argument to an extern fn, the checker SHALL verify that the AG type is compatible with the declared extern parameter type using the type mapping rules. Incompatible types SHALL produce a diagnostic error.

#### Scenario: Compatible types accepted

- **WHEN** `extern fn foo(x: str) -> nil` and AG code calls `foo("hello")`
- **THEN** the checker SHALL accept: AG `str` is compatible with extern `str`

#### Scenario: Incompatible types rejected

- **WHEN** `extern fn foo(x: str) -> nil` and AG code calls `foo(42)`
- **THEN** the checker SHALL reject: AG `int` is not compatible with extern `str`

#### Scenario: AG struct compatible with extern struct parameter

- **WHEN** `extern struct Config { host: str }` and `extern fn setup(c: Config) -> nil`, and AG code passes a value with field `host: str`
- **THEN** the checker SHALL accept (structural subtyping)
