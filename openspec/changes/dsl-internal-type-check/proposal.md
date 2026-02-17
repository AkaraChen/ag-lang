## Why

DSL blocks (`@prompt`, `@agent`, `@skill`, `@server`, `@component`) have internal validators that catch semantic errors (duplicate routes, missing required directives, invalid ports, etc.), but these validators are never called during the type-check phase. Currently `ag-checker::check_dsl_block` only type-checks capture expressions — it is completely kind-blind and does not validate DSL-internal structure. This means DSL syntax errors and semantic violations are only caught at codegen time (or never, for DSL crates without codegen). Users get no early error feedback for malformed DSL blocks.

## What Changes

- Add DSL crate dependencies to `ag-checker` and dispatch `check_dsl_block` by `dsl.kind` to each DSL's lexer → parser → validator pipeline
- Map DSL-internal `Diagnostic` types back to the checker's `Diagnostic` format with correct source spans
- Add a `validator.rs` to `ag-dsl-component` (the only DSL crate missing one)
- Validate `SkillField::type_name` strings against known AG types in the checker scope
- Surface all DSL validator warnings/errors as checker diagnostics (prompt's no-`@role` warning, server's duplicate routes, agent's duplicate hooks, skill's missing description/input/steps, etc.)

## Capabilities

### New Capabilities
- `dsl-checker-dispatch`: Dispatching DSL blocks by kind in `ag-checker` to run each DSL crate's lexer/parser/validator during the type-check phase, mapping diagnostics back to the checker
- `component-dsl-validator`: Adding a `validator.rs` to `ag-dsl-component` for semantic validation of component metadata (prop types, render presence)
- `skill-type-validation`: Validating `SkillField::type_name` strings in `@skill` blocks against the AG type system

### Modified Capabilities

## Impact

- **Crates modified**: `ag-checker` (add DSL dispatch logic, add DSL crate dependencies), `ag-dsl-component` (add validator.rs), potentially `ag-dsl-core` (if a shared diagnostic mapping trait is needed)
- **Dependencies added**: `ag-dsl-prompt`, `ag-dsl-agent`, `ag-dsl-skill`, `ag-dsl-server`, `ag-dsl-component` to `ag-checker/Cargo.toml`
- **No breaking changes** — purely additive; existing capture type-checking is preserved
- **User-facing improvement**: `.ag` files with malformed DSL blocks will now get errors at `check` time instead of silently passing to codegen
