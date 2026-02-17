## 1. Crate Setup

- [ ] 1.1 Create `crates/ag-dsl-server/` crate with `Cargo.toml`, depend on `ag-dsl-core`
- [ ] 1.2 Add `ag-dsl-server` to workspace members in root `Cargo.toml`
- [ ] 1.3 Create `src/lib.rs` with public module declarations: `pub mod ast`, `pub mod lexer`, `pub mod parser`, `pub mod validator`
- [ ] 1.4 `cargo build` to verify the crate compiles

## 2. Server AST Definition

- [ ] 2.1 Define `PathSegment` enum: `Literal(String)`, `Param(String)`, `Wildcard`
- [ ] 2.2 Define `HttpMethod` enum: `Get`, `Post`, `Put`, `Delete`, `Patch`
- [ ] 2.3 Define `Route { method: HttpMethod, path: Vec<PathSegment>, handler_capture: usize }`
- [ ] 2.4 Define `ServerTemplate { name: String, port: Option<u16>, host: Option<String>, middlewares: Vec<usize>, routes: Vec<Route> }`

## 3. Server Lexer

- [ ] 3.1 Define `ServerToken` enum: `DirectivePort`, `DirectiveHost`, `DirectiveMiddleware`, `DirectiveGet`, `DirectivePost`, `DirectivePut`, `DirectiveDelete`, `DirectivePatch`, `NumberLiteral(u64)`, `StringLiteral(String)`, `Path(String)`, `Capture(usize)`, `Eof`
- [ ] 3.2 Implement `lex(parts: &[DslPart]) -> Result<Vec<ServerToken>, Vec<Diagnostic>>` entry point
- [ ] 3.3 Implement Text segment scanning: line-start `@` + known keyword → directive token
- [ ] 3.4 Implement `@port` followed by unsigned integer literal scanning
- [ ] 3.5 Implement `@host` followed by double-quoted string literal scanning
- [ ] 3.6 Implement `@middleware` expecting a subsequent capture
- [ ] 3.7 Implement route directive scanning: `@get`/`@post`/`@put`/`@delete`/`@patch` followed by path pattern (text up to whitespace before capture)
- [ ] 3.8 Implement `DslPart::Capture` → `ServerToken::Capture(index)` passthrough with incrementing counter
- [ ] 3.9 Implement error reporting for unknown directives and unexpected `@` mid-line
- [ ] 3.10 Write lexer unit tests covering all server-dsl-parser spec lexer scenarios

## 4. Server Parser

- [ ] 4.1 Implement `parse(tokens: &[ServerToken]) -> Result<ServerTemplate, Vec<Diagnostic>>` entry point
- [ ] 4.2 Implement main loop: consume directive tokens and dispatch to parse functions
- [ ] 4.3 Implement `parse_port()`: expect `NumberLiteral`, set `port`
- [ ] 4.4 Implement `parse_host()`: expect `StringLiteral`, set `host`
- [ ] 4.5 Implement `parse_middleware()`: expect `Capture`, push to `middlewares`
- [ ] 4.6 Implement `parse_route(method)`: expect `Path` → parse path segments → expect `Capture` → push `Route`
- [ ] 4.7 Implement `parse_path_segments(path: &str) -> Result<Vec<PathSegment>, Diagnostic>`: split on `/`, classify each segment
- [ ] 4.8 Implement error handling: missing path, missing capture, missing port number, missing host string
- [ ] 4.9 Write parser unit tests covering all server-dsl-parser spec parser scenarios

## 5. Server Validator

- [ ] 5.1 Implement `validate(template: &ServerTemplate) -> Vec<Diagnostic>` entry point
- [ ] 5.2 Check duplicate `@port` (error)
- [ ] 5.3 Check duplicate `@host` (error)
- [ ] 5.4 Check duplicate routes with same method + path (error)
- [ ] 5.5 Check wildcard not in last position (error)
- [ ] 5.6 Check port range 1..65535 (error)
- [ ] 5.7 Check no routes defined (warning)
- [ ] 5.8 Write validator unit tests covering all server-dsl-parser spec validator scenarios

## 6. Integration

- [ ] 6.1 Write end-to-end test: `Vec<DslPart>` → lex → parse → validate → verify `ServerTemplate` output for a complete server block
- [ ] 6.2 Write error scenario tests: missing handler, duplicate routes, invalid path patterns, port out of range
- [ ] 6.3 Verify `cargo test` passes for all `ag-dsl-server` tests
