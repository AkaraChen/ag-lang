## Why

The language specification (`spec/lang.md` v0.1) was written before implementation began. After implementing the core compiler (lexer, parser, type checker, codegen), several design decisions have diverged from the original spec. Key changes: prompt literals became a general DSL block system (`@kind name`), extern declarations and `@js` annotations were added for JS interop, and HTTP server uses imperative Hono API instead of declarative syntax. The spec needs to be updated to v0.2 to reflect actual implementation and formalize the design direction for not-yet-implemented features (agent, tool, skill, component, server).

## What Changes

- **BREAKING**: Replace §2.5 Prompt Literals (expression-level triple-backtick) with general DSL block system (`@kind name ``` ... ```)
- **BREAKING**: Replace §5 Agent declarations (keyword syntax) with `@agent` DSL blocks using `@directive` + `#{}` capture internal syntax
- **BREAKING**: Replace §7 Skill declarations (keyword syntax) with `@skill` DSL blocks
- **BREAKING**: Replace §8 Component declarations (keyword syntax) with `@component` DSL blocks
- **BREAKING**: Replace §6 Tool declarations (keyword `tool`) with `@tool` annotation on `fn` declarations
- **BREAKING**: Replace §9 HTTP Server declarative keyword syntax with `@server` DSL block
- Add new section: Extern declarations (`extern fn`, `extern struct`, `extern type`)
- Add new section: `@js("module")` annotation for JS interop
- Add new section: General DSL block framework (`@kind name ``` ... ```)
- Add new section: Annotation system (`@tool`, `@js`)
- Update §11 Standard Library to reflect Layer A (Web API externs) / Layer B (runtime-backed) architecture
- Update §12 Grammar to include DSL blocks, extern, annotations
- Update §13 Compilation to reflect DSL handler pipeline

## Capabilities

### New Capabilities
- `lang-spec-update`: Update `spec/lang.md` from v0.1 to v0.2, reflecting all design changes above. This is a documentation-only change — updating the language specification document.

### Modified Capabilities

(none — this change updates the standalone spec document, not OpenSpec requirement specs)

## Impact

- `spec/lang.md` — full rewrite of sections §2.5, §5, §6, §7, §8, §9, updates to §11, §12, §13, new sections for DSL system, extern, annotations
- No code changes — this is a spec document update only
