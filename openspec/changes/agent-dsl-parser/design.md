## Context

The `@agent` DSL block extends the `@prompt` DSL. The prompt DSL (`ag-dsl-prompt`) already provides a complete pipeline: lexer (accepts `Vec<DslPart>`, produces `PromptToken`s), parser (tokens to `PromptTemplate` AST), validator, codegen, and handler. The prompt lexer recognizes `@role`, `@model`, `@examples`, `@output`, `@constraints`, `@messages` directives.

Agents need all the prompt directives plus agent-specific ones: `@tools`, `@skills`, `@agents` (each followed by a `#{}` capture referencing an AG array expression), and `@on <event>` (followed by a `#{}` capture referencing a handler function). The body text of an `@agent` block IS the system prompt.

This change creates `ag-dsl-agent` with lexer, parser, and validator only. No codegen.

## Goals / Non-Goals

**Goals:**
- New crate `ag-dsl-agent` following the same structure as `ag-dsl-prompt`
- Agent lexer extends the prompt lexer's directive set with `@tools`, `@skills`, `@agents`, `@on`
- Parser produces `AgentTemplate` AST
- Validator checks for duplicates and required fields
- Fully independent testing (no `.ag` files needed)

**Non-Goals:**
- No codegen (that is a separate change)
- No DslHandler implementation (requires codegen)
- No runtime design (separate concern)

## Decisions

### 1. Crate structure

```
crates/
└── ag-dsl-agent/
    ├── src/
    │   ├── lib.rs            # public API
    │   ├── ast.rs            # AgentTemplate AST definitions
    │   ├── lexer.rs          # agent token types + lexer
    │   ├── parser.rs         # agent parser producing AgentTemplate
    │   └── validator.rs      # structural validation
    └── tests/
        ├── lexer_tests.rs
        ├── parser_tests.rs
        └── validator_tests.rs
```

**Dependencies:**

```
ag-dsl-agent → ag-dsl-core    (for DslPart, Span, DslError)
ag-dsl-agent → ag-dsl-prompt  (reuse PromptSection, ModelSpec, Constraints, etc.)
```

`ag-dsl-agent` does NOT depend on `ag-lexer`/`ag-parser`/`ag-ast` or SWC crates (no codegen).

### 2. Agent-specific directives and token types

The agent lexer extends the prompt lexer's known directive set. When scanning `DslPart::Text` segments, line-initial `@` followed by a known keyword produces a directive token. The agent lexer recognizes all prompt directives plus:

| Directive | Token | Followed by |
|-----------|-------|-------------|
| `@tools` | `DirectiveTools` | `#{}` capture (next `DslPart::Capture`) |
| `@skills` | `DirectiveSkills` | `#{}` capture (next `DslPart::Capture`) |
| `@agents` | `DirectiveAgents` | `#{}` capture (next `DslPart::Capture`) |
| `@on <event>` | `DirectiveOn(String)` | `#{}` capture (next `DslPart::Capture`) |

The `AgentToken` enum wraps `PromptToken` variants and adds agent-specific variants:

```rust
enum AgentToken {
    // Agent-specific directives
    DirectiveTools,
    DirectiveSkills,
    DirectiveAgents,
    DirectiveOn(String),      // @on init, @on message, @on error

    // All prompt tokens passed through
    Prompt(PromptToken),      // wraps DirectiveRole, DirectiveModel, Text, Capture, etc.
}
```

**Rationale**: Wrapping `PromptToken` avoids duplicating all prompt token variants. The agent lexer first checks for agent-specific directives, then delegates to prompt lexer behavior for everything else.

### 3. `@tools`, `@skills`, `@agents` — capture directives

These three directives all follow the same pattern: the directive keyword appears in a `DslPart::Text` segment, and the next `DslPart` in the input is a `Capture` containing the AG array expression (e.g., `[read_file, write_file]`).

```
@tools #{[read_file, write_file]}
```

The DSL framework has already extracted `#{[read_file, write_file]}` as a `DslPart::Capture`. So the agent lexer sees:
1. `DslPart::Text("@tools ")` -> `DirectiveTools`
2. `DslPart::Capture(expr)` -> `Prompt(Capture(0))`

The parser then expects a `Capture` token after `DirectiveTools`/`DirectiveSkills`/`DirectiveAgents`.

### 4. `@on <event>` — event hook directive

The `@on` directive is followed by an event name identifier on the same line, then a `#{}` capture for the handler function:

```
@on init #{fn(ctx) { ... }}
@on message #{fn(msg) -> AgentResponse { ... }}
@on error #{fn(e: Error) { ... }}
```

The lexer extracts:
1. `DslPart::Text("@on init ")` -> `DirectiveOn("init")`
2. `DslPart::Capture(handler_expr)` -> `Prompt(Capture(n))`

Known event names: `init`, `message`, `error`. The validator warns on unknown event names but does not reject them (forward compatibility).

### 5. AgentTemplate AST

```rust
/// A complete agent template
struct AgentTemplate {
    name: String,
    // Inherited from prompt
    sections: Vec<PromptSection>,       // reused from ag-dsl-prompt
    model: Option<ModelSpec>,           // reused from ag-dsl-prompt
    output: Option<OutputSpec>,         // reused from ag-dsl-prompt
    constraints: Option<Constraints>,   // reused from ag-dsl-prompt
    // Agent-specific
    tools_capture: Option<usize>,       // capture index for @tools #{...}
    skills_capture: Option<usize>,      // capture index for @skills #{...}
    agents_capture: Option<usize>,      // capture index for @agents #{...}
    on_hooks: Vec<OnHook>,             // lifecycle hooks
}

struct OnHook {
    event: String,           // "init", "message", "error", etc.
    capture_index: usize,    // capture index for handler fn
}
```

**Key insight**: `AgentTemplate` directly reuses `PromptSection`, `ModelSpec`, `OutputSpec`, `Constraints`, `PromptPart`, `RoleName`, and `Example` types from `ag-dsl-prompt::ast`. This avoids duplication and ensures compatibility.

### 6. Parser strategy

The agent parser follows the same pattern as the prompt parser: a state machine that consumes tokens sequentially, dispatching to sub-parsers on each directive.

```
初始状态 → collect body text/captures as implicit system role
@role X  → enter role section (delegate to prompt-style parsing)
@model   → parse model list (delegate to prompt-style parsing)
@tools   → expect Capture token, store as tools_capture
@skills  → expect Capture token, store as skills_capture
@agents  → expect Capture token, store as agents_capture
@on X    → expect Capture token, create OnHook { event: X, capture }
@examples → delegate to prompt-style examples parsing
@output  → delegate to prompt-style output parsing
@constraints → delegate to prompt-style constraints parsing
@messages → delegate to prompt-style messages parsing
```

Text outside any directive (before the first directive, or after `@role`) becomes prompt content (role sections), just like in the prompt parser.

### 7. Validator rules

| Check | Severity | Message |
|-------|----------|---------|
| No `@role` and no body text | Warning | "no @role directive; content assigned to implicit system role" |
| Duplicate `@model` | Error | "duplicate @model directive" |
| Duplicate `@tools` | Error | "duplicate @tools directive" |
| Duplicate `@skills` | Error | "duplicate @skills directive" |
| Duplicate `@agents` | Error | "duplicate @agents directive" |
| Duplicate `@output` | Error | "duplicate @output directive" |
| Duplicate `@constraints` | Error | "duplicate @constraints directive" |
| Duplicate `@on` with same event | Error | "duplicate @on <event> hook" |
| `@on` with unknown event name | Warning | "unknown event '<name>'; known events are: init, message, error" |
| `@tools`/`@skills`/`@agents` without capture | Error | "expected capture expression after @tools" |
| `@on` without capture | Error | "expected capture expression after @on <event>" |

### 8. Independent testing strategy

```rust
#[test]
fn test_agent_lex_and_parse() {
    let parts = vec![
        DslPart::Text("@model claude-sonnet\n@tools ".into(), Span::dummy()),
        DslPart::Capture(Box::new(0u32), Span::dummy()),
        DslPart::Text("\n@role system\nYou are an expert coder.\n".into(), Span::dummy()),
    ];

    let tokens = agent_lexer::lex(&parts);
    let ast = agent_parser::parse("Coder", &tokens).unwrap();

    assert!(ast.model.is_some());
    assert!(ast.tools_capture.is_some());
    assert_eq!(ast.sections.len(), 1);
}
```

No `.ag` files, no host compiler, no codegen needed.

## Risks / Trade-offs

- **Dependency on ag-dsl-prompt types**: AgentTemplate directly reuses prompt AST types. If prompt types change, agent crate needs updating. Acceptable because the types are stable and semantic coupling is intentional (agents extend prompts).
- **No codegen means incomplete pipeline**: The agent DslHandler cannot be registered until codegen is implemented. This is by design -- parser-only scope keeps the change small and testable.
- **Wrapping PromptToken adds indirection**: `AgentToken::Prompt(PromptToken::Text(...))` is verbose. Acceptable for correctness; avoids maintaining two copies of prompt token definitions.
- **Unknown @on events are warnings not errors**: This allows forward compatibility (new events can be added without breaking existing agent definitions) but means typos in event names are not caught as errors.
