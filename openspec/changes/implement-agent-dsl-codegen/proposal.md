## Why

The `@agent` DSL has full parsing infrastructure (`ag-dsl-agent`: lexer, parser, validator, AST) but cannot emit JavaScript — it has no `codegen.rs` or `handler.rs`. This blocks users from compiling any `.ag` file that uses `@agent` blocks. Additionally, all DSL crates duplicate SWC helper functions, and the `@tool` annotation still lacks JSON schema generation — both prerequisites for a working agent pipeline. The runtime target is AI SDK v6's `ToolLoopAgent`, wrapped in a thin `@agentscript/runtime` npm package.

## What Changes

- **Extract shared SWC helpers** into `ag-dsl-core` — `ident()`, `str_lit()`, `num_lit()`, `expr_or_spread()`, `make_prop()`, `emit_module()` — eliminating duplication across DSL crates
- **Implement `@tool` JSON schema generation** — emit `fnName.schema = { name, description, parameters }` after each `@tool` function, passing checker's `tool_registry` to codegen
- **Add `codegen.rs` and `handler.rs` to `ag-dsl-agent`** — generates `new AgentRuntime({...})` from `AgentTemplate` AST, translating sections/model/tools/skills/agents/output/constraints/hooks
- **Register `AgentDslHandler`** in `ag-codegen` for the `"agent"` DSL kind
- **Create `@agentscript/runtime` npm package** (`runtime/agent-runtime/`) — thin TypeScript wrapper around AI SDK v6 `ToolLoopAgent`, exporting `AgentRuntime` class with model resolution, tool wrapping, and lifecycle hooks
- **Update `implement-server-dsl-codegen`** design to use shared helpers from `ag-dsl-core` instead of duplicating them

## Capabilities

### New Capabilities
- `agent-dsl-codegen`: Code generation for `@agent` DSL blocks — emitting `AgentRuntime` constructor calls from parsed `AgentTemplate` AST, including message building, tool/skill/agent capture translation, output schema, constraints, and lifecycle hooks
- `dsl-swc-helpers`: Shared SWC AST helper functions in `ag-dsl-core` for all DSL codegen crates — `ident()`, `str_lit()`, `num_lit()`, `expr_or_spread()`, `make_prop()`, `emit_module()`

### Modified Capabilities
- `dsl-codegen-framework`: Handler registration extended to include `"agent"` kind alongside `"prompt"`
- `codegen-js`: Function declaration translation extended to emit `.schema` property assignment for `@tool` functions (tool-json-schema-gen)
- `prompt-dsl-codegen`: Helper functions migrated from private to shared (`ag-dsl-core`); codegen logic unchanged

## Impact

- **Crates modified**: `ag-dsl-core` (add swc_helpers module + swc_common dep), `ag-dsl-agent` (add codegen.rs, handler.rs), `ag-dsl-prompt` (remove local helpers, import from core), `ag-codegen` (register agent handler, add tool_schema.rs, change codegen() signature), `ag-cli` (pass tool_registry to codegen)
- **Crates added as deps**: `ag-dsl-agent` → `ag-codegen`, `ag-checker` → `ag-codegen` (for tool_registry types)
- **New npm package**: `runtime/agent-runtime/` — `@agentscript/runtime` with `ai@^6`, `zod`, `@ai-sdk/openai`, `@ai-sdk/anthropic` as dependencies
- **No breaking changes** to existing compiled output — purely additive
