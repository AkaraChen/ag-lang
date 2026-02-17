## Why

The `@tool` annotation is parsed and type-checked, but codegen ignores it entirely — no JSON schema is emitted. This means tool-annotated functions compile to plain JS functions without the metadata that LLM tool-calling APIs (OpenAI, Anthropic) require. The language spec (section 7.3) already defines the expected output format (`fn.schema = { ... }`), but it's not implemented. This is the last missing piece for `@tool` to be usable end-to-end.

## What Changes

- Codegen emits a `.schema` property assignment after each `@tool`-annotated function declaration
- A type-to-JSON-Schema mapper converts AG `TypeExpr` nodes to JSON Schema objects (`str` → `"string"`, `num`/`int` → `"number"`, `bool` → `"boolean"`, `[T]` → `array`, struct/map → `object`, `T?` → adds nullable handling)
- Parameters with defaults are excluded from the `required` array
- The `@tool("description")` string is used as the schema's `description` field
- The checker's `tool_registry` (with pre-resolved param types) is passed to codegen for struct field resolution

## Capabilities

### New Capabilities
- `tool-schema-codegen`: JSON schema generation for `@tool`-annotated functions in the codegen phase, including type mapping and schema property emission

### Modified Capabilities
- `codegen-js`: Function declaration translation now emits additional `.schema` assignment for `@tool` functions

## Impact

- **ag-codegen**: New `tool_schema.rs` module for schema generation logic; `translate_fn_decl` extended
- **ag-cli**: Must pass `CheckResult` (specifically `tool_registry`) to codegen
- **ag-checker**: `ToolInfo` may need to include resolved struct field types for nested object schemas
- **Tests**: New codegen tests for schema output; existing tool tests verified
