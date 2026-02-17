## 1. Codegen API & Plumbing

- [ ] 1.1 Add `ag-checker` as a dependency of `ag-codegen` (for `Type` and `ToolInfo` types)
- [ ] 1.2 Change `codegen()` signature to accept `tool_registry: HashMap<String, ToolInfo>` parameter
- [ ] 1.3 Store tool registry in `Translator` struct, accessible during fn translation
- [ ] 1.4 Update `ag-cli` call site to pass `checked.tool_registry` to `codegen()`

## 2. Type-to-JSON-Schema Mapper

- [ ] 2.1 Create `ag-codegen/src/tool_schema.rs` module with `type_to_json_schema(ty: &Type) -> swc::Expr`
- [ ] 2.2 Implement primitive mapping: `str` → `"string"`, `num` → `"number"`, `int` → `"integer"`, `bool` → `"boolean"`
- [ ] 2.3 Implement array mapping: `[T]` → `{ type: "array", items: ... }`
- [ ] 2.4 Implement struct/object mapping: `Struct(fields)` → `{ type: "object", properties: ..., required: ... }`
- [ ] 2.5 Implement map mapping: `{str: V}` → `{ type: "object", additionalProperties: ... }`
- [ ] 2.6 Implement nullable mapping: `T?` → base type schema, excluded from required
- [ ] 2.7 Implement union mapping: `T | U` → `{ anyOf: [...] }`
- [ ] 2.8 Implement `any`/`unknown` → `{}` (empty schema)

## 3. Schema Emission in Function Translation

- [ ] 3.1 In `translate_fn_decl`, detect `tool_annotation` and look up tool registry for resolved types
- [ ] 3.2 Build full schema object: `{ name, description?, parameters: { type: "object", properties, required } }`
- [ ] 3.3 Determine `required` array by checking which AST params have no default value
- [ ] 3.4 Emit `fnName.schema = <schema>` as an `ExpressionStatement` after the function declaration

## 4. Tests

- [ ] 4.1 Unit tests for `type_to_json_schema`: primitives, arrays, objects, nullable, union, any
- [ ] 4.2 Integration tests: `@tool` fn with description emits correct schema
- [ ] 4.3 Integration tests: `@tool` fn without description omits description field
- [ ] 4.4 Integration tests: required vs optional parameters (defaults)
- [ ] 4.5 Integration tests: struct parameter expands to nested object schema
- [ ] 4.6 Verify non-tool functions are unaffected (no schema emitted)

## 5. Verification

- [ ] 5.1 Run `cargo test --workspace` — all tests pass
- [ ] 5.2 Compile `examples/simple-agent/app.ag` and verify schema output in generated JS
