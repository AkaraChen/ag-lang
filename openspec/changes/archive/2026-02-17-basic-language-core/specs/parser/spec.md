## ADDED Requirements

### Requirement: Module structure

The parser SHALL accept a token stream and produce a `Module` AST node containing a list of top-level items. Top-level items SHALL be: `import` declarations, `export` declarations, function declarations (`fn`), struct declarations, enum declarations, type alias declarations, and variable declarations (`let`/`mut`/`const`).

#### Scenario: Empty module

- **WHEN** the input is an empty token stream (only EOF)
- **THEN** parser produces a `Module` with an empty items list

#### Scenario: Mixed top-level items

- **WHEN** the input contains `import { x } from "y"\nlet a = 1\nfn foo() -> int { 42 }`
- **THEN** parser produces a `Module` with three items: Import, VarDecl, FnDecl

### Requirement: Variable declarations

The parser SHALL parse `let <ident> (: <type>)? = <expr>` as an immutable binding, `mut <ident> (: <type>)? = <expr>` as a mutable binding, and `const <ident> (: <type>)? = <expr>` as a compile-time constant. Type annotations SHALL be optional (omission triggers inference). The initializer expression SHALL be required.

#### Scenario: Let with type annotation

- **WHEN** the input is `let name: str = "Alice"`
- **THEN** parser produces `VarDecl { kind: Let, name: "name", ty: Some(Named("str")), init: StringLiteral("Alice") }`

#### Scenario: Mut without type annotation

- **WHEN** the input is `mut counter = 0`
- **THEN** parser produces `VarDecl { kind: Mut, name: "counter", ty: None, init: IntLiteral(0) }`

#### Scenario: Const declaration

- **WHEN** the input is `const MAX = 100`
- **THEN** parser produces `VarDecl { kind: Const, name: "MAX", ty: None, init: IntLiteral(100) }`

### Requirement: Function declarations

The parser SHALL parse `fn <ident>(<params>) (-> <type>)? <block>` as a function declaration. Parameters SHALL be `<ident>: <type> (= <default>)?` separated by commas. Return type annotation SHALL be optional. The body SHALL be a block. `pub` before `fn` SHALL mark the function as exported. `async` before `fn` SHALL mark it as async.

#### Scenario: Function with return type

- **WHEN** the input is `fn add(a: int, b: int) -> int { a + b }`
- **THEN** parser produces `FnDecl` with name "add", two params with type `int`, return type `int`, and body containing `Binary(Add, Ident("a"), Ident("b"))`

#### Scenario: Function with default parameter

- **WHEN** the input is `fn greet(name: str, loud: bool = false) -> str { name }`
- **THEN** parser produces `FnDecl` with second param having default value `BoolLiteral(false)`

#### Scenario: Pub async function

- **WHEN** the input is `pub async fn fetch(url: str) -> str { url }`
- **THEN** parser produces `FnDecl` with `is_pub: true` and `is_async: true`

### Requirement: Arrow function expressions

The parser SHALL parse `(<params>) => <expr>` and `(<params>) => <block>` as arrow function expressions. Single-expression arrows SHALL have the expression as implicit return. Block arrows SHALL follow block rules (last expression is implicit return).

#### Scenario: Single-expression arrow

- **WHEN** the input is `let double = (x: int) => x * 2`
- **THEN** parser produces a `VarDecl` whose init is an `Arrow` with one param and body `Binary(Mul, Ident("x"), IntLiteral(2))`

#### Scenario: Block body arrow

- **WHEN** the input is `let f = (x: int) => { let y = x + 1; y }`
- **THEN** parser produces an `Arrow` with a `Block` body where the last expression `y` is the implicit return

### Requirement: Struct declarations

The parser SHALL parse `struct <Ident> { <fields> }` where fields are `<ident>: <type> (= <default>)?,` with trailing comma optional. Fields with `?` suffix on the type SHALL be treated as optional (nullable).

#### Scenario: Struct with optional field

- **WHEN** the input is `struct User { name: str, age: int, email: str? }`
- **THEN** parser produces `StructDecl` with three fields, where `email` has type `Nullable(Named("str"))`

### Requirement: Enum declarations

The parser SHALL parse `enum <Ident> { <variants> }` where each variant is `<Ident> ( "(" <fields> ")" )?`. Variants without fields SHALL be unit variants. Variants with fields SHALL carry associated data.

#### Scenario: Enum with mixed variants

- **WHEN** the input is `enum Status { Pending, Active(since: str), Error(code: int, msg: str) }`
- **THEN** parser produces `EnumDecl` with three variants: `Pending` (no fields), `Active` (one field), `Error` (two fields)

### Requirement: Type alias declarations

The parser SHALL parse `type <Ident> = <type>` as a type alias.

#### Scenario: Simple type alias

- **WHEN** the input is `type ID = str`
- **THEN** parser produces `TypeAlias { name: "ID", ty: Named("str") }`

#### Scenario: Union type alias

- **WHEN** the input is `type Result = str | Error`
- **THEN** parser produces `TypeAlias` with `ty: Union(Named("str"), Named("Error"))`

### Requirement: Type syntax parsing

The parser SHALL parse the following type syntax:
- Named types: `str`, `int`, `num`, `bool`, `nil`, `any`, and user-defined names
- Array types: `[<type>]`
- Map types: `{<type>: <type>}`
- Nullable types: `<type>?`
- Union types: `<type> | <type>`
- Function types: `(<params>) -> <return>`
- Object types: `{ <ident>: <type>, ... }`

#### Scenario: Complex nested type

- **WHEN** the input is `let x: [str | int]? = nil`
- **THEN** parser produces a type `Nullable(Array(Union(Named("str"), Named("int"))))`

#### Scenario: Function type

- **WHEN** the input is `type Handler = (str, int) -> bool`
- **THEN** parser produces `TypeAlias` with `ty: Function(params: [Named("str"), Named("int")], ret: Named("bool"))`

### Requirement: Expression parsing with precedence

The parser SHALL parse expressions using Pratt parsing (precedence climbing) with the following precedence (low to high): assignment, pipe (`|>`), nullish coalesce (`??`), logical or (`||`), logical and (`&&`), equality (`==`, `!=`), comparison (`<`, `>`, `<=`, `>=`), addition (`+`, `-`), multiplication (`*`, `/`, `%`), exponentiation (`**`), unary (`!`, `-`), postfix (`.`, `?.`, `::`, `()`, `[]`, `?`).

#### Scenario: Arithmetic precedence

- **WHEN** the input is `1 + 2 * 3`
- **THEN** parser produces `Binary(Add, IntLiteral(1), Binary(Mul, IntLiteral(2), IntLiteral(3)))`

#### Scenario: Pipe operator

- **WHEN** the input is `data |> parse |> validate`
- **THEN** parser produces `Pipe(Pipe(Ident("data"), Ident("parse")), Ident("validate"))`

#### Scenario: Pipe with placeholder

- **WHEN** the input is `data |> transform(_, options)`
- **THEN** parser produces `Pipe(Ident("data"), Call("transform", [Placeholder, Ident("options")]))`

### Requirement: If expressions

The parser SHALL parse `if <expr> <block> (else (if_expr | block))?` as an if expression. `if` SHALL be usable as both a statement and an expression (returns value of last expression in the taken branch).

#### Scenario: If-else expression

- **WHEN** the input is `let x = if a > b { a } else { b }`
- **THEN** parser produces a `VarDecl` whose init is an `If` expression with both branches

#### Scenario: If-else if chain

- **WHEN** the input is `if a { 1 } else if b { 2 } else { 3 }`
- **THEN** parser produces nested `If` expressions

### Requirement: For-in loops

The parser SHALL parse `for <ident> in <expr> <block>` as a for-in loop.

#### Scenario: For-in loop

- **WHEN** the input is `for item in items { process(item) }`
- **THEN** parser produces `For { binding: "item", iter: Ident("items"), body: Block }`

### Requirement: While loops

The parser SHALL parse `while <expr> <block>` as a while loop.

#### Scenario: While loop

- **WHEN** the input is `while x > 0 { x = x - 1 }`
- **THEN** parser produces `While { condition: Binary(Gt, ...), body: Block }`

### Requirement: Match expressions

The parser SHALL parse `match <expr> { <arms> }` where each arm is `<pattern> (if <expr>)? => <expr> | <block>`. Patterns SHALL include: literal patterns, identifier bindings, wildcard `_`, range `a..b`, struct destructuring `{ field, ... }`, and enum variant `Enum::Variant(bindings)`.

#### Scenario: Match with guard

- **WHEN** the input is `match n { 0 => "zero", n if n > 100 => "big", _ => "other" }`
- **THEN** parser produces `Match` with three arms, the second having a guard expression

#### Scenario: Match with struct destructuring

- **WHEN** the input is `match resp { {status: 200, body} => body, _ => "" }`
- **THEN** parser produces a `Match` with a struct pattern arm

### Requirement: Try-catch statements

The parser SHALL parse `try <block> catch <ident> <block>` as error handling.

#### Scenario: Try-catch

- **WHEN** the input is `try { parse(input) } catch e { log(e) }`
- **THEN** parser produces `TryCatch { try_block: Block, catch_binding: "e", catch_block: Block }`

### Requirement: Import declarations

The parser SHALL parse `import { <idents> } from "<path>"` for named imports and `import * as <ident> from "<path>"` for namespace imports.

#### Scenario: Named imports

- **WHEN** the input is `import { read, write } from "./fs"`
- **THEN** parser produces `Import { names: ["read", "write"], path: "./fs", namespace: false }`

#### Scenario: Namespace import

- **WHEN** the input is `import * as fs from "./fs"`
- **THEN** parser produces `Import { alias: "fs", path: "./fs", namespace: true }`

### Requirement: Block implicit return

The parser SHALL treat the last expression in a block (when not followed by `;`) as the implicit return value. If the last statement ends with `;`, the block evaluates to `nil`.

#### Scenario: Implicit return

- **WHEN** the input is `fn foo() -> int { let x = 1; x + 1 }`
- **THEN** the block body has statements `[VarDecl]` and trailing expression `Binary(Add, Ident("x"), IntLiteral(1))`

#### Scenario: Explicit semicolon suppresses return

- **WHEN** the input is `fn foo() { do_something(); }`
- **THEN** the block body has statements `[ExprStmt(Call)]` and no trailing expression

### Requirement: Error recovery

The parser SHALL NOT abort on the first syntax error. It SHALL collect multiple diagnostics and attempt to recover by synchronizing on tokens like `;`, `}`, and top-level keywords (`fn`, `let`, `struct`, etc.). The parser SHALL return both a partial AST and a list of diagnostics.

#### Scenario: Multiple errors

- **WHEN** the input contains two syntax errors in different functions
- **THEN** parser returns diagnostics for both errors and a partial AST containing what could be parsed

### Requirement: Ret statement

The parser SHALL parse `ret <expr>?` as an explicit return. `ret` without an expression SHALL return `nil`.

#### Scenario: Return with value

- **WHEN** the input is `ret x + 1`
- **THEN** parser produces `Return(Some(Binary(Add, Ident("x"), IntLiteral(1))))`

#### Scenario: Return without value

- **WHEN** the input is `ret`
- **THEN** parser produces `Return(None)`
