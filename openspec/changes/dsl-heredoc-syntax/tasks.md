## 1. Lexer Changes

- [x] 1.1 Update `enter_dsl_raw_mode()` to expect `<<LABEL` instead of triple backticks, store the label string
- [x] 1.2 Replace `is_backticks_at_line_start()` with `is_heredoc_label_at_line_start()` that checks for the stored label
- [x] 1.3 Update raw mode scanning loop to use label-based end detection
- [x] 1.4 Update error messages: "expected `<<LABEL` to open DSL block", "unterminated DSL block"
- [x] 1.5 Update all lexer DSL tests to use heredoc syntax

## 2. Parser Changes

- [x] 2.1 Update `parse_dsl_block()` to detect `<<` + identifier instead of backticks for inline form
- [x] 2.2 Update error diagnostic: "expected `<<LABEL` or `from` after `@<kind> <name>`"
- [x] 2.3 Update all parser DSL tests to use heredoc syntax

## 3. Language Spec

- [x] 3.1 Update `spec/lang.md` DSL block section (syntax description, grammar, examples)

## 4. Codegen & DSL Handler Tests

- [x] 4.1 Update DSL block test source strings in `ag-codegen` tests
- [x] 4.2 Update test source strings in `ag-dsl-prompt` tests
- [x] 4.3 Update test source strings in other DSL crate tests (`ag-dsl-agent`, `ag-dsl-skill`, `ag-dsl-server`, `ag-dsl-component`)

## 5. Examples & Documentation

- [x] 5.1 Update all `.ag` files in `examples/` to use heredoc syntax
- [x] 5.2 Update README.md code examples
- [x] 5.3 Update CLAUDE.md if it contains backtick DSL examples

## 6. Integration Verification

- [x] 6.1 Run `cargo test --workspace` and verify all tests pass
- [x] 6.2 Run `cargo run -p ag-cli -- build examples/simple-agent/app.ag` and verify compilation
