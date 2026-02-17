## ADDED Requirements

### Requirement: Token types

The lexer SHALL produce tokens of the following categories:

- **Keywords**: `fn`, `let`, `const`, `mut`, `if`, `else`, `for`, `in`, `of`, `while`, `match`, `ret`, `yield`, `await`, `async`, `import`, `export`, `from`, `as`, `type`, `struct`, `enum`, `impl`, `pub`, `self`, `true`, `false`, `nil`, `use`, `with`, `on`, `_`, `try`, `catch`
- **Identifiers**: sequences starting with `[a-zA-Z_]` followed by `[a-zA-Z0-9_]`
- **Number literals**: integers (`42`), floats (`3.14`), exponent notation (`1e10`, `2.5e-3`)
- **String literals**: double-quoted (`"hello"`), single-quoted (`'hello'`)
- **Template strings**: backtick-delimited with `${expr}` interpolation
- **Operators**: `+`, `-`, `*`, `/`, `%`, `**`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!`, `|>`, `??`, `?.`, `=`, `+=`, `-=`, `*=`, `/=`, `=>`, `->`, `::`, `@`, `..`, `...`
- **Punctuation**: `{`, `}`, `(`, `)`, `[`, `]`, `<`, `>`, `,`, `;`, `:`, `.`, `?`
- **Comments**: line (`//`), block (`/* */`), doc (`///`)
- **EOF**: end-of-input marker

Each token SHALL carry a `Span` (byte offset `start` and `end`) and the source text slice.

#### Scenario: Keyword vs identifier

- **WHEN** the source contains `let x = fn_name`
- **THEN** lexer produces tokens: `Let`, `Ident("x")`, `Eq`, `Ident("fn_name")`

#### Scenario: Identifier starting with keyword prefix

- **WHEN** the source contains `letter`
- **THEN** lexer produces `Ident("letter")`, NOT `Let` followed by `Ident("ter")`

### Requirement: Number literal lexing

The lexer SHALL recognize integer literals, float literals, and exponent notation. Integers SHALL be sequences of digits. Floats SHALL be digits with a single `.` followed by more digits. Exponent notation SHALL be a number followed by `e` or `E`, optional `+`/`-`, and digits.

#### Scenario: Integer literal

- **WHEN** the source contains `42`
- **THEN** lexer produces `IntLiteral("42")`

#### Scenario: Float literal

- **WHEN** the source contains `3.14`
- **THEN** lexer produces `FloatLiteral("3.14")`

#### Scenario: Exponent notation

- **WHEN** the source contains `2.5e-3`
- **THEN** lexer produces `FloatLiteral("2.5e-3")`

### Requirement: String literal lexing

The lexer SHALL recognize double-quoted and single-quoted string literals. Escape sequences (`\\`, `\"`, `\'`, `\n`, `\t`, `\r`, `\0`) SHALL be recognized within strings. Unterminated strings SHALL produce an error token with a diagnostic.

#### Scenario: Double-quoted string

- **WHEN** the source contains `"hello world"`
- **THEN** lexer produces `StringLiteral("hello world")`

#### Scenario: Escape sequences

- **WHEN** the source contains `"line1\nline2"`
- **THEN** lexer produces `StringLiteral("line1\nline2")` preserving the escape

#### Scenario: Unterminated string

- **WHEN** the source contains `"hello` with no closing quote
- **THEN** lexer produces an `Error` token with diagnostic "unterminated string literal"

### Requirement: Template string lexing

The lexer SHALL recognize backtick-delimited template strings. When `${` is encountered inside a template, the lexer SHALL produce a `TemplateHead` or `TemplateMiddle` token, then lex the interpolated expression tokens, then continue with `TemplateMiddle` or `TemplateTail`. A template string without interpolations SHALL produce a single `TemplateNoSub` token. Nested template strings (template inside `${}`) SHALL be supported via a depth-tracking stack.

#### Scenario: Simple template string

- **WHEN** the source contains `` `hello world` ``
- **THEN** lexer produces `TemplateNoSub("hello world")`

#### Scenario: Template with interpolation

- **WHEN** the source contains `` `hello ${name}!` ``
- **THEN** lexer produces `TemplateHead("hello ")`, `Ident("name")`, `TemplateTail("!")`

#### Scenario: Multiple interpolations

- **WHEN** the source contains `` `${a} + ${b} = ${c}` ``
- **THEN** lexer produces `TemplateHead("")`, `Ident("a")`, `TemplateMiddle(" + ")`, `Ident("b")`, `TemplateMiddle(" = ")`, `Ident("c")`, `TemplateTail("")`

#### Scenario: Nested template strings

- **WHEN** the source contains `` `outer ${`inner ${x}`} end` ``
- **THEN** lexer correctly nests and produces tokens for both the outer and inner template strings

### Requirement: Comment handling

The lexer SHALL recognize line comments (`//`), block comments (`/* */`), and doc comments (`///`). Comments SHALL be emitted as tokens (not discarded), each carrying the comment text and span. Block comments SHALL support nesting (`/* outer /* inner */ still outer */`).

#### Scenario: Line comment

- **WHEN** the source contains `x // this is a comment\ny`
- **THEN** lexer produces `Ident("x")`, `LineComment("// this is a comment")`, `Ident("y")`

#### Scenario: Doc comment

- **WHEN** the source contains `/// Docs for next item\nfn foo() {}`
- **THEN** lexer produces `DocComment("/// Docs for next item")` followed by `Fn`, `Ident("foo")`, ...

#### Scenario: Block comment

- **WHEN** the source contains `x /* block */ y`
- **THEN** lexer produces `Ident("x")`, `BlockComment("/* block */")`, `Ident("y")`

### Requirement: Multi-character operator disambiguation

The lexer SHALL correctly disambiguate multi-character operators. `==` SHALL NOT be lexed as two `=`. `|>` SHALL NOT be lexed as `|` then `>`. `?.` SHALL NOT be lexed as `?` then `.`. The lexer SHALL use maximal munch (longest match) for operator tokenization.

#### Scenario: Pipe operator

- **WHEN** the source contains `a |> b`
- **THEN** lexer produces `Ident("a")`, `Pipe`, `Ident("b")`

#### Scenario: Arrow operators

- **WHEN** the source contains `=> ->`
- **THEN** lexer produces `FatArrow`, `ThinArrow`

#### Scenario: Optional chaining vs ternary-dot

- **WHEN** the source contains `x?.y`
- **THEN** lexer produces `Ident("x")`, `QuestionDot`, `Ident("y")`

### Requirement: Whitespace and newline handling

The lexer SHALL skip whitespace (spaces, tabs, carriage returns, newlines) between tokens. Whitespace SHALL NOT produce tokens. The lexer SHALL track positions correctly across newlines for accurate span reporting.

#### Scenario: Whitespace between tokens

- **WHEN** the source contains `let   x  =  42`
- **THEN** lexer produces `Let`, `Ident("x")`, `Eq`, `IntLiteral("42")` with no whitespace tokens

### Requirement: Error recovery

The lexer SHALL NOT halt on the first error. When an unrecognized character is encountered, the lexer SHALL produce an `Error` token with a diagnostic message and the offending character's span, then continue lexing from the next character.

#### Scenario: Unrecognized character

- **WHEN** the source contains `let x = 42 § y`
- **THEN** lexer produces `Let`, `Ident("x")`, `Eq`, `IntLiteral("42")`, `Error("§")`, `Ident("y")`
