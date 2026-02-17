## Context

Currently `ag-checker::check_dsl_block` is kind-blind — it only type-checks capture expressions via `check_expr`. Each DSL crate has its own lexer → parser → validator pipeline producing DSL-specific diagnostics (`Diagnostic { message, severity }`), but these validators are either called only at codegen time (prompt) or never called from the compiler at all (agent, skill, server). The checker has no dependency on any DSL crate and no knowledge of DSL-internal structure.

The DSL-internal `Diagnostic` types (in each crate's `parser.rs` and `validator.rs`) have `message: String` and `severity: Severity` but no source span. The checker's diagnostic type (`ag_ast::Diagnostic`) has `message: String` and `span: Span`.

## Goals / Non-Goals

**Goals:**
- Run each DSL crate's lexer → parser → validator during the type-check phase, catching DSL syntax and semantic errors early
- Map DSL-internal diagnostics to `ag_ast::Diagnostic` with the DSL block's span
- Add a `validator.rs` to `ag-dsl-component` for semantic validation
- Validate `SkillField::type_name` strings against known AG types
- Surface all DSL validator errors and warnings as checker diagnostics

**Non-Goals:**
- Cross-referencing capture types against DSL-specific requirements (e.g. verifying `@tools` capture is an array) — this is future work requiring richer capture-type plumbing
- Checking `DslContent::FileRef` blocks (requires filesystem access at check time)
- Adding a `DslChecker` trait to `ag-dsl-core` — direct dependencies are simpler for now, matching the pattern in `ag-codegen`

## Decisions

### 1. Direct DSL crate dependencies in ag-checker

Add `ag-dsl-prompt`, `ag-dsl-agent`, `ag-dsl-skill`, `ag-dsl-server`, `ag-dsl-component` as dependencies in `ag-checker/Cargo.toml`. The checker calls each crate's `lexer::lex()` + `parser::parse()` + `validator::validate()` directly.

**Rationale:** Matches the pattern used in `ag-codegen` where DSL crates are direct dependencies. Avoids introducing a new trait system. The set of DSL kinds is small and stable.

**Alternative considered:** A `DslChecker` trait in `ag-dsl-core` — rejected as over-engineering for 5 DSL kinds. Can be added later if extensibility is needed.

### 2. Dispatch by `dsl.kind` string

`check_dsl_block` branches on `dsl.kind` (`"prompt"`, `"agent"`, `"skill"`, `"server"`, `"component"`). Unknown kinds are silently ignored during checking (they'll error at codegen time via the handler registry).

**Rationale:** The checker validates what it can; codegen still catches unregistered kinds. This avoids duplicate error reporting.

### 3. Diagnostic mapping: DSL block span as fallback

DSL-internal diagnostics lack source spans. When mapping to `ag_ast::Diagnostic`, use the DSL block's span (`dsl.span`) as the location. This isn't precise to the line within the DSL block, but it points the user to the right block.

**Rationale:** Adding source spans to all DSL-internal diagnostics would require plumbing span offsets through each DSL lexer/parser/validator — significant refactoring. Using the block span is a practical first step.

### 4. Component validator: minimal checks

Add `validator.rs` to `ag-dsl-component` checking:
- Props with unknown AG types (types not in `["str", "num", "int", "bool", "nil", "any"]` and not array syntax)
- Duplicate prop names

Since `ag-dsl-component` uses SWC parsing (not the standard DSL lexer/parser pattern), the checker calls `parse_component()` directly and then validates the resulting `ComponentMeta`.

### 5. Skill type validation: resolve against checker scope

`SkillField::type_name` strings (e.g. `"str"`, `"[int]"`, `"UserType"`) are validated by attempting to resolve them as AG type expressions. Invalid type names produce checker errors. This reuses the existing `resolve_type` logic.

## Risks / Trade-offs

- **[Risk] DSL diagnostic spans are imprecise** — All DSL errors point to the whole `@kind name` block, not the specific line within it. → Mitigation: Error messages are descriptive enough to locate the issue. Future work can add intra-block span offsets.
- **[Risk] ag-checker now depends on all DSL crates** — This increases compile time for the checker. → Mitigation: DSL crates are small (lexer + parser + validator, no codegen/SWC deps except component). The compile time impact is minimal.
- **[Risk] ag-dsl-component depends on SWC parser** — Adding it as a checker dependency pulls in SWC parser crates. → Mitigation: These are already in the dependency tree via `ag-codegen`. No new transitive dependencies.
