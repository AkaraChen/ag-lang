## ADDED Requirements

### Requirement: spec/lang.md §2.5 — DSL Block System replaces Prompt Literals

The spec SHALL replace §2.5 "Prompt Literals" with a "DSL Block System" section. The new section SHALL describe the general `@kind name ``` ... ``` ` syntax, `#{}` expression captures, `@directive` internal syntax, and file references (`@kind name from "path"`). It SHALL note that DSL handlers are extensible — any `@kind` identifier is valid.

#### Scenario: DSL block syntax shown

- **WHEN** a reader looks at §2.5
- **THEN** the primary example SHALL be `@prompt name ``` ... ``` ` instead of `let system = ``` ... ``` `

#### Scenario: General DSL framework described

- **WHEN** a reader looks at §2.5
- **THEN** it SHALL explain that `@kind` is not limited to `prompt` — the system supports `@agent`, `@skill`, `@component`, `@server`, and any future DSL kind

#### Scenario: Capture syntax documented

- **WHEN** a reader looks at §2.5
- **THEN** it SHALL document `#{expr}` for expression interpolation and note that statement block captures are planned for future

### Requirement: spec/lang.md new section — Annotation System

The spec SHALL add a new section describing the annotation system. Annotations use `@name` or `@name(args)` syntax before declarations. The section SHALL document `@tool` (on `fn`) and `@js("module")` (on `extern`) as the two built-in annotations.

#### Scenario: @tool annotation documented

- **WHEN** a reader looks at the Annotation section
- **THEN** it SHALL show `@tool` on a `fn` declaration with doc comments, and explain that the compiler auto-generates JSON Schema from the signature

#### Scenario: @js annotation documented

- **WHEN** a reader looks at the Annotation section
- **THEN** it SHALL show `@js("module")` and `@js("module", name = "jsName")` on extern declarations

### Requirement: spec/lang.md new section — Extern Declarations

The spec SHALL add a section for extern declarations: `extern fn`, `extern struct` (with fields and method signatures), and `extern type` (opaque). It SHALL explain that externs declare JavaScript bindings with no AG body, and that `@js` annotations specify the JS module source.

#### Scenario: extern fn shown

- **WHEN** a reader looks at the Extern section
- **THEN** it SHALL show `extern fn fetch(url: str) -> Promise<Response>` and `@js("node:fs/promises") extern fn readFile(...)` examples

#### Scenario: extern struct with methods shown

- **WHEN** a reader looks at the Extern section
- **THEN** it SHALL show an extern struct with both fields and method signatures (no body)

#### Scenario: extern type shown

- **WHEN** a reader looks at the Extern section
- **THEN** it SHALL show `extern type Headers` as an opaque type declaration

### Requirement: spec/lang.md §5 — Agent as DSL Block

The spec SHALL replace §5 "Agent System" to use `@agent` DSL block syntax. The agent block SHALL extend the prompt DSL, with the body text serving as the system prompt. Agent-specific directives SHALL include `@model`, `@tools #{[...]}`, `@skills #{[...]}`, `@agents #{[...]}` (for composition), `@constraints`, `@examples`, and `@on` (for lifecycle hooks).

#### Scenario: Basic agent declaration

- **WHEN** a reader looks at §5
- **THEN** the primary example SHALL be `@agent Coder ``` @model ... @tools #{[...]} You are an expert... ``` `

#### Scenario: Agent composition

- **WHEN** a reader looks at §5.2
- **THEN** it SHALL show an orchestrator agent using `@agents #{[Coder, Reviewer]}` to reference sub-agents

### Requirement: spec/lang.md §6 — Tool as @tool Annotation

The spec SHALL replace §6 "Tool Declarations" to use `@tool` annotation on `fn` declarations. The tool function SHALL be a normal AG function with `@tool` metadata. The compiler SHALL auto-generate JSON Schema from the function signature and doc comments.

#### Scenario: Basic tool declaration

- **WHEN** a reader looks at §6
- **THEN** the primary example SHALL show `@tool` before `fn read_file(path: str) -> str | Error { ... }`

#### Scenario: JSON Schema generation explained

- **WHEN** a reader looks at §6
- **THEN** it SHALL explain that the compiler generates JSON Schema from: function name, parameter names/types, return type, `///` doc comments, and `@param`/`@returns` annotations

### Requirement: spec/lang.md §7 — Skill as DSL Block

The spec SHALL replace §7 "Skill Declarations" to use `@skill` DSL block syntax. The skill block SHALL use `@description`, `@input { ... }`, and `@steps` directives.

#### Scenario: Skill declaration

- **WHEN** a reader looks at §7
- **THEN** the primary example SHALL be `@skill refactor ``` @description "..." @input { ... } @steps ... ``` `

### Requirement: spec/lang.md §8 — Component as DSL Block

The spec SHALL replace §8 "Component Declarations" to use `@component` DSL block syntax. The component block SHALL use `@props { ... }`, `@state { ... }`, `@render`, and `@style { ... }` directives. The render section SHALL use JSX-like syntax with `#{}` captures for dynamic content.

#### Scenario: Component declaration

- **WHEN** a reader looks at §8
- **THEN** the primary example SHALL be `@component DiffView ``` @props { ... } @render <div>...</div> @style { ... } ``` `

### Requirement: spec/lang.md §9 — HTTP Server as DSL Block

The spec SHALL replace §9 "HTTP Server" to use `@server` DSL block syntax. The server block SHALL use `@port`, `@middleware`, and route directives (`@get`, `@post`, `@put`, `@delete`, `@patch`). Route handlers SHALL be captured via `#{}` syntax.

#### Scenario: Server declaration

- **WHEN** a reader looks at §9
- **THEN** the primary example SHALL be `@server app ``` @port 3000 @get /health #{handler} ``` `

#### Scenario: Route handler captures

- **WHEN** a reader looks at §9
- **THEN** route handlers SHALL use `#{}` to capture handler functions, e.g. `@get /health #{fn(c) { c.json({ status: "ok" }) }}`

### Requirement: spec/lang.md §11 — Standard Library Layer Architecture

The spec SHALL update §11 to describe the two-layer stdlib architecture: Layer A (Web API externs, zero runtime cost, `std:web/*`) and Layer B (runtime-backed modules with `@js` extern + JS runtime, `std:http/*`, `std:log`, `std:fs`, etc.).

#### Scenario: Layer A described

- **WHEN** a reader looks at §11
- **THEN** it SHALL describe `std:web/fetch`, `std:web/crypto`, etc. as zero-cost extern declarations wrapping Web APIs

#### Scenario: Layer B described

- **WHEN** a reader looks at §11
- **THEN** it SHALL describe `std:http/server`, `std:http/client`, `std:log`, `std:fs`, `std:env` as runtime-backed modules with JS implementation in `@agentscript/stdlib`

### Requirement: spec/lang.md §12 — Grammar includes DSL blocks, extern, annotations

The spec SHALL update §12 Grammar to include productions for: DSL block (`"@" IDENT IDENT ("```" dsl_content "```" | "from" STRING)`), extern declarations, annotation syntax, and `@tool` on fn.

#### Scenario: DSL block production in grammar

- **WHEN** a reader looks at §12
- **THEN** the grammar SHALL include `dsl_decl = "@" IDENT IDENT (dsl_inline | dsl_fileref)`

#### Scenario: Extern production in grammar

- **WHEN** a reader looks at §12
- **THEN** the grammar SHALL include `extern_decl = "extern" ("fn" ... | "struct" ... | "type" ...)`

#### Scenario: Annotation production in grammar

- **WHEN** a reader looks at §12
- **THEN** the grammar SHALL include `annotation = "@" IDENT ("(" annotation_args ")")?` before applicable declarations

### Requirement: spec/lang.md §13 — Compilation includes DSL handler pipeline

The spec SHALL update §13 to show the DSL handler pipeline in the compilation flow. After parsing, DSL blocks are dispatched to registered handlers (prompt, agent, skill, component, server) during codegen. The compilation mapping table SHALL be updated.

#### Scenario: DSL in compilation pipeline

- **WHEN** a reader looks at §13.2
- **THEN** the pipeline SHALL show DSL handler dispatch as a step between IR and Code Gen (or as part of Code Gen)

#### Scenario: Updated compilation mapping

- **WHEN** a reader looks at §13.3
- **THEN** the mapping table SHALL include: `@agent Coder ``` ... ``` ` → `AgentRuntime class`, `@tool fn` → `function + schema`, `@server` → Hono routes, etc.
