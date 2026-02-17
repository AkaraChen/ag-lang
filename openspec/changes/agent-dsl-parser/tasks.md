## 1. Crate setup

- [ ] 1.1 Create `crates/ag-dsl-agent/` crate with `Cargo.toml`
- [ ] 1.2 Add `ag-dsl-agent` to workspace `Cargo.toml` members
- [ ] 1.3 Add dependency on `ag-dsl-core` (for `DslPart`, `Span`, `DslError`)
- [ ] 1.4 Add dependency on `ag-dsl-prompt` (for `PromptToken`, `PromptSection`, `ModelSpec`, `OutputSpec`, `Constraints`, `PromptPart`, `RoleName`, `Example`)
- [ ] 1.5 Create `src/lib.rs` exposing `pub mod ast`, `pub mod lexer`, `pub mod parser`, `pub mod validator`
- [ ] 1.6 `cargo build -p ag-dsl-agent` passes

## 2. AST definitions

- [ ] 2.1 Define `AgentTemplate { name, sections: Vec<PromptSection>, model: Option<ModelSpec>, output: Option<OutputSpec>, constraints: Option<Constraints>, tools_capture: Option<usize>, skills_capture: Option<usize>, agents_capture: Option<usize>, on_hooks: Vec<OnHook> }`
- [ ] 2.2 Define `OnHook { event: String, capture_index: usize }`
- [ ] 2.3 Re-export prompt AST types used by `AgentTemplate` (`PromptSection`, `ModelSpec`, `OutputSpec`, `Constraints`, `PromptPart`, `RoleName`, `Example`, `OutputKind`, `OutputField`, `ConstraintValue`)

## 3. Lexer

- [ ] 3.1 Define `AgentToken` enum: `DirectiveTools`, `DirectiveSkills`, `DirectiveAgents`, `DirectiveOn(String)`, `Prompt(PromptToken)`
- [ ] 3.2 Implement `lex(parts: &[DslPart]) -> Vec<AgentToken>` entry point
- [ ] 3.3 Implement agent directive recognition in `DslPart::Text` segments: line-initial `@tools`, `@skills`, `@agents`, `@on` produce agent-specific tokens
- [ ] 3.4 Implement `@on <event>` parsing: extract event name identifier from same line after `@on`
- [ ] 3.5 Delegate all non-agent directives (`@role`, `@model`, `@examples`, `@output`, `@constraints`, `@messages`) and non-directive text to prompt lexer behavior, wrapping results as `Prompt(...)`
- [ ] 3.6 Implement `DslPart::Capture` pass-through as `Prompt(Capture(index))`
- [ ] 3.7 Write lexer unit tests: @tools, @skills, @agents, @on directives with captures
- [ ] 3.8 Write lexer unit tests: prompt directives passed through correctly (@role, @model, @constraints, @examples)
- [ ] 3.9 Write lexer unit tests: unknown @ treated as text, @ mid-line treated as text

## 4. Parser

- [ ] 4.1 Implement `parse(name: &str, tokens: &[AgentToken]) -> Result<AgentTemplate, Vec<Diagnostic>>` entry point
- [ ] 4.2 Implement state machine main loop: dispatch on `DirectiveTools`/`DirectiveSkills`/`DirectiveAgents`/`DirectiveOn` for agent-specific handling, and on `Prompt(...)` tokens for prompt-style handling
- [ ] 4.3 Implement `parse_capture_directive()`: expect `Prompt(Capture(idx))` after tools/skills/agents directives, error if missing
- [ ] 4.4 Implement `parse_on_hook()`: expect `Prompt(Capture(idx))` after `DirectiveOn(event)`, create `OnHook { event, capture_index }`
- [ ] 4.5 Implement prompt directive delegation: `Prompt(DirectiveRole(...))` → collect body, `Prompt(DirectiveModel)` → parse model list, `Prompt(DirectiveExamples)` → parse examples block, `Prompt(DirectiveConstraints)` → parse constraints block, `Prompt(DirectiveOutput)` → parse output, `Prompt(DirectiveMessages)` → parse messages capture
- [ ] 4.6 Implement default system role: text/captures before first directive assigned to implicit `@role system`
- [ ] 4.7 Implement error handling: missing captures after directives, empty agent, examples without `{`
- [ ] 4.8 Write parser unit tests: agent with @tools capture
- [ ] 4.9 Write parser unit tests: agent with @on hooks
- [ ] 4.10 Write parser unit tests: agent with mixed prompt and agent directives
- [ ] 4.11 Write parser unit tests: full agent template (all directive types)
- [ ] 4.12 Write parser unit tests: error cases (missing capture, empty agent)

## 5. Validator

- [ ] 5.1 Implement `validate(template: &AgentTemplate) -> Vec<Diagnostic>` entry point
- [ ] 5.2 Check duplicate `@model` (error)
- [ ] 5.3 Check duplicate `@tools` (error)
- [ ] 5.4 Check duplicate `@skills` (error)
- [ ] 5.5 Check duplicate `@agents` (error)
- [ ] 5.6 Check duplicate `@output` (error)
- [ ] 5.7 Check duplicate `@constraints` (error)
- [ ] 5.8 Check duplicate `@on` with same event name (error)
- [ ] 5.9 Check `@on` with unknown event name — warn "unknown event '<name>'; known events are: init, message, error"
- [ ] 5.10 Check no `@role` and no body text — warn "no @role directive; content assigned to implicit system role"
- [ ] 5.11 Write validator unit tests: duplicate directives produce errors
- [ ] 5.12 Write validator unit tests: unknown @on event produces warning
- [ ] 5.13 Write validator unit tests: valid agent passes without errors

## 6. Integration tests

- [ ] 6.1 Write end-to-end test: `Vec<DslPart>` input -> lex -> parse -> validate for simple agent
- [ ] 6.2 Write end-to-end test: full agent with @model, @tools, @skills, @on, @role, @constraints, @examples
- [ ] 6.3 Write end-to-end test: agent with agent composition (@agents directive)
- [ ] 6.4 Write end-to-end test: error scenarios (duplicate directives, missing captures)
