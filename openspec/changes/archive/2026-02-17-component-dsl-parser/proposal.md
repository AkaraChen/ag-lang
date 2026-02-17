## Why

Components are React JSX code. The `@component` DSL block should simply wrap that code and let SWC parse it. No `#{}` captures are needed — the JSX is self-contained. However, AG needs to know the component's props interface for type checking. JSDoc annotations on the `export default` provide this — same pattern as `@tool` extracting type info from `fn` signatures. The JSDoc-derived types get injected into AG's type scope so other AG code can reference the component with proper typing.

## What Changes

- `@component` DSL block content is pure React JSX code — no `#{}` captures
- Use `swc_ecma_parser` to parse the JSX content
- The component is the `export default` declaration
- Extract JSDoc comments (`@param`, `@returns`, description) from the export default to derive prop types and documentation
- Inject extracted type information into AG's type scope (like `@tool` injects fn signature types)
- Add `swc_ecma_parser` as a new dependency

## Capabilities

### New Capabilities
- `component-dsl-parser`: `@component` DSL handler that parses embedded JSX via SWC, extracts `export default`, and derives prop types from JSDoc

### Modified Capabilities

(none)

## Impact

- New crate: `crates/ag-dsl-component/` — `lib.rs` implementing `DslHandler` + JSDoc extraction
- `Cargo.toml` workspace — add `ag-dsl-component` member
- New dependency: `swc_ecma_parser` for JSX parsing
- `crates/ag-codegen/` — register component handler in translator
