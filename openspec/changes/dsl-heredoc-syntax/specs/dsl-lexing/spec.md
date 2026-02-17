## MODIFIED Requirements

### Requirement: DSL raw mode entry

The lexer SHALL support a `enter_dsl_raw_mode()` method callable by the parser. When invoked, the lexer SHALL expect `<<` followed by an identifier (the heredoc label) as the next non-whitespace characters. Upon encountering them, the lexer SHALL store the label, emit a `DslBlockStart` token, and switch to raw scanning mode.

#### Scenario: Enter raw mode with heredoc

- **WHEN** the parser calls `enter_dsl_raw_mode()` and the next characters are `<<EOF` followed by a newline
- **THEN** lexer stores label `"EOF"`, emits `DslBlockStart`, and enters raw scanning mode

#### Scenario: Missing heredoc label after raw mode entry

- **WHEN** the parser calls `enter_dsl_raw_mode()` but the next characters are not `<<` followed by an identifier
- **THEN** lexer emits an `Error` token with diagnostic "expected `<<LABEL` to open DSL block"

### Requirement: DSL block end detection

In raw mode, the lexer SHALL recognize the stored heredoc label at the beginning of a line (with optional leading whitespace) as the block end. Upon encountering it, the lexer SHALL emit any accumulated `DslText`, then emit `DslBlockEnd`, and return to normal tokenization mode. The label appearing mid-line SHALL NOT be treated as block end — it SHALL be included in `DslText`.

#### Scenario: Block end at line start

- **WHEN** raw mode is active with label `"EOF"` and content is `  line1\n  line2\nEOF\n`
- **THEN** lexer produces `DslText("  line1\n  line2\n")`, `DslBlockEnd`

#### Scenario: Label mid-line is not block end

- **WHEN** raw mode is active with label `"EOF"` and content is `use EOF in code\nEOF\n`
- **THEN** lexer produces `DslText("use EOF in code\n")`, `DslBlockEnd`

#### Scenario: Indented block end

- **WHEN** raw mode is active with label `"EOF"` and content is `  content\n  EOF\n`
- **THEN** lexer produces `DslText("  content\n")`, `DslBlockEnd` (leading whitespace before label is allowed)

### Requirement: Unterminated DSL block error

If the lexer reaches end-of-file while in raw mode without encountering a closing heredoc label, it SHALL emit an `Error` token with diagnostic "unterminated DSL block" and the span covering from the `DslBlockStart` to EOF.

#### Scenario: Missing closing label

- **WHEN** the source is `@prompt system <<EOF\n  content\n` with no closing `EOF`
- **THEN** lexer produces `DslBlockStart`, `DslText("  content\n")`, `Error("unterminated DSL block")`
