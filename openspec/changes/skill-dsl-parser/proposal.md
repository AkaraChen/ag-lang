## Why

Skill declarations use `@skill` DSL blocks with `@description`, `@input`, `@steps`, `@output` directives. We need an `ag-dsl-skill` crate to parse these into a structured `SkillTemplate` AST.

## What Changes

- New `ag-dsl-skill` crate with lexer, parser, validator
- Lexer recognizes: `@description`, `@input`, `@steps`, `@output`
- Parser produces `SkillTemplate` AST with: description string, input schema (typed fields with defaults), steps list, output schema
- `@input` and `@output` parse field declarations: `name: type (= default)?`
- `@steps` captures ordered step descriptions with `#{}` captures
- Validator checks required directives present, no duplicates
- No codegen

## Capabilities

### New Capabilities

- `skill-dsl-parser` — Lexer, parser, and validator for @skill DSL blocks

### Modified Capabilities

None.

## Impact

- New crate `crates/ag-dsl-skill`
- Update `Cargo.toml` workspace members
