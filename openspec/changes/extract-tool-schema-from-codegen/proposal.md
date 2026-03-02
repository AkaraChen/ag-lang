## Why

`ag-codegen` depends on `ag-checker` solely to access `Type` and `ToolInfo` for emitting `@tool` JSON Schema. This creates a backwards dependency in the compiler pipeline (`codegen → checker`), violating the intended flow `parser → checker → codegen`. Additionally, the tool schema emission behavior is completely unspecified — neither the `tool-annotation` spec nor the `codegen-js` spec mention it.

## What Changes

- Add `JsonSchema` enum and `ToolSchemaInfo` struct to `ag-ast` as an intermediate representation between checker and codegen
- Checker converts `Type → JsonSchema` when registering `@tool` functions, producing `ToolSchemaInfo` instead of `ToolInfo`
- Codegen consumes `ToolSchemaInfo` (from `ag-ast`) instead of `ToolInfo` (from `ag-checker`)
- **BREAKING**: Remove `ag-checker` dependency from `ag-codegen/Cargo.toml`
- `ag-codegen/src/tool_schema.rs` rewrites to convert `JsonSchema → swc::Expr` instead of `Type → swc::Expr`
- `ag-cli` passes `ToolSchemaInfo` (already available from `CheckResult`) to codegen

## Capabilities

### New Capabilities

- `tool-schema-ir`: Intermediate representation (`JsonSchema`, `ToolSchemaInfo`) for passing tool type information from checker to codegen without direct dependency

### Modified Capabilities

- `tool-annotation`: Add requirement for checker to produce `ToolSchemaInfo` (with `JsonSchema`) instead of raw `ToolInfo` (with `Type`)
- `codegen-js`: Add requirement for `@tool` schema emission (`fnName.schema = { ... }`) which currently exists in code but is unspecified

## Impact

- **Crates modified**: `ag-ast`, `ag-checker`, `ag-codegen`, `ag-cli`
- **Dependency change**: `ag-codegen` drops `ag-checker` from dependencies
- **API change**: `CheckResult.tool_registry` type changes from `HashMap<String, ToolInfo>` to `HashMap<String, ToolSchemaInfo>`
- **No runtime behavior change**: Generated JavaScript output remains identical
