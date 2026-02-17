## 1. Parser Refactor

- [x] 1.1 Extract `parse_block_body` method from `parse_block` — moves the inner loop (stmts + tail_expr parsing, lines 1132-1187) into a new method `parse_block_body(&mut self) -> (Vec<Stmt>, Option<Box<Expr>>)` that stops at `RBrace | Eof`
- [x] 1.2 Update `parse_block` to use `parse_block_body` — consume `{`, call `parse_block_body`, consume `}`, return `Block`
- [x] 1.3 Verify existing tests still pass after refactor — `cargo test -p ag-parser`

## 2. Block Capture Parsing

- [x] 2.1 Update DSL capture parsing to use `parse_block_body` — replace `sub_parser.parse_expr(0)` (line 682) with `sub_parser.parse_block_body()`, then: if stmts is empty and tail_expr is Some(expr), use expr directly; otherwise wrap in `Expr::Block(Block { stmts, tail_expr })`
- [x] 2.2 Add parser tests for single-expression captures — verify backward compatibility: `#{name}`, `#{a + b}`, `#{[x, y]}`, `#{fn(x) { x + 1 }}` all produce unwrapped expressions
- [x] 2.3 Add parser tests for block captures — verify: `#{let x = 1; x + 1}` produces `Expr::Block`, `#{let x = 1; println(x);}` produces `Expr::Block` with no tail, empty `#{}` produces diagnostic

## 3. CodegenContext Extension

- [x] 3.1 Add `translate_block` method to `CodegenContext` trait in `ag-dsl-core` — accepts `&dyn Any`, returns `Vec<swc_ecma_ast::Stmt>`
- [x] 3.2 Implement `translate_block` in `ag-codegen` CodegenContext impl — downcast to `Block`, call `translate_block_with_implicit_return`
- [x] 3.3 Add end-to-end test: a DSL block with a block capture compiles to correct JS — verify the IIFE wrapping works via `translate_expr` on an `Expr::Block`
