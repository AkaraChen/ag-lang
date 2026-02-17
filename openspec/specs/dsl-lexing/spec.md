## ADDED Requirements

### Requirement: At-sign token emission

The lexer SHALL emit an `At` token when encountering `@` in the source. The `At` token is a regular punctuation token — DSL-specific behavior is determined by the parser based on context.

#### Scenario: At-sign in source

- **WHEN** the source contains `@prompt system`
- **THEN** lexer produces `At`, `Ident("prompt")`, `Ident("system")`

### Requirement: DSL raw mode entry

The lexer SHALL support a `enter_dsl_raw_mode()` method callable by the parser. When invoked, the lexer SHALL expect three backticks (`` ``` ``) as the next non-whitespace characters. Upon encountering them, the lexer SHALL emit a `DslBlockStart` token and switch to raw scanning mode.

#### Scenario: Enter raw mode

- **WHEN** the parser calls `enter_dsl_raw_mode()` and the next characters are `` ``` `` followed by a newline
- **THEN** lexer emits `DslBlockStart` and enters raw scanning mode

#### Scenario: Missing backticks after raw mode entry

- **WHEN** the parser calls `enter_dsl_raw_mode()` but the next characters are not `` ``` ``
- **THEN** lexer emits an `Error` token with diagnostic "expected `` ``` `` to open DSL block"

### Requirement: Raw mode text scanning

In raw mode, the lexer SHALL collect characters as `DslText` tokens. The lexer SHALL NOT apply normal tokenization rules (keywords, operators, etc.) to the content. Characters SHALL be accumulated into a single `DslText` token until a capture boundary (`#{`) or block end (`` ``` ``) is encountered.

#### Scenario: Plain DSL text

- **WHEN** raw mode is active and the content is `You are a helpful assistant.\n`
- **THEN** lexer produces `DslText("You are a helpful assistant.\n")`

#### Scenario: DSL text with special characters

- **WHEN** raw mode is active and the content contains `let x = 42; fn foo() { }`
- **THEN** lexer produces `DslText("let x = 42; fn foo() { }")` — no keyword/operator tokenization

### Requirement: Capture boundary detection in raw mode

In raw mode, when the lexer encounters `#{`, it SHALL emit the accumulated `DslText` (if any), then emit a `DslCaptureStart` token, and switch back to normal tokenization mode. Normal mode SHALL continue until a matching `}` is found (respecting nesting of `{}`), at which point the lexer SHALL emit `DslCaptureEnd` and return to raw mode.

#### Scenario: Single capture

- **WHEN** raw mode content is `Hello #{name}!`
- **THEN** lexer produces `DslText("Hello ")`, `DslCaptureStart`, `Ident("name")`, `DslCaptureEnd`, `DslText("!")`

#### Scenario: Capture with complex expression

- **WHEN** raw mode content is `Value: #{items.len() + 1}`
- **THEN** lexer produces `DslText("Value: ")`, `DslCaptureStart`, `Ident("items")`, `Dot`, `Ident("len")`, `LParen`, `RParen`, `Plus`, `IntLiteral("1")`, `DslCaptureEnd`

#### Scenario: Multiple captures

- **WHEN** raw mode content is `#{a} and #{b}`
- **THEN** lexer produces `DslCaptureStart`, `Ident("a")`, `DslCaptureEnd`, `DslText(" and ")`, `DslCaptureStart`, `Ident("b")`, `DslCaptureEnd`

#### Scenario: Nested braces in capture

- **WHEN** raw mode content is `#{obj.map((x) => { x + 1 })}`
- **THEN** lexer tracks brace nesting and only emits `DslCaptureEnd` at the outermost `}` that matches the `#{`

### Requirement: DSL block end detection

In raw mode, the lexer SHALL recognize `` ``` `` at the beginning of a line (with optional leading whitespace) as the block end. Upon encountering it, the lexer SHALL emit any accumulated `DslText`, then emit `DslBlockEnd`, and return to normal tokenization mode. A `` ``` `` that appears mid-line (not at the start) SHALL NOT be treated as block end — it SHALL be included in `DslText`.

#### Scenario: Block end at line start

- **WHEN** raw mode content is `  line1\n  line2\n```\n`
- **THEN** lexer produces `DslText("  line1\n  line2\n")`, `DslBlockEnd`

#### Scenario: Backticks mid-line are not block end

- **WHEN** raw mode content is `use ``` in code\n```\n`
- **THEN** lexer produces `DslText("use ``` in code\n")`, `DslBlockEnd`

#### Scenario: Indented block end

- **WHEN** raw mode content is `  content\n  ```\n`
- **THEN** lexer produces `DslText("  content\n")`, `DslBlockEnd` (leading whitespace before `` ``` `` is allowed)

### Requirement: Hash-sign not followed by brace in raw mode

In raw mode, a `#` character not immediately followed by `{` SHALL be treated as regular text and included in the `DslText` token.

#### Scenario: Standalone hash

- **WHEN** raw mode content is `## Heading\n#{expr}\n`
- **THEN** lexer produces `DslText("## Heading\n")`, `DslCaptureStart`, `Ident("expr")`, `DslCaptureEnd`, `DslText("\n")`

### Requirement: Unterminated DSL block error

If the lexer reaches end-of-file while in raw mode without encountering a closing `` ``` ``, it SHALL emit an `Error` token with diagnostic "unterminated DSL block" and the span covering from the `DslBlockStart` to EOF.

#### Scenario: Missing closing backticks

- **WHEN** the source is `` @prompt system ```\n  content\n `` with no closing `` ``` ``
- **THEN** lexer produces `DslBlockStart`, `DslText("  content\n")`, `Error("unterminated DSL block")`
