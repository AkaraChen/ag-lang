## Why

The `@tool` annotation on `fn` declarations needs parser support. Currently `@js(...)` is the only annotation the parser handles. We need to extend annotation parsing to support `@tool` and `@tool("description")`, and attach the metadata to the `FnDecl` AST node. This enables JSON Schema generation from function signatures.

## What Changes

- Extend the parser to recognize `@tool` and `@tool("description")` before `fn` declarations
- Add `ToolAnnotation` to the AST (attached to `FnDecl`)
- Checker validates `@tool` is only on `fn` (not on struct, extern, etc.)
- Checker validates tool function params are serializable types

## Capabilities

### New Capabilities

- `tool-annotation` — Parser + checker support for @tool annotation on fn declarations

### Modified Capabilities

None. The annotation infrastructure from `@js` already exists; this extends it to a new annotation kind.

## Impact

- `crates/ag-parser` — Recognize and parse `@tool` / `@tool("description")` annotations
- `crates/ag-ast` — Add `ToolAnnotation` variant to annotation types on `FnDecl`
- `crates/ag-checker` — Validate placement (fn-only) and param serializability
