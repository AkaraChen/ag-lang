## 1. Dependencies & Setup

- [x] 1.1 Add `ag-dsl-prompt`, `ag-dsl-agent`, `ag-dsl-skill`, `ag-dsl-server`, `ag-dsl-component` as dependencies in `ag-checker/Cargo.toml`
- [x] 1.2 Verify `cargo build -p ag-checker` succeeds with new dependencies

## 2. Component Validator

- [x] 2.1 Create `ag-dsl-component/src/validator.rs` with `Diagnostic`, `Severity` types and `validate(meta: &ComponentMeta) -> Vec<Diagnostic>` function
- [x] 2.2 Implement duplicate prop name detection (error)
- [x] 2.3 Implement unknown prop type warning (non-primitive, non-array types)
- [x] 2.4 Implement empty props warning
- [x] 2.5 Add `pub mod validator;` to `ag-dsl-component/src/lib.rs`
- [x] 2.6 Add unit tests for the component validator (valid component, duplicate props, unknown types, no props)

## 3. Checker DSL Dispatch

- [x] 3.1 Refactor `check_dsl_block` to dispatch by `dsl.kind` for inline blocks, calling each DSL crate's lex → parse → validate pipeline
- [x] 3.2 Implement prompt DSL checking: call `ag_dsl_prompt::lexer::lex` → `parser::parse` → `validator::validate`, map diagnostics to `ag_ast::Diagnostic` with block span
- [x] 3.3 Implement agent DSL checking: call `ag_dsl_agent::lexer::lex` → `parser::parse` → `validator::validate`, map diagnostics
- [x] 3.4 Implement skill DSL checking: call `ag_dsl_skill::lexer::lex` → `parser::parse` → `validator::validate`, map diagnostics
- [x] 3.5 Implement server DSL checking: call `ag_dsl_server::lexer::lex` → `parser::parse` → `validator::validate`, map diagnostics
- [x] 3.6 Implement component DSL checking: call `ag_dsl_component::parse_component` → `validator::validate`, map diagnostics
- [x] 3.7 Handle parse errors from DSL parsers: map parser `Result::Err` diagnostics to checker errors with block span
- [x] 3.8 Ensure unknown DSL kinds are silently skipped (no checker error)

## 4. Skill Type Validation

- [x] 4.1 Add a helper method to the checker that resolves a `SkillField::type_name` string against the checker's scope and type aliases
- [x] 4.2 After successful skill DSL parsing, validate each input and output field type name, reporting errors for unresolvable types
- [x] 4.3 Add unit tests for skill type validation: primitives pass, arrays pass, unknown types error, struct types pass, type aliases pass

## 5. Integration Tests

- [x] 5.1 Add checker tests for prompt DSL validation (empty prompt → error)
- [x] 5.2 Add checker tests for server DSL validation (port 0 → error, duplicate routes → error)
- [x] 5.3 Add checker tests for agent DSL validation (duplicate hooks → error)
- [x] 5.4 Add checker tests for skill DSL validation (missing description → error, missing input → error)
- [x] 5.5 Add checker tests for component DSL validation (duplicate props → error)
- [x] 5.6 Add checker test verifying captures still type-checked alongside DSL validation
- [x] 5.7 Add checker test verifying FileRef blocks are skipped
- [x] 5.8 Verify `cargo test --workspace` passes with all new and existing tests
