## ADDED Requirements

### Requirement: Server DSL codegen module
The `ag-dsl-server` crate SHALL have a `codegen.rs` module that generates SWC AST nodes from a `ServerTemplate`. The `generate()` function SHALL accept a `ServerTemplate`, a slice of type-erased capture references, and a `CodegenContext`, and return `Vec<swc::ModuleItem>`.

#### Scenario: Minimal server with one route
- **WHEN** a `ServerTemplate` has name "app", port 3000, no host, no middlewares, and one GET route at "/health" with handler capture index 0
- **THEN** the generated JavaScript SHALL contain:
  - `import { Hono } from "hono"`
  - `import { serve } from "@agentscript/serve"`
  - `const app = new Hono()`
  - `app.get("/health", <translated_capture_0>)`
  - `serve(app, { port: 3000 })`

#### Scenario: Server with host and port
- **WHEN** a `ServerTemplate` has port 8080 and host "0.0.0.0"
- **THEN** the `serve()` call SHALL include both: `serve(app, { port: 8080, host: "0.0.0.0" })`

#### Scenario: Server with no port and no host
- **WHEN** a `ServerTemplate` has no port and no host
- **THEN** the `serve()` call SHALL pass an empty options object: `serve(app, {})`

### Requirement: Middleware codegen
The codegen SHALL emit `<name>.use(<expr>)` for each middleware capture, in the order they appear in the template, before any route registrations.

#### Scenario: Multiple middlewares
- **WHEN** a `ServerTemplate` has middleware captures at indices 0 and 1
- **THEN** the output SHALL contain `app.use(<capture_0>)` and `app.use(<capture_1>)` in order, before any route method calls

### Requirement: Route codegen with all HTTP methods
The codegen SHALL emit `<name>.<method>("<path>", <handler>)` for each route, supporting GET, POST, PUT, DELETE, and PATCH methods.

#### Scenario: All five HTTP methods
- **WHEN** a `ServerTemplate` has routes for GET, POST, PUT, DELETE, and PATCH
- **THEN** the output SHALL contain `app.get(...)`, `app.post(...)`, `app.put(...)`, `app.delete(...)`, `app.patch(...)` respectively

### Requirement: Path serialization
Route paths SHALL be serialized from `Vec<PathSegment>` to Hono-compatible path strings. `Literal("x")` becomes `/x`, `Param("id")` becomes `/:id`, `Wildcard` becomes `/*`.

#### Scenario: Root path
- **WHEN** a route has path `[Literal("")]` or empty segments representing "/"
- **THEN** the serialized path SHALL be `"/"`

#### Scenario: Parameterized path
- **WHEN** a route has path `[Literal("users"), Param("id")]`
- **THEN** the serialized path SHALL be `"/users/:id"`

#### Scenario: Wildcard path
- **WHEN** a route has path `[Literal("files"), Wildcard]`
- **THEN** the serialized path SHALL be `"/files/*"`

### Requirement: Capture translation
Handler and middleware captures SHALL be translated via `ctx.translate_expr()`. The codegen SHALL extract captures from the `DslPart` slice (filtering for `DslPart::Capture` variants) and index into them using the capture indices stored in the `ServerTemplate`.

#### Scenario: Handler capture translation
- **WHEN** a route has `handler_capture: 2` and the captures slice has 3 entries
- **THEN** `ctx.translate_expr(captures[2])` SHALL be called and its result used as the route handler argument

### Requirement: Server DSL handler
The `ag-dsl-server` crate SHALL have a `handler.rs` module with a `ServerDslHandler` struct implementing `ag_dsl_core::DslHandler`. It SHALL follow the 5-step pipeline: lex → parse → validate → collect captures → codegen.

#### Scenario: Inline block handling
- **WHEN** `handle()` receives a `DslBlock` with `DslContent::Inline` containing server directives
- **THEN** it SHALL return `Ok(Vec<ModuleItem>)` with the generated Hono JavaScript

#### Scenario: FileRef block rejection
- **WHEN** `handle()` receives a `DslBlock` with `DslContent::FileRef`
- **THEN** it SHALL return `Err(DslError)` with a message indicating file references are not supported for server blocks

#### Scenario: Parse error propagation
- **WHEN** the server parser returns diagnostics (e.g., invalid port)
- **THEN** the handler SHALL return `Err(DslError)` with the diagnostic messages joined

### Requirement: Handler registration in ag-codegen
The `ServerDslHandler` SHALL be registered in the `ag-codegen` crate's `codegen()` function for the DSL kind `"server"`, so that `@server` blocks are dispatched to it during compilation.

#### Scenario: Server block dispatch
- **WHEN** an AG module contains `@server app ``` @port 3000 @get /health #{handler} ```
- **THEN** the codegen pipeline SHALL produce JavaScript containing Hono route registrations

#### Scenario: Unregistered kind still errors for other DSLs
- **WHEN** an AG module contains a `@unknown` DSL block
- **THEN** codegen SHALL still return an error for unregistered kinds (existing behavior preserved)

### Requirement: Import deduplication
The codegen SHALL emit exactly two import statements: one for `Hono` from `"hono"` and one for `serve` from `"@agentscript/serve"`. These SHALL NOT be duplicated regardless of the number of routes.

#### Scenario: Multiple routes produce single import set
- **WHEN** a `ServerTemplate` has 5 routes
- **THEN** the output SHALL contain exactly one `import { Hono }` and one `import { serve }` statement

### Requirement: SWC dependencies
The `ag-dsl-server/Cargo.toml` SHALL include `swc_common = "18"`, `swc_ecma_ast = "20"`, and `swc_ecma_codegen = "23"` as dependencies. The `ag-codegen/Cargo.toml` SHALL include `ag-dsl-server` as a dependency.

#### Scenario: Crate compiles with SWC dependencies
- **WHEN** `cargo build -p ag-dsl-server` is run
- **THEN** the build SHALL succeed with no errors
