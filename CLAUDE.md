# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

AgentScript (ag-lang) is a compiler for a purpose-built language for authoring AI agents. It has a JS-flavored syntax with first-class DSL blocks (`@prompt`, `@agent`, `@skill`, `@component`, `@server`), annotations (`@tool`, `@js`), and compiles to Node.js JavaScript. The compiler is written in Rust.

The language spec is at `spec/lang.md`.

## Build & Test

```bash
cargo build --workspace       # build all 13 crates
cargo test --workspace        # run all ~315 tests
cargo test -p ag-lexer        # test a single crate
cargo test -p ag-codegen -- pipe  # run tests matching "pipe" in ag-codegen
cargo run -p ag-cli -- build examples/simple-agent/app.ag  # compile an .ag file
cargo run -p ag-cli -- check examples/simple-agent/app.ag  # type-check only
```

**CRITICAL**: NEVER run multiple `cargo build`/`cargo test`/`cargo run` commands in background simultaneously. Parallel Rust compilations will exhaust system memory. Always run them sequentially.

## Architecture

The compiler follows a classic pipeline. Each stage is a separate crate:

```
.ag source → ag-lexer → ag-parser → ag-checker → ag-codegen → .js output
                                                       ↑
                                              ag-dsl-{kind} handlers
```

### Crate Dependency Graph

```
ag-cli (binary)
 ├── ag-parser  (parse source → AST)
 │    └── ag-ast  (AST node types, shared by all crates)
 ├── ag-checker (type check AST)
 │    └── ag-ast
 ├── ag-codegen (AST → JavaScript via SWC)
 │    ├── ag-ast
 │    ├── ag-dsl-core  (DslHandler trait, CodegenContext trait)
 │    └── ag-dsl-prompt  (only registered codegen handler)
 └── ag-stdlib (resolves `std:` imports by injecting extern declarations)
      └── ag-parser
```

### Key Concepts

**DSL blocks** (`@kind name ``` ... ``` `) are the core domain abstraction. The parser captures them as `DslBlock` AST nodes with raw text + `#{ expr }` captures. During codegen, each block is dispatched to a registered `DslHandler`. Each DSL kind has its own crate (`ag-dsl-{kind}`) with internal lexer/parser/validator/AST for its directive syntax.

**A complete DSL crate** (see `ag-dsl-prompt`) has: `ast.rs`, `lexer.rs`, `parser.rs`, `validator.rs`, `codegen.rs`, `handler.rs`. The handler implements `ag_dsl_core::DslHandler` and is registered in `ag-codegen/src/lib.rs`.

**Incomplete DSL crates** (`ag-dsl-agent`, `ag-dsl-skill`, `ag-dsl-server`, `ag-dsl-component`) have parsing infrastructure but no `codegen.rs` or `handler.rs` — they cannot emit JavaScript yet. Use `ag-dsl-prompt` as the reference implementation when adding codegen to these.

**Annotations** (`@tool`, `@js`) are parsed in `ag-parser` and stored on `FnDecl`/`ExternFnDecl` nodes. `@tool` is validated in `ag-checker`. `@js` generates import statements in `ag-codegen`.

**Codegen uses SWC** — the `ag-codegen` crate builds `swc_ecma_ast` nodes and emits JavaScript through `swc_ecma_codegen`. All expression/statement translation happens in `ag-codegen/src/lib.rs`. The `CodegenContext` trait bridges AG expression translation into DSL handlers.

**Stdlib resolution** happens in `ag-cli`: `std:` prefixed imports are resolved by parsing bundled AG source from `ag-stdlib`, then injecting extern declarations into the module before type-checking.

### Implementation Status

| Feature | Parse | Check | Codegen |
|---------|-------|-------|---------|
| Core language (fn, let, match, pipe, etc.) | done | done | done |
| `@prompt` DSL | done | done | done |
| `@tool` annotation | done | done | partial (no JSON schema gen) |
| `@js` annotation + extern | done | done | done |
| `@agent` DSL | done | - | **missing** |
| `@skill` DSL | done | - | **missing** |
| `@server` DSL | done | - | **missing** |
| `@component` DSL | done | - | **missing** |

## Commit after opsx:apply

## OpenSpec

This project uses OpenSpec (`/opsx:*` commands) for structured change management. Archived changes are in `openspec/changes/archive/`. Main specs are in `openspec/specs/`.
