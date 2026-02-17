## ADDED Requirements

### Requirement: Parser recognizes @js annotation syntax

The parser SHALL recognize the `@js(...)` annotation appearing before an `extern` declaration. The annotation SHALL accept a string literal as the module path, and optionally a `name = "jsName"` parameter for renaming. The parsed annotation SHALL be stored as a `JsAnnotation` attached to the subsequent extern declaration.

#### Scenario: @js with module path only

- **WHEN** the parser encounters `@js("node:fs/promises") extern fn readFile(path: str, encoding: str) -> Promise<str>`
- **THEN** it SHALL produce an `ExternFnDecl` with a `JsAnnotation { module: Some("node:fs/promises"), js_name: None }`

#### Scenario: @js with module path and name

- **WHEN** the parser encounters `@js("node:path", name = "join") extern fn path_join(parts: ...str) -> str`
- **THEN** it SHALL produce an `ExternFnDecl` with a `JsAnnotation { module: Some("node:path"), js_name: Some("join") }`

#### Scenario: No @js annotation means global

- **WHEN** the parser encounters `extern fn fetch(url: str) -> Promise<Response>` without a preceding `@js`
- **THEN** it SHALL produce an `ExternFnDecl` with `js_annotation: None`, indicating a global binding

#### Scenario: @js on extern struct

- **WHEN** the parser encounters `@js("some-module") extern struct Foo { x: int }`
- **THEN** it SHALL produce an `ExternStructDecl` with a `JsAnnotation { module: Some("some-module"), js_name: None }`

#### Scenario: @js on extern type

- **WHEN** the parser encounters `@js("some-module") extern type Bar`
- **THEN** it SHALL produce an `ExternTypeDecl` with a `JsAnnotation { module: Some("some-module"), js_name: None }`

### Requirement: @js annotation only valid before extern

The parser SHALL produce a diagnostic error if `@js(...)` appears before a non-extern declaration (e.g., before a regular `fn`, `struct`, or `let`).

#### Scenario: @js before regular fn is error

- **WHEN** the parser encounters `@js("mod") fn foo() -> str { return "bar" }`
- **THEN** it SHALL produce a diagnostic error: `@js annotation is only valid before extern declarations`

#### Scenario: @js before let is error

- **WHEN** the parser encounters `@js("mod") let x = 5`
- **THEN** it SHALL produce a diagnostic error: `@js annotation is only valid before extern declarations`

### Requirement: @js annotation AST representation

The `JsAnnotation` AST node SHALL contain: `module: Option<String>` (the JS module path, `None` for globals), `js_name: Option<String>` (the JS-side name if different from the AG name, `None` if same), and `span: Span`.

#### Scenario: JsAnnotation fields

- **WHEN** `@js("node:path", name = "join")` is parsed
- **THEN** the `JsAnnotation` SHALL have `module = Some("node:path")`, `js_name = Some("join")`, and a valid `span`

### Requirement: Codegen produces import for @js annotated externs

When an `extern` declaration has a `@js("module")` annotation and the extern is referenced in the compilation unit, the codegen SHALL produce a JavaScript import statement. The import SHALL use named import syntax.

#### Scenario: @js extern fn import generation

- **WHEN** codegen processes `@js("node:fs/promises") extern fn readFile(...)` and `readFile` is called in the module
- **THEN** the output SHALL contain `import { readFile } from "node:fs/promises";`

#### Scenario: @js extern fn with name produces aliased import

- **WHEN** codegen processes `@js("node:path", name = "join") extern fn path_join(...)` and `path_join` is called
- **THEN** the output SHALL contain `import { join as path_join } from "node:path";`

#### Scenario: @js extern struct import generation

- **WHEN** codegen processes `@js("some-module") extern struct Foo { ... }` and `Foo` is referenced
- **THEN** the output SHALL contain `import { Foo } from "some-module";`

#### Scenario: No @js means no import

- **WHEN** codegen processes `extern fn fetch(url: str) -> Promise<Response>` (no `@js`)
- **THEN** no import statement SHALL be generated for `fetch`; it is assumed globally available

### Requirement: Codegen merges imports from same module

When multiple `extern` declarations reference the same JS module via `@js("module")`, the codegen SHALL merge them into a single `import { a, b, c } from "module";` statement.

#### Scenario: Two externs from same module

- **WHEN** codegen processes `@js("node:fs/promises") extern fn readFile(...)` and `@js("node:fs/promises") extern fn writeFile(...)`, and both are referenced
- **THEN** the output SHALL contain exactly one `import { readFile, writeFile } from "node:fs/promises";`

#### Scenario: Externs from different modules stay separate

- **WHEN** codegen processes `@js("node:fs/promises") extern fn readFile(...)` and `@js("node:path") extern fn join(...)`
- **THEN** the output SHALL contain two separate import statements

#### Scenario: Mixed aliased and non-aliased from same module

- **WHEN** codegen processes `@js("mod") extern fn foo(...)` and `@js("mod", name = "bar") extern fn baz(...)`, both referenced
- **THEN** the output SHALL contain `import { foo, bar as baz } from "mod";`

### Requirement: Codegen places imports at module top

All import statements generated from `@js` annotations SHALL be placed at the top of the JavaScript output file, before any other statements.

#### Scenario: Import placement

- **WHEN** codegen processes a module with `@js("mod") extern fn foo(...)` and then regular AG code
- **THEN** the output SHALL have the `import` statement before any other JavaScript statements

### Requirement: Codegen only imports referenced externs

The codegen SHALL only generate import statements for extern declarations that are actually referenced (called or used) in the current compilation unit. Unused extern declarations with `@js` annotations SHALL NOT produce import statements.

#### Scenario: Unused extern is not imported

- **WHEN** `@js("mod") extern fn foo(...)` is declared but never called in the module
- **THEN** no `import { foo } from "mod";` SHALL appear in the output

#### Scenario: Used extern is imported

- **WHEN** `@js("mod") extern fn foo(...)` is declared and `foo()` is called
- **THEN** `import { foo } from "mod";` SHALL appear in the output
