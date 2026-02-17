## MODIFIED Requirements

### Requirement: Runtime import generation

When the handler produces `PromptTemplate` constructor calls, it SHALL also emit an import statement: `import { PromptTemplate } from "@agentscript/prompt-runtime";`. The import SHALL be emitted once per module, not once per prompt block. SWC helper functions (`ident`, `str_lit`, `expr_or_spread`, `make_prop`, `emit_module`) SHALL be imported from `ag_dsl_core::swc_helpers` instead of being defined locally. Content-building functions (`build_content_expr`, `build_output_schema`, `build_constraints_expr`, `ag_type_to_json_schema`, `constraint_value_to_expr`) SHALL be `pub` to allow reuse by other DSL codegen crates (e.g., agent).

#### Scenario: Single import for multiple prompts

- **WHEN** a module contains two `@prompt` blocks
- **THEN** output contains exactly one `import { PromptTemplate } from "@agentscript/prompt-runtime";` at the top

#### Scenario: SWC helpers from ag-dsl-core

- **WHEN** the prompt codegen module is compiled
- **THEN** it SHALL use `ag_dsl_core::swc_helpers::{ident, str_lit, expr_or_spread, make_prop}` instead of local function definitions

#### Scenario: Content builders are public

- **WHEN** the `ag-dsl-agent` crate imports `ag_dsl_prompt::codegen::build_content_expr`
- **THEN** the import SHALL compile successfully (function is `pub`)
