## Why

DSL captures (`#{}`) currently only support single expressions. The upcoming `@server` and `@skill` DSL blocks require statement block captures — route handlers need multiple statements (variable bindings, conditionals, returns), and skill steps may contain imperative logic. Without block capture support, these DSL handlers cannot express real-world handlers inline, forcing users to define every handler as a separate named function.

## What Changes

- **BREAKING**: Modify DSL capture parsing to accept both expressions and statement blocks inside `#{...}`
- If the capture content is a single expression (no semicolons, no statements), parse as before — backward compatible
- If the capture content contains statements (let/mut/const bindings, semicolons, control flow), parse as a `Block { stmts, tail_expr }` and wrap in `Expr::Block`
- Update the `dsl-parsing` spec's "Capture expression parsing" requirement to allow statement blocks
- Update the `dsl-core` spec's `CodegenContext` to include `translate_block` for block captures

## Capabilities

### New Capabilities
- `dsl-block-capture`: DSL captures support statement blocks (`#{ let x = 1; let y = 2; x + y }`) in addition to single expressions (`#{name}`)

### Modified Capabilities
- `dsl-parsing`: The "Capture expression parsing" requirement changes from expression-only to expression-or-block
- `dsl-core`: `CodegenContext` gains a `translate_block` method for handlers that need to translate block captures

## Impact

- `crates/ag-parser/src/lib.rs` — capture parsing logic (lines 680-687) changes from `parse_expr(0)` to block-aware parsing
- `crates/ag-dsl-core/src/lib.rs` — `CodegenContext` trait gains `translate_block` method
- `crates/ag-codegen/src/lib.rs` — `CodegenContext` impl adds `translate_block`
- No lexer changes — brace nesting already works correctly
- No AST changes — `Expr::Block(Block)` already exists
- No codegen changes for `Expr::Block` — `block_to_expr` already handles it
- Backward compatible — single-expression captures continue to work as before
