## 1. Dependencies & Module Setup

- [ ] 1.1 Add SWC dependencies (`swc_common = "18"`, `swc_ecma_ast = "20"`, `swc_ecma_codegen = "23"`) to `ag-dsl-server/Cargo.toml`
- [ ] 1.2 Add `codegen` and `handler` modules to `ag-dsl-server/src/lib.rs`
- [ ] 1.3 Add `ag-dsl-server` dependency to `ag-codegen/Cargo.toml`

## 2. Codegen Implementation

- [ ] 2.1 Create `ag-dsl-server/src/codegen.rs` with `generate()` function, SWC helper functions (`ident`, `str_lit`, `expr_or_spread`, `make_prop`), and `emit_module()` test helper
- [ ] 2.2 Implement import generation: `import { Hono } from "hono"` and `import { serve } from "@agentscript/serve"`
- [ ] 2.3 Implement `const <name> = new Hono()` declaration
- [ ] 2.4 Implement path serialization: `Vec<PathSegment>` → Hono path string (`/users/:id`, `/*`, `/`)
- [ ] 2.5 Implement middleware codegen: `<name>.use(<capture_expr>)` for each middleware
- [ ] 2.6 Implement route codegen: `<name>.<method>("<path>", <handler_expr>)` for all 5 HTTP methods
- [ ] 2.7 Implement `serve(<name>, { port: N, host: "..." })` call with optional port/host
- [ ] 2.8 Add unit tests for codegen: minimal server, port+host, middlewares, all HTTP methods, parameterized paths, wildcard paths, no-port/no-host

## 3. Handler Implementation

- [ ] 3.1 Create `ag-dsl-server/src/handler.rs` with `ServerDslHandler` struct implementing `DslHandler`
- [ ] 3.2 Implement 5-step pipeline in `handle()`: lex → parse → validate → collect captures → codegen
- [ ] 3.3 Return `DslError` for `DslContent::FileRef` with descriptive message
- [ ] 3.4 Add unit tests: inline block handling, file ref rejection, parse error propagation

## 4. Registration & Integration

- [ ] 4.1 Register `ServerDslHandler` for kind `"server"` in `ag-codegen/src/lib.rs` `codegen()` function
- [ ] 4.2 Add end-to-end integration tests in `ag-codegen` parsing `@server` blocks from AG source strings and verifying JavaScript output
- [ ] 4.3 Verify `cargo test --workspace` passes with all new and existing tests
