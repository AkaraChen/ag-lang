## ADDED Requirements

### Requirement: DslHandler trait

The codegen module SHALL define a `DslHandler` trait with a `handle` method that takes a `DslBlock` and a `CodegenContext`, and returns a `Result<Vec<ModuleItem>, CodegenError>`. The `CodegenContext` SHALL provide a `translate_expr` method for converting captured AgentScript expressions to SWC expressions.

#### Scenario: Handler trait signature

- **WHEN** a new DSL handler is implemented
- **THEN** it implements `DslHandler` with `fn handle(&self, block: &DslBlock, ctx: &mut CodegenContext) -> Result<Vec<ModuleItem>, CodegenError>`

### Requirement: Handler registration

The `Translator` (codegen entry point) SHALL maintain a `HashMap<String, Box<dyn DslHandler>>` mapping DSL kind names to their handlers. Handlers SHALL be registered before codegen begins. The registration API SHALL be `register_dsl_handler(kind: &str, handler: Box<dyn DslHandler>)`.

#### Scenario: Register and use handler

- **WHEN** a handler is registered for kind "prompt" and the AST contains `DslBlock { kind: "prompt", ... }`
- **THEN** codegen invokes the registered "prompt" handler for that block

### Requirement: Unregistered DSL kind error

When codegen encounters a `DslBlock` whose `kind` has no registered handler, it SHALL produce a `CodegenError` with message "no handler registered for DSL kind `<kind>`" and the span of the `DslBlock`.

#### Scenario: Missing handler

- **WHEN** the AST contains `DslBlock { kind: "graphql", ... }` but no handler is registered for "graphql"
- **THEN** codegen produces error "no handler registered for DSL kind `graphql`"

### Requirement: CodegenContext provides expression translation

The `CodegenContext` passed to handlers SHALL provide `translate_expr(&mut self, expr: &ag_ast::Expr) -> swc_ecma_ast::Expr` so that handlers can translate captured AgentScript expressions into JavaScript AST nodes.

#### Scenario: Handler translates capture

- **WHEN** a handler receives `DslPart::Capture(expr)` and calls `ctx.translate_expr(expr)`
- **THEN** it receives back the corresponding `swc_ecma_ast::Expr` node

### Requirement: Built-in prompt handler

The codegen module SHALL include a built-in prompt handler registered for kind `"prompt"`. For inline blocks, it SHALL translate the DSL content to a JavaScript template literal where `DslPart::Text` segments become literal string parts and `DslPart::Capture(expr)` segments become `${}` interpolation expressions. The result SHALL be a `const <name> = `...`;` declaration.

#### Scenario: Prompt with captures

- **WHEN** the input is `@prompt system ``` You are #{role}. Answer in #{lang}. ```\n`
- **THEN** codegen produces `` const system = `You are ${role}. Answer in ${lang}.`; ``

#### Scenario: Prompt without captures

- **WHEN** the input is `@prompt greeting ``` Hello, world! ```\n`
- **THEN** codegen produces `` const greeting = `Hello, world!`; ``

### Requirement: Built-in prompt handler FileRef

For `DslContent::FileRef(path)`, the prompt handler SHALL translate to a file read at runtime: `const <name> = await fs.readFile("<path>", "utf-8");` or equivalent. The exact translation MAY vary but SHALL produce valid JavaScript that loads the file content.

#### Scenario: Prompt from file

- **WHEN** the input is `@prompt system from "./system-prompt.txt"`
- **THEN** codegen produces JavaScript that reads the file content into a `system` constant

### Requirement: Handler receives complete DslBlock

Handlers SHALL receive the full `DslBlock` including `kind`, `name`, `content`, and `span`. This allows handlers to use the block name for variable naming, the span for error reporting, and the content for code generation.

#### Scenario: Handler uses block name

- **WHEN** the prompt handler receives `DslBlock { kind: "prompt", name: "system", ... }`
- **THEN** it uses "system" as the JavaScript variable name in the output

### Requirement: Checker validates capture expressions

The type checker SHALL traverse `DslBlock` nodes and type-check all `DslPart::Capture(expr)` expressions using standard expression checking. Type errors within captures SHALL be reported. The checker SHALL NOT constrain the capture expression's type — any type is accepted.

#### Scenario: Valid capture expression

- **WHEN** a capture contains `#{user.name}` and `user` is declared as `User { name: str }`
- **THEN** checker resolves the capture expression type to `str` without error

#### Scenario: Invalid capture expression

- **WHEN** a capture contains `#{undefined_var}`
- **THEN** checker produces diagnostic "undefined variable `undefined_var`"

#### Scenario: Capture type not constrained

- **WHEN** a capture contains `#{42}` (type `int`) inside a prompt DSL block
- **THEN** checker accepts without error — type constraint is the handler's responsibility, not the checker's
