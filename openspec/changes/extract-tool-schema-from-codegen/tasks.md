## 1. Add IR types to ag-ast

- [x] 1.1 Add `JsonSchema` enum to `ag-ast/src/lib.rs` with variants: String, Number, Integer, Boolean, Null, Any, Array, Object, AnyOf
- [x] 1.2 Add `ToolSchemaInfo` struct to `ag-ast/src/lib.rs` with fields: description, params

## 2. Update ag-checker to produce ToolSchemaInfo

- [x] 2.1 Add `type_to_json_schema(ty: &Type) -> JsonSchema` conversion function in `ag-checker/src/lib.rs`
- [x] 2.2 Replace `ToolInfo` with `ToolSchemaInfo` (from ag-ast) in `Checker.tool_registry` and `CheckResult`
- [x] 2.3 Update tool registration logic to convert `Type` → `JsonSchema` when registering @tool functions
- [x] 2.4 Remove `ToolInfo` struct from ag-checker
- [x] 2.5 Add tests for `type_to_json_schema` covering: primitives, arrays, maps, structs, unions, nullable, non-serializable (fn → Any)

## 3. Update ag-codegen to use JsonSchema

- [x] 3.1 Rewrite `tool_schema.rs` to convert `JsonSchema → swc::Expr` instead of `Type → swc::Expr`
- [x] 3.2 Update `Translator` and `translate_item_into` to use `ToolSchemaInfo` instead of `ToolInfo`
- [x] 3.3 Update `codegen_with_tools` public API to accept `HashMap<String, ToolSchemaInfo>`
- [x] 3.4 Remove `ag-checker` from `ag-codegen/Cargo.toml` dependencies
- [x] 3.5 Update tool_schema tests to use `JsonSchema` input

## 4. Update ag-cli

- [x] 4.1 Update `cmd_build` to pass `checked.tool_registry` (now `HashMap<String, ToolSchemaInfo>`) to codegen

## 5. Verify

- [x] 5.1 Run `cargo build --workspace` — confirm clean compilation
- [x] 5.2 Run `cargo test --workspace` — confirm all tests pass
- [x] 5.3 Verify `ag-codegen/Cargo.toml` does not list `ag-checker` in [dependencies]
