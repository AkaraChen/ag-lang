## ADDED Requirements

### Requirement: DslHandler trait definition

The `ag-dsl-core` crate SHALL define a `DslHandler` trait with a `handle` method. The method SHALL accept a `&DslBlock` and `&mut dyn CodegenContext`, and return `Result<Vec<swc_ecma_ast::ModuleItem>, DslError>`. This trait SHALL be the sole interface between the host compiler and any DSL implementation.

#### Scenario: Handler trait is implementable

- **WHEN** a new DSL crate (e.g., `ag-dsl-prompt`) implements `DslHandler`
- **THEN** it can be registered with the host codegen and invoked for matching DSL blocks

### Requirement: CodegenContext trait definition

The `ag-dsl-core` crate SHALL define a `CodegenContext` trait with a `translate_expr` method that converts a host AST expression (received as `&dyn Any`) into a `swc_ecma_ast::Expr`. This allows DSL handlers to translate captured AgentScript expressions to JavaScript without depending on `ag-ast` directly.

#### Scenario: Handler translates capture via context

- **WHEN** a handler receives a `DslPart::Capture` and calls `ctx.translate_expr(&capture)`
- **THEN** it receives a `swc_ecma_ast::Expr` representing the JavaScript translation

### Requirement: DslBlock shared type

The `ag-dsl-core` crate SHALL define `DslBlock` with fields: `kind: String`, `name: String`, `content: DslContent`, `span: Span`. `DslContent` SHALL be an enum with variants `Inline { parts: Vec<DslPart> }` and `FileRef { path: String, span: Span }`.

#### Scenario: Inline block data

- **WHEN** the host compiler parses `@prompt system ``` Hello #{name} ```\n`
- **THEN** it constructs `DslBlock { kind: "prompt", name: "system", content: Inline { parts: [Text("Hello "), Capture(...)] } }`

#### Scenario: File reference data

- **WHEN** the host compiler parses `@prompt system from "./prompt.txt"`
- **THEN** it constructs `DslBlock { kind: "prompt", name: "system", content: FileRef { path: "./prompt.txt", ... } }`

### Requirement: DslPart with type-erased captures

`DslPart` SHALL be an enum with variants `Text(String, Span)` and `Capture(Box<dyn Any>, Span)`. The `Capture` variant SHALL use type erasure (`Box<dyn Any>`) so that `ag-dsl-core` does NOT depend on `ag-ast`. Handlers SHALL use `CodegenContext::translate_expr` to process captures rather than downcasting directly.

#### Scenario: Capture is opaque to core

- **WHEN** `ag-dsl-core` is compiled
- **THEN** it does NOT have a dependency on `ag-ast`

#### Scenario: Handler processes capture

- **WHEN** a handler encounters `DslPart::Capture(any, span)`
- **THEN** it passes `any` to `ctx.translate_expr()` and receives a JS expression

### Requirement: DslError type

The `ag-dsl-core` crate SHALL define `DslError` with fields `message: String` and `span: Option<Span>`. This type SHALL be used by all handlers to report errors from DSL processing.

#### Scenario: Handler reports error

- **WHEN** a handler encounters invalid DSL content
- **THEN** it returns `Err(DslError { message: "...", span: Some(...) })`

### Requirement: Span type

The `ag-dsl-core` crate SHALL define or re-export a `Span` type with `start: u32` and `end: u32` fields representing byte offsets. This SHALL be the same `Span` type used across all DSL crates.

#### Scenario: Consistent span usage

- **WHEN** any DSL crate creates a span
- **THEN** it uses the `Span` type from `ag-dsl-core`
