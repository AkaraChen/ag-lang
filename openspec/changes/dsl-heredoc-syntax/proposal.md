## Why

The current DSL block delimiter (triple backticks ` ``` `) clashes with Markdown code fences, causing rendering and escaping issues in documentation, README files, and any tool that interprets Markdown. Switching to heredoc-style delimiters (`<<LABEL ... LABEL`) eliminates this conflict, improves readability in docs, and aligns with a well-known convention from shell scripting and other languages.

## What Changes

- **BREAKING**: DSL block syntax changes from triple backticks to heredoc-style delimiters:
  ```
  // Before
  @prompt greeting ```
  @role system
  Hello!
  ```

  // After
  @prompt greeting <<EOF
  @role system
  Hello!
  EOF
  ```
- Lexer: replace backtick-based raw mode entry/exit with heredoc label scanning
- Parser: update DSL block parsing to expect `<<LABEL` opener and `LABEL` closer
- The heredoc label is user-chosen (e.g., `EOF`, `PROMPT`, `END`, any identifier) and the closing label must match exactly at the start of a line
- `#{ expr }` captures inside DSL blocks continue to work unchanged
- The `from` file-reference form (`@component Foo from "./foo.tsx"`) is unaffected

## Capabilities

### New Capabilities
- `dsl-heredoc-delimiter`: Heredoc-style delimiter syntax (`<<LABEL ... LABEL`) for DSL block boundaries, replacing triple backticks

### Modified Capabilities
- `dsl-lexing`: Raw mode entry/exit changes from backtick detection to heredoc label detection
- `dsl-parsing`: DSL block parsing updated to expect heredoc syntax instead of backticks

## Impact

- **Lexer** (`ag-lexer`): `enter_dsl_raw_mode()`, raw scanning loop, `is_backticks_at_line_start()` all rewritten
- **Parser** (`ag-parser`): `parse_dsl_block()` updated for new syntax
- **All tests**: Lexer and parser DSL tests updated (~50+ test strings)
- **Language spec** (`spec/lang.md`): DSL block section and grammar rewritten
- **Examples**: All `.ag` files updated to new syntax
- **Documentation**: README, CLAUDE.md examples updated
- **Codegen tests**: All test source strings using DSL blocks updated
- **DSL handler tests**: Test strings in ag-dsl-prompt and other DSL crates
