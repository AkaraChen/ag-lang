## Context

The compiler pipeline flows `ag-parser → ag-checker → ag-codegen`. However, `ag-codegen` currently depends on `ag-checker` to access the `Type` enum and `ToolInfo` struct. This dependency exists solely for emitting `fnName.schema = { ... }` JSON Schema objects for `@tool`-annotated functions. The `tool_schema.rs` module in codegen converts checker `Type` values to SWC AST expressions representing JSON Schema.

Current data flow:
```
ag-checker::check()
  → CheckResult { tool_registry: HashMap<String, ToolInfo> }
      ToolInfo { description: Option<String>, param_types: Vec<(String, Type)> }

ag-cli passes tool_registry to ag-codegen

ag-codegen::tool_schema::build_tool_schema()
  → converts Type → JSON Schema swc::Expr
```

## Goals / Non-Goals

**Goals:**
- Remove `ag-codegen`'s dependency on `ag-checker`
- Introduce a clean intermediate representation for tool schema info that both checker and codegen can use
- Specify the currently undocumented `@tool` schema emission behavior

**Non-Goals:**
- Changing the generated JavaScript output (must remain identical)
- Restructuring how DSL handlers work
- Moving tool_schema.rs to a separate crate (overkill for now)

## Decisions

### Decision 1: Place `JsonSchema` and `ToolSchemaInfo` in `ag-ast`

**Choice**: Add the intermediate types to `ag-ast`.

**Alternatives considered**:
- **New `ag-types` crate**: Adds a crate for two types. Over-engineered.
- **Move `Type` to `ag-ast`**: `Type` is semantic (checker output), not syntactic. Wrong home.
- **Convert in `ag-cli`**: Pushes type conversion logic into the binary. Less clean separation.

**Rationale**: `ag-ast` is already the shared vocabulary crate — it contains `Span`, `Diagnostic`, `ToolAnnotation`, and all AST types. Both `ag-checker` and `ag-codegen` already depend on it. Adding `JsonSchema` (a data transfer type) is consistent with this role.

### Decision 2: Checker produces `ToolSchemaInfo` directly

**Choice**: The checker converts `Type → JsonSchema` at tool registration time, producing `ToolSchemaInfo` instead of `ToolInfo`.

**Rationale**: The conversion logic (Type → JsonSchema) is simple and belongs with the Type-aware code. The checker already does serializability validation, so it already understands the mapping. This keeps the conversion colocated with the type system.

### Decision 3: Keep `tool_schema.rs` in `ag-codegen`

**Choice**: The file stays in codegen but changes its input from `Type` to `JsonSchema`.

**Rationale**: The file's job is converting schema descriptions to SWC AST — that's codegen work. It just no longer needs to know about checker types.

## Risks / Trade-offs

- **[Duplication of type concepts]** `JsonSchema` partially mirrors `Type` for serializable types → Acceptable because `JsonSchema` is a transfer type with different semantics (JSON Schema, not AG types). The overlap is intentional.
- **[API change in CheckResult]** `tool_registry` type changes → All consumers (just `ag-cli`) need updating. Single call site, low risk.
