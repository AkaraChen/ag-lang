# DSL Block Capture

## Purpose
Defines how DSL captures (`#{...}`) support block bodies with statements and tail expressions, including the shared `parse_block_body` method and the `translate_block` codegen interface.

## Requirements

### Requirement: Block body parsing in DSL captures

The parser SHALL parse DSL capture content (`#{...}`) as a block body: zero or more statements followed by an optional tail expression. If the content is a single expression with no statements, the capture SHALL contain that expression directly. If the content has statements, the capture SHALL contain an `Expr::Block(Block { stmts, tail_expr })`.

#### Scenario: Single expression capture (backward compatible)

- **WHEN** a DSL block contains `#{name}`
- **THEN** the capture SHALL contain `Expr::Ident("name")` -- NOT wrapped in a Block

#### Scenario: Single complex expression capture (backward compatible)

- **WHEN** a DSL block contains `#{items.len() + 1}`
- **THEN** the capture SHALL contain `Expr::Binary(...)` -- NOT wrapped in a Block

#### Scenario: Statement block capture with tail expression

- **WHEN** a DSL block contains `#{let x = compute(); let y = transform(x); y.result}`
- **THEN** the capture SHALL contain `Expr::Block(Block { stmts: [VarDecl(x, compute()), VarDecl(y, transform(x))], tail_expr: Some(y.result) })`

#### Scenario: Statement block capture with semicolon-terminated tail

- **WHEN** a DSL block contains `#{let x = 1; println(x);}`
- **THEN** the capture SHALL contain `Expr::Block(Block { stmts: [VarDecl(x, 1), ExprStmt(println(x))], tail_expr: None })`

#### Scenario: Array literal capture (no false positive)

- **WHEN** a DSL block contains `#{[tool_a, tool_b]}`
- **THEN** the capture SHALL contain `Expr::Array(...)` -- NOT misinterpreted as a statement block

#### Scenario: Function expression capture

- **WHEN** a DSL block contains `#{fn(req) { req.json({ status: "ok" }) }}`
- **THEN** the capture SHALL contain the function expression -- braces inside the fn body do not interfere

### Requirement: parse_block_body shared method

The parser SHALL have a `parse_block_body` method that parses statements and an optional tail expression WITHOUT expecting `{` and `}` delimiters. This method SHALL be used by both `parse_block` (for `{ ... }` blocks) and the DSL capture parser (for `#{ ... }` captures).

#### Scenario: parse_block uses parse_block_body

- **WHEN** the parser encounters `{ let x = 1; x + 1 }`
- **THEN** `parse_block` SHALL consume `{`, call `parse_block_body`, consume `}`, and return the block

#### Scenario: DSL capture uses parse_block_body

- **WHEN** the parser encounters a DSL capture with tokens `let`, `x`, `=`, `1`, `;`, `x`, `+`, `1`, `Eof`
- **THEN** the capture parser SHALL call `parse_block_body` and get `(stmts: [VarDecl], tail_expr: Some(x + 1))`

### Requirement: CodegenContext translate_block method

The `CodegenContext` trait in `ag-dsl-core` SHALL include a `translate_block` method that accepts a `&dyn Any` (type-erased `Block`) and returns `Vec<swc_ecma_ast::Stmt>`. This allows DSL handlers to translate block captures into raw statements rather than IIFE-wrapped expressions.

#### Scenario: Handler translates block capture to statements

- **WHEN** a DSL handler receives a capture containing `Expr::Block(block)` and calls `ctx.translate_block(&block)`
- **THEN** it receives a `Vec<Stmt>` representing the translated JavaScript statements with implicit return on the tail expression

#### Scenario: Handler translates expression capture via translate_expr

- **WHEN** a DSL handler receives a capture containing a simple expression and calls `ctx.translate_expr(&expr)`
- **THEN** it receives a `swc_ecma_ast::Expr` -- existing behavior preserved

### Requirement: Capture expression parsing (modified)

Within a DSL capture (`#{...}`), the parser SHALL parse the content as a block body: statements followed by an optional tail expression. If the content is a single expression, it SHALL be used directly (backward compatible). If statements are present, the content SHALL be wrapped in `Expr::Block`.

#### Scenario: Valid expression capture (unchanged)

- **WHEN** a capture contains `#{items.len() + 1}`
- **THEN** parser produces the expression AST as before

#### Scenario: Valid block capture (new)

- **WHEN** a capture contains `#{let x = 1; x + 1}`
- **THEN** parser produces `Expr::Block(Block { stmts: [VarDecl], tail_expr: Some(Binary) })`

#### Scenario: Empty capture

- **WHEN** a capture contains `#{}`
- **THEN** parser produces a diagnostic "empty capture"
