## 1. AST Changes

- [x] 1.1 Add `ToolAnnotation` struct to `ag-ast/src/lib.rs` with fields: `description: Option<String>`, `span: Span`
- [x] 1.2 Add `tool_annotation: Option<ToolAnnotation>` field to `FnDecl` in `ag-ast/src/lib.rs`
- [x] 1.3 Update all `FnDecl` construction sites in the parser to include `tool_annotation: None` as default

## 2. Parser Changes

- [x] 2.1 Add `parse_tool_annotation()` method that parses `@tool` or `@tool("description")` and returns `ToolAnnotation`
- [x] 2.2 Add `parse_tool_annotated_fn()` method that parses the annotation then delegates to `parse_fn_decl()` with annotation
- [x] 2.3 Extend `parse_fn_decl()` signature to accept `tool_annotation: Option<ToolAnnotation>` parameter
- [x] 2.4 Extend the `TokenKind::At` branch in `parse_item()` to check for `"tool"` after `@` and dispatch to `parse_tool_annotated_fn()`
- [x] 2.5 Handle `@tool pub fn` ordering: after parsing annotation, check for `pub` before `fn`/`async`
- [x] 2.6 Handle `pub @tool fn` ordering: in the `TokenKind::Pub` branch, check for `@tool` after `pub`
- [x] 2.7 Emit parser error if `@tool` is not followed by `fn`, `pub`, or `async`: `"@tool annotation can only be applied to fn declarations"`

## 3. Parser Tests

- [x] 3.1 Test: `@tool fn foo() { }` produces `FnDecl` with `ToolAnnotation { description: None }`
- [x] 3.2 Test: `@tool("desc") fn foo() { }` produces `FnDecl` with `ToolAnnotation { description: Some("desc") }`
- [x] 3.3 Test: `@tool pub fn foo() { }` produces `FnDecl` with `is_pub: true` and `ToolAnnotation`
- [x] 3.4 Test: `pub @tool fn foo() { }` produces `FnDecl` with `is_pub: true` and `ToolAnnotation`
- [x] 3.5 Test: `@tool("desc") pub async fn foo() { }` produces `FnDecl` with `is_pub: true`, `is_async: true`, `ToolAnnotation`
- [x] 3.6 Test: `@tool struct Foo { }` produces parser error
- [x] 3.7 Test: `@tool let x = 5` produces parser error
- [x] 3.8 Test: `fn foo() { }` without annotation produces `FnDecl` with `tool_annotation: None`

## 4. Checker Changes

- [x] 4.1 Add tool registry data structure to checker symbol table (map of fn name to ToolAnnotation metadata)
- [x] 4.2 When visiting `FnDecl` with `tool_annotation: Some(...)`, register in tool registry
- [x] 4.3 Add `is_serializable_type()` helper that checks if a type maps to JSON Schema (str, num, int, bool, [T], {str: V}, serializable structs)
- [x] 4.4 Walk params of @tool fn and emit warning for non-serializable param types

## 5. Checker Tests

- [x] 5.1 Test: @tool fn with serializable params (str, int, bool) passes without warning
- [x] 5.2 Test: @tool fn with fn-type param emits serializability warning
- [x] 5.3 Test: @tool fn with array of serializable type passes without warning
- [x] 5.4 Test: @tool fn registered in tool registry after checking
- [x] 5.5 Test: non-tool fn not in tool registry
