## ADDED Requirements

### Requirement: Shared SWC helper module in ag-dsl-core

The `ag-dsl-core` crate SHALL export a `swc_helpers` module containing shared SWC AST construction functions used by all DSL codegen crates. The module SHALL be accessible as `ag_dsl_core::swc_helpers`.

#### Scenario: Module is importable

- **WHEN** a DSL crate depends on `ag-dsl-core`
- **THEN** it SHALL be able to `use ag_dsl_core::swc_helpers::{ident, str_lit, make_prop}` without additional dependencies beyond `swc_ecma_ast`

### Requirement: ident helper

The `ident(name: &str) -> swc::Ident` function SHALL create a SWC `Ident` node with the given name, `DUMMY_SP` span, empty `SyntaxContext`, and `optional: false`.

#### Scenario: Create identifier

- **WHEN** `ident("foo")` is called
- **THEN** the returned `Ident` SHALL have `sym: "foo"`, `span: DUMMY_SP`, `optional: false`

### Requirement: str_lit helper

The `str_lit(s: &str) -> swc::Expr` function SHALL create a SWC string literal expression wrapping the given string.

#### Scenario: Create string literal

- **WHEN** `str_lit("hello")` is called
- **THEN** the returned `Expr` SHALL be `Expr::Lit(Lit::Str(Str { value: "hello", ... }))`

### Requirement: num_lit helper

The `num_lit(n: f64) -> swc::Expr` function SHALL create a SWC numeric literal expression with the given value.

#### Scenario: Create number literal

- **WHEN** `num_lit(42.0)` is called
- **THEN** the returned `Expr` SHALL be `Expr::Lit(Lit::Num(Number { value: 42.0, ... }))`

### Requirement: bool_lit helper

The `bool_lit(b: bool) -> swc::Expr` function SHALL create a SWC boolean literal expression.

#### Scenario: Create boolean literal

- **WHEN** `bool_lit(true)` is called
- **THEN** the returned `Expr` SHALL be `Expr::Lit(Lit::Bool(Bool { value: true, ... }))`

### Requirement: expr_or_spread helper

The `expr_or_spread(expr: swc::Expr) -> swc::ExprOrSpread` function SHALL wrap an expression in an `ExprOrSpread` with `spread: None`.

#### Scenario: Wrap expression

- **WHEN** `expr_or_spread(str_lit("x"))` is called
- **THEN** the returned value SHALL have `spread: None` and `expr` pointing to the string literal

### Requirement: make_prop helper

The `make_prop(key: &str, value: swc::Expr) -> swc::PropOrSpread` function SHALL create a key-value property for use in object literals, with the key as an `IdentName`.

#### Scenario: Create property

- **WHEN** `make_prop("name", str_lit("Alice"))` is called
- **THEN** the returned value SHALL be `PropOrSpread::Prop(KeyValueProp { key: "name", value: "Alice" })`

### Requirement: emit_module helper

The `emit_module(items: &[swc::ModuleItem]) -> String` function SHALL construct a SWC `Module` from the given items and emit it as a JavaScript string using `swc_ecma_codegen::Emitter`.

#### Scenario: Emit simple module

- **WHEN** `emit_module` is called with module items containing a variable declaration `const x = 1`
- **THEN** the returned string SHALL contain `const x = 1;`

### Requirement: ag-dsl-core Cargo.toml dependencies

The `ag-dsl-core/Cargo.toml` SHALL include `swc_common = "18"` and `swc_ecma_codegen = "23"` as dependencies (in addition to existing `swc_ecma_ast = "20"`).

#### Scenario: Crate compiles with new dependencies

- **WHEN** `cargo build -p ag-dsl-core` is run
- **THEN** the build SHALL succeed with no errors
