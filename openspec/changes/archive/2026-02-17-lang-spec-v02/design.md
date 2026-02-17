## Context

The AgentScript language spec (`spec/lang.md` v0.1) was designed before implementation. After building the core compiler pipeline (lexer → parser → type checker → codegen), the implementation introduced:

1. A general-purpose **DSL block system** (`@kind name ``` ... ```) instead of keyword-per-construct syntax
2. **Extern declarations** (`extern fn/struct/type`) + **`@js` annotations** for JavaScript interop
3. An **annotation system** (`@js`, and now `@tool`) for decorating declarations
4. A **layered stdlib architecture** (Layer A: zero-cost Web API externs, Layer B: runtime-backed modules)
5. Imperative HTTP server via Hono instead of declarative `http server { ... }` syntax

The spec needs a v0.2 update to reflect these realities and formalize design direction for unimplemented features.

## Goals / Non-Goals

**Goals:**
- Update `spec/lang.md` to accurately reflect implemented features (DSL blocks, extern, @js, stdlib layers)
- Formalize the design for unimplemented features (agent, tool, skill, component, server) using the DSL block / annotation paradigm
- Keep the spec as the single source of truth for language design

**Non-Goals:**
- No code changes — this is a documentation-only update
- Not rewriting the grammar from scratch — update incrementally
- Not specifying implementation details (those live in OpenSpec specs)

## Decisions

### Decision 1: Three-tier construct system

All language constructs now fall into three categories:

| Tier | Mechanism | Used For |
|------|-----------|----------|
| DSL Blocks | `@kind name ``` ... ``` ` | Configuration-first constructs: prompt, agent, skill, component, server |
| Annotations | `@attr` on declarations | Metadata on code constructs: `@tool` on fn, `@js` on extern |
| Language Primitives | Keywords | Code-first constructs: fn, struct, enum, type, extern, let/mut/const, control flow |

**Rationale**: DSL blocks suit constructs that are primarily configuration/text (prompts, agent config, skill steps). Annotations suit constructs that ARE code but need metadata (tools are functions with schema). Language primitives suit core programming constructs.

### Decision 2: Agent as extended prompt DSL

`@agent` DSL handler extends the `@prompt` handler, adding agent-specific directives (`@tools`, `@model`, `@skills`, `@agents`, `@on`). The body text of an `@agent` block IS the system prompt.

**Rationale**: An agent's core identity IS its prompt. Tools, model, and constraints are metadata around that prompt. This avoids a separate `prompt =` field inside agents.

**Alternative considered**: Keep `agent` as keyword with DSL prompt reference. Rejected because it splits agent definition across two places.

### Decision 3: Tool as `@tool` annotation on fn

Tools are functions with metadata for LLM tool-calling. The `@tool` annotation marks a `fn` declaration as a tool. The compiler extracts JSON Schema from the function signature + doc comments.

**Rationale**: Tools have executable function bodies — they're fundamentally code, not configuration. An annotation preserves the function as a first-class citizen while adding the tool-calling metadata layer.

**Alternative considered**: `@tool` DSL block. Rejected because DSL blocks don't naturally contain executable code (captures are expressions only).

### Decision 4: HTTP server as `@server` DSL block

Use `@server` DSL block instead of the original `http server { ... }` keyword syntax or a macro system.

**Rationale**: Keeps all domain-specific constructs in the unified DSL system. Route handlers will use `#{}` captures. This requires future DSL enhancement to support statement block captures, but maintains architectural consistency.

**Alternative considered**: Keep `http server` as language primitive with compiler desugaring. Rejected in favor of DSL uniformity — one system for all domain constructs.

### Decision 5: DSL capture extension (future)

The DSL capture system currently supports expression captures only (`#{expr}`). For `@server` and `@skill` to work fully, statement block captures (`#{ ... statements ... }`) will be needed. This is a future enhancement to the DSL framework.

### Decision 6: Extern + @js as the JS interop layer

Extern declarations (`extern fn/struct/type`) declare JavaScript bindings with no AG implementation. `@js("module")` specifies which JS module to import from. This replaces the original spec's implicit JS interop.

**Rationale**: Explicit is better than implicit. The AG compiler needs to know exactly what comes from JS to type-check and generate correct imports.

## Risks / Trade-offs

- **[DSL statement captures not yet implemented]** → `@server` and `@skill` with complex handlers will need this. Mitigation: spec describes the target design; implementation can stage the DSL enhancement separately.
- **[Removing keyword syntax is breaking]** → `agent`, `skill`, `component` change from keywords to DSL block identifiers. Mitigation: spec is v0.2, no backward compatibility needed at this stage.
- **[Agent as prompt limits lifecycle hooks]** → `@on message(msg)` in DSL blocks with handler code needs statement captures. Mitigation: agents can reference external handler functions via `#{}` captures until statement captures land.
