## Context

The `@agent` DSL has full parsing in `ag-dsl-agent` (lexer, parser, validator) producing an `AgentTemplate` AST that reuses `PromptSection`, `ModelSpec`, `OutputSpec`, `Constraints` from `ag-dsl-prompt::ast`, plus agent-specific fields: `tools_capture`, `skills_capture`, `agents_capture`, and `on_hooks: Vec<OnHook>`. The `@prompt` DSL is the only DSL with working codegen. The `@tool` annotation is parsed/checked but codegen emits no schema.

The language spec (§6.4, §14.3-14.4) defines the compilation target:
- `@agent` → `import { AgentRuntime } from "@agentscript/runtime"` + `new AgentRuntime({...})`
- `@tool fn` → `function fn(...) {...}` + `fn.schema = { name, description, parameters }`

The runtime target is AI SDK v6 (`ToolLoopAgent`), wrapped in a thin `@agentscript/runtime` npm package.

Three cross-cutting concerns were identified during exploration:
1. SWC helper duplication across DSL crates (`ident`, `str_lit`, `make_prop`, etc.)
2. Tool JSON schema generation (prerequisite for agent tools to work)
3. The `implement-server-dsl-codegen` change also duplicates helpers — needs updating

## Goals / Non-Goals

**Goals:**
- Extract shared SWC helper functions into `ag-dsl-core` for all DSL codegen crates
- Emit `fnName.schema = {...}` for every `@tool`-annotated function (reusing checker's resolved `Type`)
- Emit correct `AgentRuntime` constructor calls from `@agent` DSL blocks
- Create `@agentscript/runtime` npm package wrapping AI SDK v6 `ToolLoopAgent`
- Follow the same 5-step handler pattern as `ag-dsl-prompt`
- Full test coverage

**Non-Goals:**
- `@skill` / `@server` / `@component` codegen (separate changes)
- Streaming agent support in runtime (can be added later)
- Doc comment extraction for per-parameter tool descriptions
- Type-checking agent directive captures (future checker work)
- Migrating `@prompt` to AI SDK v6 (PromptTemplate stays independent)

## Decisions

### 1. Extract shared SWC helpers to `ag-dsl-core::swc_helpers`

New module `ag-dsl-core/src/swc_helpers.rs` with:
```rust
pub fn ident(name: &str) -> swc::Ident
pub fn str_lit(s: &str) -> swc::Expr
pub fn num_lit(n: f64) -> swc::Expr
pub fn bool_lit(b: bool) -> swc::Expr
pub fn expr_or_spread(expr: swc::Expr) -> swc::ExprOrSpread
pub fn make_prop(key: &str, value: swc::Expr) -> swc::PropOrSpread
pub fn emit_module(items: &[swc::ModuleItem]) -> String
```

Requires adding `swc_common = "18"` and `swc_ecma_codegen = "23"` to `ag-dsl-core/Cargo.toml`.

**Rationale:** `ag-dsl-core` already depends on `swc_ecma_ast` and is depended on by every DSL crate. Adding helpers here eliminates triplication (currently in `ag-codegen` and `ag-dsl-prompt`, and would be needed by `ag-dsl-agent`, `ag-dsl-server`, etc.). The `implement-server-dsl-codegen` design explicitly plans to duplicate these — this avoids that.

**Alternatives considered:**
- New `ag-dsl-shared` crate: adds a crate just for 7 functions — excessive indirection
- Make prompt helpers `pub`: couples all DSL crates to `ag-dsl-prompt`, wrong dependency direction
- Keep duplicating: accumulates tech debt with every new DSL

**Migration:** `ag-dsl-prompt/src/codegen.rs` deletes its local helpers and imports from `ag_dsl_core::swc_helpers`. `ag-codegen/src/lib.rs` does the same for its `ident()` and `expr_or_spread()`.

### 2. Tool JSON schema: pass checker's `ToolInfo` to codegen

The codegen entry point changes from `codegen(module: &Module) -> String` to `codegen(module: &Module, tool_registry: &HashMap<String, ToolInfo>) -> String`. The `ToolInfo` contains resolved `Type` enums per parameter.

New file `ag-codegen/src/tool_schema.rs` with `type_to_json_schema(ty: &Type) -> swc::Expr` mapping:

| AG Type | JSON Schema |
|---------|-------------|
| `str` | `{ "type": "string" }` |
| `num` | `{ "type": "number" }` |
| `int` | `{ "type": "integer" }` |
| `bool` | `{ "type": "boolean" }` |
| `[T]` | `{ "type": "array", "items": <T> }` |
| `{str: V}` | `{ "type": "object", "additionalProperties": <V> }` |
| `Struct(fields)` | `{ "type": "object", "properties": {...}, "required": [...] }` |
| `T?` | base type schema; excluded from `required` |
| `T \| U` | `{ "anyOf": [...] }` |
| `any` | `{}` |

Schema emitted as assignment statement: `fnName.schema = { name: "fnName", description: "...", parameters: {...} };`

**Rationale:** Aligns with existing `tool-json-schema-gen` change design. Using checker `Type` avoids re-resolving struct fields in codegen.

### 3. Agent codegen generates `AgentRuntime` constructor

Target output (from spec §6.4):
```javascript
import { AgentRuntime } from "@agentscript/runtime";
const Coder = new AgentRuntime({
  model: ["claude-sonnet", "gpt-4o"],
  tools: [read_file, write_file],
  messages: [{ role: "system", content: "You are an expert..." }],
  outputSchema: { type: "object", properties: {...}, required: [...] },
  constraints: { temperature: 0.3, max_tokens: 4096 },
  hooks: { init: initHandler, error: errorHandler }
});
```

`AgentTemplate` AST field → JS property mapping:

| AST Field | JS Property | Generation Logic |
|---|---|---|
| `sections: Vec<PromptSection>` | `messages: [...]` | Reuse prompt's `build_content_expr` for role bodies |
| `model: Option<ModelSpec>` | `model: [...]` | Array of model name strings |
| `tools_capture: Option<usize>` | `tools: <expr>` | `ctx.translate_expr(captures[idx])` |
| `skills_capture: Option<usize>` | `skills: <expr>` | Same |
| `agents_capture: Option<usize>` | `agents: <expr>` | Same |
| `output: Option<OutputSpec>` | `outputSchema: {...}` | Reuse prompt's `build_output_schema` |
| `constraints: Option<Constraints>` | `constraints: {...}` | Reuse prompt's `build_constraints_expr` |
| `on_hooks: Vec<OnHook>` | `hooks: { event: <expr> }` | Object with event keys, capture values |

**Code reuse:** `build_content_expr`, `build_output_schema`, `build_constraints_expr` stay in `ag-dsl-prompt::codegen` as `pub` functions (they use prompt-specific types like `PromptPart`). The agent codegen imports and calls them since `ag-dsl-agent` already depends on `ag-dsl-prompt`.

### 4. Thin `@agentscript/runtime` wrapper around AI SDK v6

Package at `runtime/agent-runtime/`, published as `@agentscript/runtime`. TypeScript.

```typescript
export class AgentRuntime {
  constructor(config: AgentRuntimeConfig) {
    // Internally creates ToolLoopAgent from AI SDK v6
  }
  async generate(opts: GenerateOptions): Promise<GenerateResult>
  stream(opts: StreamOptions): StreamResult
}
```

Key responsibilities:
- **Model resolution**: `"claude-sonnet"` → `anthropic('claude-sonnet-4-20250514')`, `"gpt-4o"` → `openai('gpt-4o')`. First model in array is primary, rest are fallback.
- **Tool wrapping**: Accepts AG tool functions (with `.schema` property) and wraps them as AI SDK `tool()` objects with `inputSchema` derived from `.schema.parameters` via `jsonSchema()`.
- **Message building**: Passes `messages` array to `ToolLoopAgent.instructions` (system messages concatenated).
- **Constraints mapping**: `temperature`, `max_tokens` → ToolLoopAgent config.
- **Hooks**: `init` called in constructor, `message`/`error` mapped to `onStepFinish`/error handling.
- **Output schema**: JSON Schema from `outputSchema` → AI SDK `Output.object()` via `jsonSchema()`.

**Rationale:** Thin wrapper insulates generated code from AI SDK v6 beta API changes. Matches spec's import path `@agentscript/runtime`. Uses `jsonSchema()` from `ai` to accept JSON Schema directly (avoids Zod dependency for schema conversion).

**Dependencies:**
```json
{
  "ai": "^6.0.0",
  "@ai-sdk/openai": "^2.0.0",
  "@ai-sdk/anthropic": "^2.0.0"
}
```

### 5. Agent handler follows 5-step pipeline

`AgentDslHandler` in `ag-dsl-agent/src/handler.rs` mirrors `PromptDslHandler` exactly:
1. Lex `DslPart[]` → `AgentToken[]`
2. Parse → `AgentTemplate`
3. Validate (non-fatal warnings)
4. Collect captures from parts
5. `codegen::generate(template, captures, ctx)` → `Vec<ModuleItem>`

`FileRef` returns `DslError` (not meaningful for agents).

### 6. Prompt codegen helpers made `pub` for reuse

The following functions in `ag-dsl-prompt/src/codegen.rs` change from `fn` to `pub fn`:
- `build_content_expr` (line 290) — builds string/template content from `PromptPart` body
- `build_output_schema` (line 404) — builds JSON Schema object from `OutputSpec`
- `ag_type_to_json_schema` (line 452) — maps type string → JSON Schema SWC expression
- `build_constraints_expr` (line 477) — builds constraints object from `Constraints`
- `constraint_value_to_expr` (line 490) — converts `ConstraintValue` to SWC expression

These stay in `ag-dsl-prompt` because they depend on prompt-specific AST types (`PromptPart`, `OutputSpec`, etc.). The low-level SWC helpers (`ident`, `str_lit`, etc.) move to `ag-dsl-core`.

## Risks / Trade-offs

- **[Risk] AI SDK v6 is beta; API may change** → Mitigation: Runtime wrapper absorbs changes; codegen output stays stable. Only wrapper TypeScript needs updating.
- **[Risk] `ag-codegen` gains dependency on `ag-checker`** for `ToolInfo`/`Type` types → Mitigation: Only types needed, not logic. Single call site in `ag-cli`. Consider extracting `Type` + `ToolInfo` to a shared types crate later if coupling becomes problematic.
- **[Risk] `swc_common` + `swc_ecma_codegen` added to `ag-dsl-core`** increases its dependency surface → Mitigation: These are already used by every DSL crate and `ag-codegen`; no new transitive deps. The `emit_module` helper is `#[cfg(test)]`-gated in most contexts anyway.
- **[Risk] Capture index correctness for tools/skills/agents/hooks** → Mitigation: Comprehensive tests verifying capture indices match between parser and codegen.
- **[Risk] JSON Schema → AI SDK tool input** — AI SDK v6 tools normally use Zod schemas (`inputSchema`), not JSON Schema → Mitigation: AI SDK v6 provides `jsonSchema()` adapter that accepts raw JSON Schema objects. The runtime uses this to wrap `.schema.parameters`.
