## Why

HTTP server declarations use `@server` DSL blocks with `@port`, `@host`, `@middleware`, and route directives (`@get`, `@post`, `@put`, `@delete`, `@patch`). We need an `ag-dsl-server` crate to parse these.

## What Changes

- New `ag-dsl-server` crate with lexer, parser, validator
- Lexer recognizes: `@port`, `@host`, `@middleware`, `@get`, `@post`, `@put`, `@delete`, `@patch`
- Route directives have path patterns: `@get /path/to/:param`
- Route handlers and middleware are `#{}` captures
- Parser produces `ServerTemplate` AST with: port, host, middleware list, route list
- Each route has: method, path pattern, handler capture
- Validator checks no duplicate routes for same method+path
- No codegen

## Capabilities

### New Capabilities

- `server-dsl-parser` — Lexer, parser, and validator for @server DSL blocks

### Modified Capabilities

None.

## Impact

- New crate `crates/ag-dsl-server`
- Update `Cargo.toml` workspace members
