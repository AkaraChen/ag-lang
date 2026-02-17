## Context

The `@tool` annotation pipeline is complete through parsing and type-checking, but codegen emits no schema. The checker already builds a `tool_registry: HashMap<String, ToolInfo>` with resolved `Type` enums per parameter. The codegen crate works with the raw AST (`TypeExpr`) and currently has no access to checker results.

Two approaches exist: (A) generate schemas from AST `TypeExpr` in codegen, or (B) pass the checker's resolved `Type` data to codegen. Each has trade-offs around struct resolution.

## Goals / Non-Goals

**Goals:**
- Emit `fnName.schema = { name, description, parameters }` for every `@tool`-annotated function
- Map all serializable AG types to JSON Schema: primitives, arrays, objects, nullable, unions, enums
- Handle `required` array: parameters with defaults are optional
- Use `@tool("...")` description as the schema description

**Non-Goals:**
- Doc comment (`///`) parsing and `@param`/`@returns` extraction — separate future change
- Per-parameter descriptions in schema — depends on doc comments
- Schema validation at runtime — schemas are static metadata only
- Strict mode / `additionalProperties: false` — can be added later

## Decisions

### 1. Pass checker's `ToolInfo` to codegen via a new parameter

The codegen entry point `codegen(module)` becomes `codegen(module, tool_registry)`. The `ToolInfo` contains resolved `Type` enums which already handle struct field expansion and type alias resolution. This avoids duplicating type resolution logic in codegen.

**Rationale**: The checker already resolves struct types to their fields, type aliases to their targets, and validates serializability. Reusing this in codegen is simpler and ensures consistency. The AST `TypeExpr::Named("MyStruct")` can't be expanded without the checker's scope.

**Alternatives considered**:
- Generate from AST `TypeExpr` only: works for primitives/arrays/maps but fails for named struct types since codegen doesn't have scope to resolve `TypeExpr::Named("Config")` to its fields
- Pass full `CheckResult`: too broad; only `tool_registry` is needed

### 2. Schema emitted as assignment statement after function declaration

For `@tool fn foo(...) { ... }`, codegen emits:
```javascript
function foo(...) { ... }
foo.schema = { name: "foo", description: "...", parameters: { ... } };
```

This is a separate `ExpressionStatement` in the SWC module body, immediately after the function declaration.

**Rationale**: Matches the pattern shown in the language spec (section 7.3). Property assignment is simple to generate in SWC and doesn't require wrapping the function.

**Alternatives considered**:
- Object.defineProperty — unnecessary complexity
- Wrapping in IIFE — obscures the function declaration

### 3. Type-to-JSON-Schema mapping implemented as a standalone function in a new module

New file `ag-codegen/src/tool_schema.rs` with a pure function `type_to_json_schema(ty: &Type) -> swc::Expr` that recursively maps checker `Type` to SWC object literals representing JSON Schema.

**Mapping table:**

| AG Type | JSON Schema |
|---------|-------------|
| `str` | `{ "type": "string" }` |
| `num` | `{ "type": "number" }` |
| `int` | `{ "type": "integer" }` |
| `bool` | `{ "type": "boolean" }` |
| `[T]` | `{ "type": "array", "items": <T-schema> }` |
| `{str: V}` | `{ "type": "object", "additionalProperties": <V-schema> }` |
| `Struct(fields)` | `{ "type": "object", "properties": { ... }, "required": [...] }` |
| `T?` / `Nullable(T)` | merge `<T-schema>` with nullable handling |
| `T \| U` (Union) | `{ "anyOf": [<T-schema>, <U-schema>] }` |
| `Enum(variants)` | `{ "anyOf": [variant-schemas...] }` |
| `any` / `unknown` | `{}` (no constraints) |

**Rationale**: Standalone module keeps schema generation testable in isolation. Using checker `Type` (not AST `TypeExpr`) makes the mapping straightforward — no need to resolve names.

### 4. Optional parameters determined by AST default values

The `required` array excludes parameters that have `default: Some(...)` in the AST `FnDecl.params`. This information is available directly from the AST without needing checker involvement.

**Rationale**: Default value presence is a syntactic property visible in the AST. No type-level analysis needed.

## Risks / Trade-offs

- **[Codegen API change]** Adding `tool_registry` parameter to `codegen()` changes the public API → Mitigation: Only one call site (`ag-cli`), easy to update
- **[Checker dependency in codegen]** `ag-codegen` now depends on `ag-checker::Type` → Mitigation: Consider defining the type mapping types in a shared crate, or just depend on `ag-checker` directly. The simplest approach is to have codegen depend on checker types. Alternatively, re-export `Type` from a shared location.
- **[Enum schemas complexity]** AG enums with variants produce complex `anyOf` schemas → Mitigation: Start with simple flat mapping; iterate on complex cases later
