## Why

Agent declarations use `@agent` DSL blocks. The DSL framework already captures the raw content with `DslPart::Text` and `DslPart::Capture`. Now we need an `ag-dsl-agent` crate that parses the agent-specific directives (`@model`, `@tools`, `@skills`, `@agents`, `@on`) and extracts all metadata into a structured `AgentTemplate` AST. This builds on the same pattern as `ag-dsl-prompt`.

## What Changes

- New `ag-dsl-agent` crate with lexer, parser, validator
- Lexer recognizes agent-specific directives: `@model`, `@tools`, `@skills`, `@agents`, `@on`
- Parser produces `AgentTemplate` AST with: model spec, tool list, skill list, sub-agent list, lifecycle hooks, and embedded prompt sections (inheriting from prompt DSL)
- Validator checks for duplicates, missing required fields
- No codegen — parser only

## Capabilities

### New Capabilities

- `agent-dsl-parser` — Lexer, parser, and validator for @agent DSL blocks

### Modified Capabilities

None.

## Impact

- New crate `crates/ag-dsl-agent`
- Update `Cargo.toml` workspace members
