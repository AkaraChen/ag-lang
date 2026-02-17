## Why

The `@server` DSL has full parsing infrastructure (`ag-dsl-server`: lexer, parser, validator, AST) but cannot emit JavaScript — it has no `codegen.rs` or `handler.rs`. This blocks users from compiling any `.ag` file that uses `@server` blocks. The prompt DSL (`ag-dsl-prompt`) is the only DSL with working codegen; server is the next logical step since it has a clear compilation target (Hono HTTP server).

## What Changes

- Add `codegen.rs` to `ag-dsl-server` — generates SWC AST nodes that produce Hono-based JavaScript (imports, `new Hono()`, middleware registration, route registration, `serve()` call)
- Add `handler.rs` to `ag-dsl-server` — implements `DslHandler` trait, wiring lexer → parser → validator → codegen
- Register `ServerDslHandler` in `ag-codegen` so `@server` blocks are dispatched during compilation
- Add SWC dependencies to `ag-dsl-server/Cargo.toml`
- Add `ag-dsl-server` dependency to `ag-codegen/Cargo.toml`
- Add end-to-end tests in `ag-codegen` for server DSL compilation

## Capabilities

### New Capabilities
- `server-dsl-codegen`: Code generation for `@server` DSL blocks — emitting Hono-based JavaScript from parsed `ServerTemplate` AST, including route registration, middleware, and the `serve()` call

### Modified Capabilities
- `prompt-dsl-codegen`: No requirement changes — used as reference only

## Impact

- **Crates modified**: `ag-dsl-server` (add codegen.rs, handler.rs, update lib.rs, Cargo.toml), `ag-codegen` (register handler, add dependency, add integration tests)
- **Dependencies added**: SWC crates to `ag-dsl-server`, `ag-dsl-server` to `ag-codegen`
- **Runtime dependencies**: Compiled output will import from `hono` and `@agentscript/serve` (npm packages, not part of this change)
- **No breaking changes** — purely additive
