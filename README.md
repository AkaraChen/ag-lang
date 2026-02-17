# AgentScript

A purpose-built programming language for authoring AI agents. AgentScript has a JS-flavored syntax with first-class DSL blocks for prompts, agents, skills, UI components, and HTTP servers; annotations for tool metadata and JS interop; and compiles to Node.js JavaScript.

## Quick Start

```bash
# Build the compiler
cargo build --workspace

# Compile an AgentScript file to JavaScript
cargo run -p ag-cli -- build examples/simple-agent/app.ag

# Type-check without compiling
cargo run -p ag-cli -- check examples/simple-agent/app.ag
```

## Language Overview

````javascript
// Tools — annotated functions that agents can invoke
@tool("Look up documentation for a topic")
fn lookup_docs(topic: str) -> str {
    `Documentation for: ${topic}`
}

// Prompts — first-class DSL blocks for LLM messages
@prompt system_prompt ```
@role system
You are a helpful coding assistant.
You specialize in TypeScript and Rust programming.
```

// Pattern matching, pipe operator
fn process(input: str) -> str {
    input |> classify |> format_result
}

fn classify(q: str) -> str {
    match q {
        "math" => "calculation",
        "docs" => "documentation",
        _ => "general",
    }
}
````

See `spec/lang.md` for the full language specification and `examples/` for working code.

## Language Features

- **JS-flavored syntax** — `fn`, `let`, `mut`, `match`, `|>` pipe, `?.` optional chain, `??` nullish coalesce
- **DSL blocks** — `@prompt`, `@agent`, `@skill`, `@component`, `@server` with `#{ expr }` captures
- **Annotations** — `@tool` marks functions as LLM-callable tools, `@js("module")` binds to JavaScript
- **Extern declarations** — type-safe bindings to JavaScript functions, structs, and types
- **Structural typing** — basic type inference, union types, nullable types
- **Standard library** — Web APIs (`std:web/fetch`), HTTP server/client, filesystem, logging

## Project Structure

```
crates/
  ag-ast/          # AST node types shared across all crates
  ag-lexer/        # Tokenizer with DSL raw mode
  ag-parser/       # Recursive descent parser (LL(1))
  ag-checker/      # Type checker (structural typing, basic inference)
  ag-codegen/      # JavaScript codegen via SWC
  ag-cli/          # CLI: `asc build` and `asc check`
  ag-stdlib/       # Standard library module resolver
  ag-dsl-core/     # DslHandler trait and CodegenContext trait
  ag-dsl-prompt/   # @prompt DSL (complete: parse + codegen)
  ag-dsl-agent/    # @agent DSL (parse only)
  ag-dsl-skill/    # @skill DSL (parse only)
  ag-dsl-server/   # @server DSL (parse only)
  ag-dsl-component/# @component DSL (parse only)
spec/
  lang.md          # Language specification v0.2
examples/
  simple-agent/    # Prompts, tools, match, pipe
  http-server/     # HTTP routes, handlers, JSON
```

## Tests

```bash
cargo test --workspace   # 315 tests across all crates
```
