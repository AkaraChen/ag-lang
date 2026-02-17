## Context

DSL captures (`#{...}`) are parsed by the main parser's DSL block handler at `ag-parser/src/lib.rs:680-687`. Currently, when the parser encounters `DslCaptureStart`, it collects tokens until `DslCaptureEnd`, creates a sub-parser, and calls `sub_parser.parse_expr(0)` — which only accepts a single expression.

The lexer already handles brace nesting correctly via `dsl_capture_depth`, so `#{ let x = 1; x + 1 }` already produces the correct token stream (the inner `{` and `}` don't terminate the capture). The issue is purely in the parser.

The AST already has `Expr::Block(Box<Block>)` where `Block { stmts, tail_expr, span }`. The codegen already handles `Expr::Block` via `block_to_expr()` which wraps in an IIFE when statements are present or returns the tail expression directly when there are no statements.

## Goals / Non-Goals

**Goals:**
- Support statement blocks inside DSL captures: `#{let x = compute(); x.result}`
- Maintain full backward compatibility with expression-only captures: `#{name}`, `#{a + b}`
- Add `translate_block` to `CodegenContext` so DSL handlers can translate block captures

**Non-Goals:**
- No changes to the lexer — it already works
- No new AST nodes — `Expr::Block` already exists
- No changes to `block_to_expr` in codegen — it already works
- Not implementing multi-statement DSL directives (that's a separate concern)

## Decisions

### Decision 1: Parse capture as block body, not as block expression

The sub-parser should parse the capture content as the *body* of a block (statements + optional tail expression), not as a block expression (which would require `{` and `}`). The capture's `#{` and `}` serve as the block delimiters — the content inside should NOT have another layer of braces.

```
#{let x = 1; x + 1}     // parsed as Block { stmts: [let x = 1], tail_expr: x + 1 }
```

Not:
```
#{{ let x = 1; x + 1 }}  // don't require double braces
```

**Rationale**: The `#{...}` syntax already provides the block boundary. Requiring `#{{ ... }}` would be ugly and confusing.

### Decision 2: Single expression → use as-is, not wrap in Block

If the capture content is a single expression with no statements, it should be stored as that expression directly (not wrapped in a `Block`). This preserves backward compatibility — existing captures produce identical AST.

**Detection**: After parsing the block body, if `stmts` is empty and `tail_expr` is `Some(expr)`, unwrap and use `expr` directly. Otherwise, wrap in `Expr::Block(Block { stmts, tail_expr })`.

**Rationale**: Backward compatibility. Existing DSL handlers (`ag-dsl-prompt`) expect `Capture` to contain an expression they can pass to `ctx.translate_expr()`. If we always wrap in `Expr::Block`, `translate_expr` still works (it handles `Expr::Block`), but it's cleaner to not change the AST for single-expression captures.

### Decision 3: Add `translate_block` to `CodegenContext`

DSL handlers may want to translate a block capture differently than an expression capture — e.g., a server route handler should produce a function body, not an IIFE. Adding `translate_block(&self, block: &dyn Any) -> Vec<swc_ecma_ast::Stmt>` gives handlers this option.

The default path (`translate_expr` on an `Expr::Block`) still works and wraps in an IIFE, which is correct for most uses. The `translate_block` method provides the alternative of getting raw statements.

**Rationale**: Flexibility for DSL handlers without breaking the existing path.

### Decision 4: New parser method `parse_block_body`

Extract the inner loop of `parse_block()` (lines 1132-1187) into a new method `parse_block_body() -> (Vec<Stmt>, Option<Box<Expr>>)` that doesn't expect `{` and `}` delimiters. Both `parse_block()` and the DSL capture parser will use this method.

**Rationale**: DRY — the block body parsing logic (statements, tail expression detection, semicolons) is identical whether delimited by `{ }` or `#{ }`. Extracting it avoids code duplication.

## Risks / Trade-offs

- **[Minimal risk]** Backward compatibility: Single-expression captures produce identical AST, so existing handlers are unaffected.
- **[Low risk]** `parse_block_body` extraction: Mechanical refactor of existing code into a shared method. No behavioral change for `parse_block`.
- **[No risk]** Lexer: No changes needed. Already handles nested braces.
