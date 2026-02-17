## Context

The `ag-dsl-server` crate has complete parsing infrastructure (lexer, parser, validator, AST) producing a `ServerTemplate` with port, host, middlewares, and routes. The `ag-dsl-prompt` crate is the only DSL with working codegen — it follows a 5-step pipeline: lex → parse → validate → collect captures → codegen. The server DSL needs `codegen.rs` and `handler.rs` to complete the same pipeline.

The target JavaScript output is a Hono-based HTTP server (spec section 10.3). Hono is already the chosen framework in the spec, and `@agentscript/serve` is the runtime wrapper for `serve()`.

## Goals / Non-Goals

**Goals:**
- Emit correct Hono-based JavaScript from `@server` DSL blocks
- Support all directives: `@port`, `@host`, `@middleware`, `@get/@post/@put/@delete/@patch`
- Translate route path parameters (`:name`, `*`) to Hono path syntax
- Translate handler captures via `CodegenContext::translate_expr`
- Follow the exact same architectural pattern as `ag-dsl-prompt`
- Full test coverage (unit tests in codegen.rs, handler.rs; integration tests in ag-codegen)

**Non-Goals:**
- Type-checking server routes or handler signatures (future `ag-checker` work)
- Runtime `@agentscript/serve` package implementation (npm side)
- WebSocket support, SSE, or streaming
- FileRef form for `@server` blocks (not meaningful for servers)

## Decisions

### 1. Output shape: Hono constructor + method calls + serve()

The codegen emits:
```javascript
import { Hono } from "hono";
import { serve } from "@agentscript/serve";
const <name> = new Hono();
<name>.use(<middleware_expr>);           // per @middleware
<name>.get("/path", <handler_expr>);     // per route
serve(<name>, { port: N, host: "..." }); // at end
```

**Rationale:** This matches the spec exactly (section 10.3) and is the simplest correct output. Each route handler capture is translated via `ctx.translate_expr()`, which handles the full AG expression tree (including async fns, arrow fns, etc.).

**Alternative considered:** Wrapping everything in a factory function — rejected because the spec shows top-level `const app = new Hono()` directly.

### 2. Path segment serialization

`PathSegment::Literal("users")` → `"/users"`, `PathSegment::Param("id")` → `"/:id"`, `PathSegment::Wildcard` → `"/*"`. Segments are joined to produce the full path string (e.g. `"/users/:id"`).

**Rationale:** Hono uses Express-style path params natively, so the parsed representation round-trips cleanly.

### 3. Handler as ServerDslHandler struct

A `ServerDslHandler` struct implementing `DslHandler`, following the identical 5-step pattern from `PromptDslHandler`:
1. Lex `DslPart` slice → `ServerToken` stream
2. Parse tokens → `ServerTemplate`
3. Validate template (non-fatal warnings)
4. Collect captures from parts
5. Call `codegen::generate()`

**Rationale:** Consistency with `ag-dsl-prompt`. The trait interface is already defined.

### 4. No FileRef support for @server

Unlike `@prompt` which supports `@prompt name from "path"`, `@server` blocks must be inline — a file reference doesn't make sense for route declarations.

**Rationale:** The handler will return a `DslError` for `DslContent::FileRef` inputs.

### 5. SWC dependency versions match existing crates

`swc_common = "18"`, `swc_ecma_ast = "20"`, `swc_ecma_codegen = "23"` — same as `ag-dsl-prompt`.

## Risks / Trade-offs

- **[Risk] Capture index correctness** — Route handlers and middlewares reference captures by index. The codegen must maintain the same indexing as the lexer/parser. → Mitigation: Tests verify capture indices match expected handler positions.
- **[Risk] Path serialization edge cases** — Root path `/` and trailing wildcards need careful handling. → Mitigation: Dedicated test cases for `/`, `/*`, `/users/:id`, and compound paths.
- **[Risk] Port/host omission** — If no `@port`/`@host` directives, `serve()` call omits those config fields. → Mitigation: The `serve()` options object only includes fields that are `Some` in the template.
