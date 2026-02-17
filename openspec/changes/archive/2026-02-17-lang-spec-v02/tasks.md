## 1. New Sections

- [x] 1.1 Add §2.5 "DSL Block System" — replace old Prompt Literals with general `@kind name ``` ... ``` ` syntax, `#{}` captures, `@directive` internal syntax, file refs
- [x] 1.2 Add new section "Extern Declarations" — `extern fn/struct/type`, variadic params, codegen erasure
- [x] 1.3 Add new section "Annotation System" — `@tool` on fn, `@js("module")` on extern, general `@name(args)` syntax

## 2. Rewritten Sections

- [x] 2.1 Rewrite §5 "Agent System" — `@agent` DSL block with `@model`, `@tools`, `@skills`, `@agents`, `@on` directives, body text as system prompt, agent composition
- [x] 2.2 Rewrite §6 "Tool Declarations" — `@tool` annotation on `fn`, auto JSON Schema from signature + doc comments
- [x] 2.3 Rewrite §7 "Skill Declarations" — `@skill` DSL block with `@description`, `@input`, `@steps` directives
- [x] 2.4 Rewrite §8 "Component Declarations" — `@component` DSL block with `@props`, `@state`, `@render`, `@style` directives
- [x] 2.5 Rewrite §9 "HTTP Server" — `@server` DSL block with `@port`, `@middleware`, route directives (`@get`, `@post`, etc.) and `#{}` handler captures

## 3. Updated Sections

- [x] 3.1 Update §11 "Standard Library" — Layer A (Web API externs, zero-cost) / Layer B (runtime-backed, @agentscript/stdlib) architecture
- [x] 3.2 Update §12 "Grammar" — add DSL block, extern, annotation, @tool productions; remove old agent/tool/skill/component/http keyword productions
- [x] 3.3 Update §13 "Compilation" — DSL handler pipeline, updated mapping table (DSL blocks → JS output), annotation processing
- [x] 3.4 Update §14 "Complete Example" — rewrite to use DSL blocks, @tool, extern, @server syntax
- [x] 3.5 Update §15 "Compiler Implementation Notes" — add DSL handler dispatch to lookahead table, annotation handling
- [x] 3.6 Update §2.1 "Keywords" — remove `agent`, `tool`, `skill`, `component`, `prompt`, `http`, `route` from reserved keywords; add `extern`; note that `@agent` etc. are DSL kind identifiers not keywords
- [x] 3.7 Update Appendix A "Keyword Comparison" — reflect removed/added keywords
