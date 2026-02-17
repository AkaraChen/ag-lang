## Context

Based on the extensible DSL framework (`@kind name ``` ... ```) and the `ag-dsl-prompt` crate pattern, this change implements the skill DSL handler. Skill declarations use `@skill` DSL blocks with `@description`, `@input`, `@steps`, and `@output` directives. We need an `ag-dsl-skill` crate to parse these into a structured `SkillTemplate` AST.

Scope is lexer + parser + validator only. No codegen — that comes in a follow-up change.

## Goals / Non-Goals

**Goals:**
- New crate `ag-dsl-skill`, same structure as `ag-dsl-prompt` (lexer, ast, parser, validator)
- Parse `@description`, `@input`, `@steps`, `@output` directives into `SkillTemplate` AST
- Validate required directives and reject duplicates
- Fully independent testability — no dependency on `ag-lexer`/`ag-parser`/`ag-ast`

**Non-Goals:**
- No codegen (SkillTemplate -> JS) — separate change
- No DslHandler impl or integration with `ag-codegen` — separate change
- No runtime type resolution — type names are plain strings at this level

## Decisions

### 1. Crate structure mirrors ag-dsl-prompt

```
crates/
└── ag-dsl-skill/
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs           # public API
    │   ├── ast.rs           # SkillTemplate AST
    │   ├── lexer.rs         # skill directive lexer
    │   ├── parser.rs        # token stream -> AST
    │   └── validator.rs     # structural validation
    └── tests/
```

**Dependency**: `ag-dsl-skill` depends on `ag-dsl-core` for `DslPart`, `Span`, and shared types. No SWC dependency since there is no codegen.

### 2. SkillTemplate AST

```rust
struct SkillTemplate {
    name: String,
    description: Option<String>,
    input_fields: Vec<SkillField>,
    steps: Vec<SkillStep>,
    output_fields: Vec<SkillField>,
}

struct SkillField {
    name: String,
    type_name: String,         // "str", "bool", "[str]", etc.
    default: Option<String>,   // literal default value as string
}

struct SkillStep {
    number: u32,               // 1-based step number
    text: String,              // natural language description
    captures: Vec<String>,     // #{} capture names found in text
}
```

**Rationale**: `SkillField` stores `type_name` as a plain string — no AG type resolution at this level. Types are identifiers like `"str"`, `"bool"`, `"int"`, `"[str]"`. Resolution to actual AG types is deferred to codegen or the host type checker.

### 3. @description takes a quoted string

`@description "Refactor code for readability"` — the lexer expects a string literal after the directive keyword. This matches how descriptions appear in skill blocks: a single descriptive sentence.

**Alternative considered**: Freeform text until next directive (like @steps). Rejected because descriptions should be concise single values, and quoting makes the boundary explicit.

### 4. @input and @output parse field declaration blocks

`@input { query: str, max_results: int = 10, dry_run: bool = false }`

Field syntax: `name: type` with optional `= default`. The `{...}` braces delimit the block. Both `@input` and `@output` use the same field syntax, but `@output` fields do not support defaults.

**Rationale**: Reusing field declaration syntax across input/output keeps the DSL consistent. Defaults only make sense for inputs (callers can omit them), not outputs (the skill always produces all output fields).

### 5. Type names are plain strings

Type names are lexed as identifiers or bracket-wrapped identifiers: `str`, `bool`, `int`, `num`, `[str]`, `[int]`. No generics, no nested types, no `Option<T>` — just flat string tokens.

**Rationale**: The skill DSL operates at a configuration level. Full type resolution belongs to the AG type checker, not the DSL parser. Keeping types as strings avoids coupling the DSL crate to the AG type system.

### 6. @steps collects free-form text with captures

`@steps` is followed by free-form text until the next `@` directive or end of input. Inside steps text, numbered lines (`1.`, `2.`, etc.) mark individual steps. `#{}` captures within step text reference input fields or expressions.

```
@steps
1. Analyze the #{language} code in #{file_path}
2. Identify refactoring opportunities
3. Apply #{strategy} pattern if dry_run is false
```

The parser splits on numbered line prefixes and extracts capture names from `#{...}` patterns within each step's text.

**Rationale**: Steps are natural language instructions, not structured data. Numbered lists are the natural way to express ordered steps. Captures allow referencing input fields, maintaining the DSL's interpolation model.

### 7. Validator enforces required directives and no duplicates

Required: `@description`, `@input`, `@steps`. Optional: `@output`.

Duplicate directives of any kind are errors. The validator checks the parsed AST for structural completeness.

**Rationale**: A skill without a description, inputs, or steps is meaningless. Output is optional because some skills perform side effects rather than producing structured data.

## Risks / Trade-offs

- **No codegen in scope**: The `SkillTemplate` AST is not yet consumed by anything. This is intentional — the AST design can be validated independently before committing to a codegen strategy.
- **Type names as strings lose early validation**: Invalid types like `"foobar"` pass the DSL parser. Mitigation: the AG type checker validates type names when the skill is used in a full compilation.
- **Step numbering is positional**: The parser expects `1.`, `2.`, etc. in order. Non-numbered text after `@steps` is collected as a single unnumbered step. This is a pragmatic choice — strict numbering validation can be added later.
- **Default values as strings**: `default: Option<String>` stores the literal text (e.g., `"false"`, `"10"`), not a parsed value. Codegen will need to interpret these. This avoids value-type coupling at the parser level.
