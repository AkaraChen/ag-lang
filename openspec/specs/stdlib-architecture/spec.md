## ADDED Requirements

### Requirement: Compiler resolves std: module prefix

The compiler SHALL recognize import paths starting with `std:` as standard library module references. When encountering `import { ... } from "std:<path>"`, the compiler SHALL resolve the path through the `ag-stdlib` crate rather than treating it as a file system path or npm module.

#### Scenario: std:web/fetch import resolution

- **WHEN** the compiler encounters `import { fetch, Request, Response } from "std:web/fetch"`
- **THEN** it SHALL resolve this to the `web/fetch.ag` module in `ag-stdlib`, parse the extern declarations, and register them in the current compilation unit's symbol table

#### Scenario: std:log import resolution

- **WHEN** the compiler encounters `import { info, warn } from "std:log"`
- **THEN** it SHALL resolve this to the `log.ag` module in `ag-stdlib` and process its declarations

#### Scenario: Unknown std module is error

- **WHEN** the compiler encounters `import { foo } from "std:nonexistent"`
- **THEN** it SHALL produce a diagnostic error: standard library module `std:nonexistent` not found

### Requirement: ag-stdlib crate provides module resolution

The `ag-stdlib` Rust crate SHALL export a function `resolve_std_module(path: &str) -> Option<&str>` that maps `std:*` module paths to their `.ag` source content. The `.ag` files SHALL be embedded in the Rust binary via `include_str!`.

#### Scenario: resolve_std_module for known module

- **WHEN** `resolve_std_module("std:web/fetch")` is called
- **THEN** it SHALL return `Some(...)` containing the `.ag` source text of the `web/fetch` module

#### Scenario: resolve_std_module for unknown module

- **WHEN** `resolve_std_module("std:unknown")` is called
- **THEN** it SHALL return `None`

### Requirement: Layer A modules contain only extern declarations

Layer A modules (`std:web/*`) SHALL consist entirely of `extern` declarations (extern fn, extern struct, extern type) with appropriate `@js` annotations or no annotations (for Web globals). Layer A modules SHALL NOT contain any AG function implementations. They provide compile-time types only, with zero runtime cost.

#### Scenario: web/fetch module content

- **WHEN** the `std:web/fetch` module is loaded
- **THEN** it SHALL contain extern declarations for `fetch`, `Request`, `Response`, `Headers`, `URL`, `URLSearchParams` — all as `extern fn`, `extern struct`, or `extern type` with no function bodies

#### Scenario: Layer A module has no runtime output

- **WHEN** codegen processes a module that only imports from `std:web/fetch`
- **THEN** no import statements for `@agentscript/stdlib` SHALL be generated — all Web APIs are global or already imported via `@js` annotations

### Requirement: Layer B modules contain AG code with JS runtime backing

Layer B modules (`std:log`, `std:encoding`, `std:env`, `std:fs`) SHALL contain AG function declarations that compile to JavaScript. The compiled output SHALL import from the `@agentscript/stdlib` npm package. Layer B `.ag` files MAY contain both extern declarations (for their dependencies) and regular AG function definitions.

#### Scenario: std:log compilation

- **WHEN** AG code imports from `std:log` and calls `info("message")`
- **THEN** the codegen SHALL produce `import { info } from "@agentscript/stdlib/log";` and the call `info("message")`

#### Scenario: Layer B module with internal externs

- **WHEN** a Layer B module internally uses `extern fn` to reference Node.js APIs (e.g., `@js("node:fs/promises") extern fn readFile(...)`)
- **THEN** these extern declarations SHALL be processed normally, generating appropriate JS imports in the compiled output

### Requirement: Layer B codegen maps to @agentscript/stdlib npm paths

When a Layer B standard library module is imported and its functions are called, the codegen SHALL produce import statements targeting the `@agentscript/stdlib` npm package. The import path SHALL follow the pattern `@agentscript/stdlib/<module_name>`.

#### Scenario: std:log maps to @agentscript/stdlib/log

- **WHEN** AG code imports `{ info, warn, error }` from `std:log`
- **THEN** codegen SHALL produce `import { info, warn, error } from "@agentscript/stdlib/log";`

#### Scenario: std:encoding maps to @agentscript/stdlib/encoding

- **WHEN** AG code imports `{ json }` from `std:encoding`
- **THEN** codegen SHALL produce `import { json } from "@agentscript/stdlib/encoding";`

#### Scenario: std:env maps to @agentscript/stdlib/env

- **WHEN** AG code imports `{ get }` from `std:env`
- **THEN** codegen SHALL produce `import { get } from "@agentscript/stdlib/env";`

### Requirement: Standard library modules are parseable by the compiler

All `.ag` files in `ag-stdlib/modules/` SHALL be valid AG source files that the compiler can lex, parse, and type-check. The compiler SHALL process stdlib modules using the same pipeline as user code (lex → parse → check), with the addition of `extern` declaration handling.

#### Scenario: Stdlib module parses without errors

- **WHEN** the compiler loads and parses `ag-stdlib/modules/web/fetch.ag`
- **THEN** it SHALL produce a valid AST with no parse errors

#### Scenario: Stdlib module type-checks

- **WHEN** the compiler type-checks a user module that imports from `std:web/fetch` and calls `fetch("url")`
- **THEN** the checker SHALL resolve the types correctly and produce no type errors

### Requirement: Selective import from std modules

The compiler SHALL support importing specific symbols from std modules, not just the entire module. Only the imported symbols SHALL be registered in the current scope.

#### Scenario: Import specific symbols

- **WHEN** AG code has `import { fetch, Request } from "std:web/fetch"`
- **THEN** only `fetch` and `Request` SHALL be available in scope; `Response`, `Headers`, etc. SHALL NOT be directly accessible

#### Scenario: Import non-existent symbol is error

- **WHEN** AG code has `import { nonexistent } from "std:web/fetch"`
- **THEN** the compiler SHALL produce a diagnostic error: `nonexistent` is not exported by `std:web/fetch`

### Requirement: ag-stdlib crate structure

The `ag-stdlib` crate SHALL be located at `crates/ag-stdlib/` in the workspace. It SHALL contain a `modules/` directory with `.ag` files organized by module hierarchy. The crate SHALL depend on no other ag-lang crates (it is a data-only crate providing embedded source text).

#### Scenario: Crate directory structure

- **WHEN** inspecting the `ag-stdlib` crate
- **THEN** it SHALL have the structure:
  - `crates/ag-stdlib/Cargo.toml`
  - `crates/ag-stdlib/src/lib.rs` (containing `resolve_std_module`)
  - `crates/ag-stdlib/modules/web/fetch.ag`
  - `crates/ag-stdlib/modules/web/crypto.ag`
  - `crates/ag-stdlib/modules/web/encoding.ag`
  - `crates/ag-stdlib/modules/web/streams.ag`
  - `crates/ag-stdlib/modules/web/timers.ag`
  - `crates/ag-stdlib/modules/log.ag`
  - `crates/ag-stdlib/modules/encoding.ag`
  - `crates/ag-stdlib/modules/env.ag`
  - `crates/ag-stdlib/modules/fs.ag`

#### Scenario: Crate has no ag-lang dependencies

- **WHEN** inspecting `crates/ag-stdlib/Cargo.toml`
- **THEN** it SHALL NOT depend on `ag-ast`, `ag-parser`, `ag-checker`, or any other ag-lang compiler crate

### Requirement: Compiler integrates ag-stdlib for module resolution

The compiler (specifically `ag-parser` or a module resolver component) SHALL depend on `ag-stdlib` and call `resolve_std_module` when processing `std:*` imports. The resolved source text SHALL be fed through the normal compiler pipeline.

#### Scenario: End-to-end std import

- **WHEN** a user writes:
  ```
  import { fetch, Response } from "std:web/fetch"

  async fn getData() -> str {
    let resp = await fetch("https://example.com")
    return await resp.text()
  }
  ```
- **THEN** the compiler SHALL:
  1. Resolve `std:web/fetch` via `ag-stdlib`
  2. Parse the extern declarations from the module
  3. Register `fetch` and `Response` in the symbol table
  4. Type-check the function body (fetch returns `Promise<Response>`, resp.text() returns `Promise<str>`)
  5. Generate JS with no import for `fetch` (global Web API) and the compiled function
