## Context

The compiler already supports `@js` annotations on extern declarations. The parser recognizes `@` followed by `"js"` as a special case in the `TokenKind::At` branch of `parse_item()`, then calls `parse_js_annotated_extern()` which parses the annotation and attaches a `JsAnnotation` to the subsequent `ExternFnDecl`/`ExternStructDecl`/`ExternTypeDecl`. The `@tool` annotation follows the same pattern but applies to `fn` declarations instead of `extern` declarations.

## Goals / Non-Goals

**Goals:**
- Parse `@tool` (no args) and `@tool("description")` annotations before `fn` declarations
- Attach a `ToolAnnotation` to the `FnDecl` AST node
- Validate in the checker that `@tool` only appears on `fn` declarations
- Warn in the checker if `@tool` fn has non-serializable param types

**Non-Goals:**
- No codegen changes (JSON Schema generation is a future change)
- No runtime changes
- No `@tool` support on extern fn, struct, enum, let, or other declarations

## Decisions

### Decision 1: Add ToolAnnotation to AST

Add a `ToolAnnotation` struct to `ag-ast`, similar to `JsAnnotation`:

```rust
pub struct ToolAnnotation {
    pub description: Option<String>,
    pub span: Span,
}
```

The `description` is `None` for bare `@tool` and `Some("...")` for `@tool("description")`. This is attached to `FnDecl` via a new `tool_annotation: Option<ToolAnnotation>` field.

### Decision 2: Extend existing annotation parsing in ag-parser

The parser's `TokenKind::At` branch already lookaheads for `"js"`. Extend this to also check for `"tool"`. When parser sees `@` followed by `"tool"`, call a new `parse_tool_annotated_fn()` method that:

1. Parses the `@tool` or `@tool("description")` annotation
2. Expects `fn`, `pub fn`, `async fn`, or `pub async fn` to follow
3. Passes the annotation into `parse_fn_decl()` (which needs a new parameter)

This keeps the annotation dispatch in one place and mirrors the `@js` path.

### Decision 3: Checker validates @tool only on fn

The parser itself enforces this by requiring `fn`/`pub`/`async` after `@tool`. If the token after the annotation is not one of these, the parser emits an error: `"@tool annotation can only be applied to fn declarations"`. This matches how `@js` emits `"@js annotation can only be applied to extern declarations"`.

### Decision 4: Checker validates serializability

The checker walks `@tool`-annotated functions and checks that each parameter type maps to a JSON Schema type. Serializable types are: `str`, `num`, `int`, `bool`, `[T]` (where T is serializable), `{K: V}` (where K is str and V is serializable), and structs whose fields are all serializable. Non-serializable types (fn types, opaque extern types, Promise, etc.) produce a warning, not a hard error. This allows progressive adoption.

## Risks / Trade-offs

- **[Parser lookahead grows]** Adding `"tool"` to the `@` branch is a second string comparison. This is trivial and follows the established pattern. If more annotations are added later, this should be refactored into a general annotation dispatch table.
- **[Serializability check is approximate]** Without full type resolution across modules, the checker may miss some non-serializable types. Making it a warning (not error) mitigates this.
