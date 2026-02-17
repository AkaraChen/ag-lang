## ADDED Requirements

### Requirement: Lexer recognizes extern keyword

The lexer SHALL recognize `extern` as a reserved keyword and emit a `Token::Extern` token. The keyword SHALL NOT be usable as an identifier.

#### Scenario: extern keyword tokenization

- **WHEN** the lexer encounters the text `extern`
- **THEN** it SHALL emit `Token::Extern` with the correct span

#### Scenario: extern in identifier position is error

- **WHEN** the lexer encounters `let extern = 5`
- **THEN** the parser SHALL produce a diagnostic error because `extern` is a reserved keyword

### Requirement: Parser produces ExternFnDecl AST node

The parser SHALL parse `extern fn <name>(<params>) -> <return_type>` into an `ExternFnDecl` AST node. The node SHALL contain the function name, parameter list (with types), and return type. The node SHALL NOT contain a function body.

#### Scenario: Simple extern fn

- **WHEN** the parser encounters `extern fn fetch(url: str) -> Promise<Response>`
- **THEN** it SHALL produce an `ExternFnDecl` with name `fetch`, one parameter `url: str`, and return type `Promise<Response>`

#### Scenario: Extern fn with optional parameter

- **WHEN** the parser encounters `extern fn fetch(input: str, init: RequestInit?) -> Promise<Response>`
- **THEN** it SHALL produce an `ExternFnDecl` with two parameters, the second having type `RequestInit?` (optional)

#### Scenario: Extern fn with union type parameter

- **WHEN** the parser encounters `extern fn fetch(input: str | Request) -> Promise<Response>`
- **THEN** it SHALL produce an `ExternFnDecl` with one parameter having union type `str | Request`

#### Scenario: Extern fn with no return type

- **WHEN** the parser encounters `extern fn console_log(msg: any)`
- **THEN** it SHALL produce an `ExternFnDecl` with return type implicitly `nil`

#### Scenario: Extern fn with body is error

- **WHEN** the parser encounters `extern fn foo() -> str { return "bar" }`
- **THEN** it SHALL produce a diagnostic error indicating extern functions cannot have a body

### Requirement: Parser produces ExternStructDecl AST node

The parser SHALL parse `extern struct <Name> { <fields_and_methods> }` into an `ExternStructDecl` AST node. The node SHALL contain the struct name, a list of fields (name + type), and a list of method signatures (name + params + return type, no body).

#### Scenario: Extern struct with fields and methods

- **WHEN** the parser encounters:
  ```
  extern struct Request {
    method: str
    url: str
    headers: Headers
    fn json() -> Promise<any>
    fn text() -> Promise<str>
  }
  ```
- **THEN** it SHALL produce an `ExternStructDecl` with name `Request`, three fields (`method: str`, `url: str`, `headers: Headers`), and two method signatures (`json() -> Promise<any>`, `text() -> Promise<str>`)

#### Scenario: Extern struct with only fields

- **WHEN** the parser encounters `extern struct Point { x: num, y: num }`
- **THEN** it SHALL produce an `ExternStructDecl` with name `Point` and two fields, no methods

#### Scenario: Extern struct with only methods

- **WHEN** the parser encounters `extern struct Headers { fn get(name: str) -> str? fn set(name: str, value: str) -> nil }`
- **THEN** it SHALL produce an `ExternStructDecl` with no fields and two method signatures

#### Scenario: Extern struct method with body is error

- **WHEN** the parser encounters `extern struct Foo { fn bar() -> str { return "baz" } }`
- **THEN** it SHALL produce a diagnostic error indicating extern struct methods cannot have a body

### Requirement: Parser produces ExternTypeDecl AST node

The parser SHALL parse `extern type <Name>` into an `ExternTypeDecl` AST node. This declares an opaque type — the compiler knows the name but not the internal structure.

#### Scenario: Simple extern type

- **WHEN** the parser encounters `extern type Headers`
- **THEN** it SHALL produce an `ExternTypeDecl` with name `Headers`

#### Scenario: Multiple extern types

- **WHEN** the parser encounters `extern type Headers` followed by `extern type URL` followed by `extern type ReadableStream`
- **THEN** it SHALL produce three separate `ExternTypeDecl` nodes

### Requirement: Checker registers extern fn in symbol table

The checker SHALL register each `ExternFnDecl` into the symbol table as a callable function. When an extern fn is called, the checker SHALL validate arguments against the declared parameter types and infer the return type from the declaration.

#### Scenario: Valid call to extern fn

- **WHEN** the checker processes `extern fn fetch(url: str) -> Promise<Response>` and then encounters `let r = fetch("http://example.com")`
- **THEN** it SHALL resolve `fetch` from the symbol table, validate the argument type `str`, and infer `r` as `Promise<Response>`

#### Scenario: Argument type mismatch on extern fn

- **WHEN** the checker processes `extern fn fetch(url: str) -> Promise<Response>` and then encounters `fetch(42)`
- **THEN** it SHALL produce a type error: expected `str`, got `int`

#### Scenario: Duplicate extern fn declaration

- **WHEN** the checker encounters two `extern fn` declarations with the same name in the same scope
- **THEN** it SHALL produce a diagnostic error indicating duplicate declaration

### Requirement: Checker registers extern struct as type

The checker SHALL register each `ExternStructDecl` as a type in the type environment. Field access on values of extern struct type SHALL be type-checked against the declared fields. Method calls SHALL be type-checked against the declared method signatures.

#### Scenario: Field access on extern struct

- **WHEN** the checker processes `extern struct Response { status: int, ok: bool }` and encounters `resp.status`
- **THEN** it SHALL infer the type of `resp.status` as `int`

#### Scenario: Unknown field access on extern struct

- **WHEN** the checker processes `extern struct Response { status: int }` and encounters `resp.foo`
- **THEN** it SHALL produce a type error: field `foo` does not exist on type `Response`

#### Scenario: Method call on extern struct

- **WHEN** the checker processes `extern struct Response { fn json() -> Promise<any> }` and encounters `resp.json()`
- **THEN** it SHALL infer the return type as `Promise<any>`

### Requirement: Checker registers extern type as opaque

The checker SHALL register each `ExternTypeDecl` as an opaque type. Values of an opaque type can be passed around (assigned, passed as arguments, returned) but field access and method calls SHALL produce a diagnostic error.

#### Scenario: Passing opaque type as argument

- **WHEN** `extern type Headers` is declared and a function takes `headers: Headers`, and a value of type `Headers` is passed
- **THEN** the checker SHALL accept the argument

#### Scenario: Field access on opaque type is error

- **WHEN** `extern type Headers` is declared and code attempts `headers.length`
- **THEN** the checker SHALL produce a type error: cannot access fields on opaque type `Headers`

### Requirement: Codegen erases extern fn declarations

The codegen SHALL NOT produce any JavaScript function definition for `ExternFnDecl` nodes. At call sites, the codegen SHALL emit a direct reference to the function name.

#### Scenario: Extern fn call codegen

- **WHEN** codegen processes `extern fn fetch(url: str) -> Promise<Response>` and a call `fetch("http://example.com")`
- **THEN** the output SHALL contain `fetch("http://example.com")` but SHALL NOT contain any `function fetch(...)` definition

### Requirement: Codegen erases extern struct declarations

The codegen SHALL NOT produce any JavaScript class or constructor for `ExternStructDecl` nodes. The type exists only at compile time.

#### Scenario: Extern struct erasure

- **WHEN** codegen processes `extern struct Request { method: str, url: str }` and `new Request("http://example.com")`
- **THEN** the output SHALL contain `new Request("http://example.com")` but SHALL NOT contain any `class Request` definition

### Requirement: Codegen erases extern type declarations

The codegen SHALL NOT produce any JavaScript output for `ExternTypeDecl` nodes. These are purely compile-time type information.

#### Scenario: Extern type erasure

- **WHEN** codegen processes `extern type Headers`
- **THEN** no JavaScript output SHALL be produced for this declaration

### Requirement: Variadic parameter support in extern fn

The parser SHALL support `...T` syntax for the last parameter of an `extern fn`, indicating a rest/variadic parameter. The checker SHALL allow zero or more arguments of type `T` at the variadic position. The codegen SHALL emit JavaScript rest parameter syntax `(...args)`.

#### Scenario: Variadic extern fn parsing

- **WHEN** the parser encounters `extern fn console_log(args: ...any) -> nil`
- **THEN** it SHALL produce an `ExternFnDecl` with one variadic parameter `args` of type `any`

#### Scenario: Variadic call with multiple arguments

- **WHEN** the checker processes `extern fn console_log(args: ...any) -> nil` and encounters `console_log("a", 1, true)`
- **THEN** it SHALL accept the call with three arguments

#### Scenario: Variadic not in last position is error

- **WHEN** the parser encounters `extern fn foo(a: ...str, b: int) -> nil`
- **THEN** it SHALL produce a diagnostic error: variadic parameter must be the last parameter

#### Scenario: Variadic codegen

- **WHEN** codegen processes a call to a variadic extern fn `console_log("hello", "world")`
- **THEN** the output SHALL contain `console_log("hello", "world")` (spread at call site is not needed; the JS function naturally accepts multiple args)
