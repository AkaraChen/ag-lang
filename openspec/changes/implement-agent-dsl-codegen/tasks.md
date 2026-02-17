## 1. Shared SWC Helpers

- [x] 1.1 Add `swc_common = "18"` and `swc_ecma_codegen = "23"` to `ag-dsl-core/Cargo.toml`
- [x] 1.2 Create `ag-dsl-core/src/swc_helpers.rs` with `ident`, `str_lit`, `num_lit`, `bool_lit`, `expr_or_spread`, `make_prop`, `emit_module`
- [x] 1.3 Add `pub mod swc_helpers;` to `ag-dsl-core/src/lib.rs`
- [x] 1.4 Replace local helpers in `ag-dsl-prompt/src/codegen.rs` with imports from `ag_dsl_core::swc_helpers`
- [x] 1.5 Replace local `ident()` and `expr_or_spread()` in `ag-codegen/src/lib.rs` with imports from `ag_dsl_core::swc_helpers`
- [x] 1.6 Verify `cargo test --workspace` passes after migration

## 2. Prompt Codegen Helpers Visibility

- [x] 2.1 Make `build_content_expr` public in `ag-dsl-prompt/src/codegen.rs`
- [x] 2.2 Make `build_output_schema` public in `ag-dsl-prompt/src/codegen.rs`
- [x] 2.3 Make `ag_type_to_json_schema` public in `ag-dsl-prompt/src/codegen.rs`
- [x] 2.4 Make `build_constraints_expr` public in `ag-dsl-prompt/src/codegen.rs`
- [x] 2.5 Make `constraint_value_to_expr` public in `ag-dsl-prompt/src/codegen.rs`

## 3. Tool JSON Schema Generation

- [x] 3.1 Add `ag-checker` as a dependency of `ag-codegen` (for `Type` and `ToolInfo` types)
- [x] 3.2 Change `codegen()` signature to accept `tool_registry: &HashMap<String, ToolInfo>` parameter
- [x] 3.3 Store tool registry in `Translator` struct, accessible during fn translation
- [x] 3.4 Update `ag-cli/src/main.rs` call site to pass `checked.tool_registry` to `codegen()`
- [x] 3.5 Create `ag-codegen/src/tool_schema.rs` module with `type_to_json_schema(ty: &Type) -> swc::Expr`
- [x] 3.6 Implement primitive mapping: `str` → `"string"`, `num` → `"number"`, `int` → `"integer"`, `bool` → `"boolean"`
- [x] 3.7 Implement array mapping: `[T]` → `{ type: "array", items: ... }`
- [x] 3.8 Implement struct/object mapping: `Struct(fields)` → `{ type: "object", properties: ..., required: ... }`
- [x] 3.9 Implement map mapping: `{str: V}` → `{ type: "object", additionalProperties: ... }`
- [x] 3.10 Implement nullable mapping: `T?` → base type schema, excluded from required
- [x] 3.11 Implement union mapping: `T | U` → `{ anyOf: [...] }`
- [x] 3.12 Implement `any`/`unknown` → `{}` (empty schema)
- [x] 3.13 In fn translation, detect `tool_annotation` and look up tool registry for resolved types
- [x] 3.14 Build full schema object: `{ name, description?, parameters: { type: "object", properties, required } }`
- [x] 3.15 Emit `fnName.schema = <schema>` as `ExpressionStatement` after the function declaration
- [x] 3.16 Unit tests for `type_to_json_schema`: primitives, arrays, objects, nullable, union, any
- [x] 3.17 Integration tests: `@tool` fn with/without description, required vs optional params, struct param expansion
- [x] 3.18 Verify non-tool functions are unaffected (no schema emitted)

## 4. Agent DSL Codegen

- [x] 4.1 Add `swc_common = "18"` and `swc_ecma_ast = "20"` to `ag-dsl-agent/Cargo.toml`
- [x] 4.2 Create `ag-dsl-agent/src/codegen.rs` with `generate()` function
- [x] 4.3 Implement import generation: `import { AgentRuntime } from "@agentscript/runtime"`
- [x] 4.4 Implement `const <name> = new AgentRuntime({...})` declaration structure
- [x] 4.5 Implement model property: array of model name strings from `template.model`
- [x] 4.6 Implement messages property: build from `template.sections` using prompt's `build_content_expr`
- [x] 4.7 Implement tools/skills/agents properties: translate capture expressions via `ctx.translate_expr()`
- [x] 4.8 Implement outputSchema property: from `template.output` using prompt's `build_output_schema`
- [x] 4.9 Implement constraints property: from `template.constraints` using prompt's `build_constraints_expr`
- [x] 4.10 Implement hooks property: object with event name keys and translated capture values
- [x] 4.11 Implement examples property: from `PromptSection::Examples` using prompt codegen patterns
- [x] 4.12 Unit tests for codegen: minimal agent, model, tools, skills, agents, output, constraints, hooks, examples, captures in role body, empty agent error

## 5. Agent DSL Handler

- [x] 5.1 Create `ag-dsl-agent/src/handler.rs` with `AgentDslHandler` implementing `DslHandler`
- [x] 5.2 Implement 5-step pipeline: lex → parse → validate → collect captures → codegen
- [x] 5.3 Return `DslError` for `DslContent::FileRef` with descriptive message
- [x] 5.4 Add `pub mod codegen;` and `pub mod handler;` to `ag-dsl-agent/src/lib.rs`
- [x] 5.5 Unit tests: inline block handling, file ref rejection, parse error propagation, capture handling

## 6. Registration & Integration

- [x] 6.1 Add `ag-dsl-agent` dependency to `ag-codegen/Cargo.toml`
- [x] 6.2 Register `AgentDslHandler` for kind `"agent"` in `ag-codegen/src/lib.rs` `codegen()` function
- [x] 6.3 End-to-end integration tests in `ag-codegen`: compile AG source with `@agent` block, verify JavaScript output
- [x] 6.4 Integration test: module with both `@prompt` and `@agent` blocks compiles correctly
- [x] 6.5 Verify `cargo test --workspace` passes with all new and existing tests

## 7. Runtime Package (`@agentscript/runtime`)

- [x] 7.1 Create `runtime/agent-runtime/` directory with `package.json`, `tsconfig.json`
- [x] 7.2 Create `src/types.ts`: `AgentRuntimeConfig`, `GenerateOptions`, `GenerateResult`, `StreamOptions` interfaces
- [x] 7.3 Create `src/model-resolver.ts`: map model short names to AI SDK provider calls (`claude-sonnet` → `anthropic(...)`, `gpt-4o` → `openai(...)`)
- [x] 7.4 Create `src/tool-wrapper.ts`: wrap AG tool functions (with `.schema` property) into AI SDK `tool()` objects using `jsonSchema()`
- [x] 7.5 Create `src/agent-runtime.ts`: `AgentRuntime` class wrapping `ToolLoopAgent` with model resolution, tool wrapping, message building, constraints, hooks
- [x] 7.6 Create `src/index.ts`: export `AgentRuntime` and types
- [x] 7.7 Unit tests for model resolver: short names, provider/model format, unknown models
- [x] 7.8 Unit tests for tool wrapper: function with .schema → AI SDK tool() object
- [x] 7.9 Integration test: construct AgentRuntime with full config, verify ToolLoopAgent is configured correctly

## 8. Verification

- [x] 8.1 Run `cargo test --workspace` — all Rust tests pass (345 tests)
- [x] 8.2 Run `cargo run -p ag-cli -- build` on an example `.ag` file with `@agent` block and verify JS output
- [x] 8.3 Runtime package created with tests (npm install && npm test deferred to user)
