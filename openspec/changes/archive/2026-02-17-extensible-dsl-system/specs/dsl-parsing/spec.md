## ADDED Requirements

### Requirement: DSL block triggered by top-level @

The parser SHALL recognize a DSL block when it encounters `@` at top-level followed by an identifier (DSL kind), another identifier (block name), and either `` ``` `` (inline block) or `from` (file reference). The parser SHALL NOT maintain a list of valid DSL kind names — any identifier after `@` is accepted.

#### Scenario: Inline DSL block

- **WHEN** the input is `@prompt system ``` content ```\n`
- **THEN** parser produces `DslBlock { kind: "prompt", name: "system", content: Inline(...) }`

#### Scenario: File reference DSL block

- **WHEN** the input is `@component Button from "./button.tsx"`
- **THEN** parser produces `DslBlock { kind: "component", name: "Button", content: FileRef("./button.tsx") }`

#### Scenario: Unknown DSL kind accepted

- **WHEN** the input is `@graphql GetUsers ``` query { users { id } } ```\n`
- **THEN** parser produces `DslBlock { kind: "graphql", name: "GetUsers", content: Inline(...) }` without error

### Requirement: Inline DSL block parsing

When parsing an inline DSL block, the parser SHALL instruct the lexer to enter raw mode. It SHALL then collect `DslText` and `DslCaptureStart`/`DslCaptureEnd` bounded expression sequences into a `Vec<DslPart>`. Each `DslText` token SHALL become `DslPart::Text`. Each capture sequence (tokens between `DslCaptureStart` and `DslCaptureEnd`) SHALL be parsed as an AgentScript expression and become `DslPart::Capture(expr)`.

#### Scenario: Inline block with captures

- **WHEN** the input is `@prompt sys ``` Hello #{name}, you have #{count} messages. ```\n`
- **THEN** parser produces `DslBlock` with `content: Inline([Text("Hello "), Capture(Ident("name")), Text(", you have "), Capture(Ident("count")), Text(" messages.")])`

#### Scenario: Inline block without captures

- **WHEN** the input is `@css styles ``` .btn { color: red; } ```\n`
- **THEN** parser produces `DslBlock` with `content: Inline([Text(".btn { color: red; }")])`

### Requirement: Capture expression parsing

Within a DSL capture (`#{...}`), the parser SHALL parse a single AgentScript expression using the standard expression parser. Statements SHALL NOT be allowed — only expressions. If the content is not a valid expression, the parser SHALL emit a diagnostic.

#### Scenario: Valid capture expression

- **WHEN** a capture contains `#{items.len() + 1}`
- **THEN** parser produces `Capture(Binary(Add, Call(Member(Ident("items"), "len"), []), IntLiteral(1)))`

#### Scenario: Invalid capture content

- **WHEN** a capture contains `#{let x = 1}`
- **THEN** parser produces a diagnostic "expected expression in capture, found statement"

### Requirement: File reference parsing

When the parser encounters `from` after `@ <kind> <name>`, it SHALL expect a string literal as the file path. The result SHALL be `DslContent::FileRef(path)`. The path string SHALL be preserved as-is (no resolution at parse time).

#### Scenario: File reference with double-quoted path

- **WHEN** the input is `@component Card from "./card.tsx"`
- **THEN** parser produces `DslBlock { kind: "component", name: "Card", content: FileRef("./card.tsx") }`

#### Scenario: Missing path after from

- **WHEN** the input is `@component Card from`
- **THEN** parser produces diagnostic "expected string literal after `from`"

#### Scenario: Non-string after from

- **WHEN** the input is `@component Card from card.tsx`
- **THEN** parser produces diagnostic "expected string literal after `from`, found identifier"

### Requirement: DSL block as top-level item

The parser SHALL include `DslBlock` as a valid top-level `Item` variant. DSL blocks SHALL be allowed at module scope alongside function declarations, struct declarations, imports, etc.

#### Scenario: Mixed module items

- **WHEN** the input is `import { x } from "y"\n@prompt sys ``` hello ```\nfn foo() -> int { 1 }`
- **THEN** parser produces a `Module` with three items: `Import`, `DslBlock`, `FnDecl`

### Requirement: @ not followed by DSL pattern

If the parser encounters `@` at top-level but the subsequent tokens do not match the DSL pattern (`@ <ident> <ident> (``` | from)`), the parser SHALL emit a diagnostic. This covers cases like `@ 42` or `@` at EOF.

#### Scenario: Invalid @ usage

- **WHEN** the input is `@42`
- **THEN** parser produces diagnostic "expected identifier after `@`"

#### Scenario: @ with only one identifier

- **WHEN** the input is `@prompt\nfn foo() {}`
- **THEN** parser produces diagnostic "expected DSL block name after `@prompt`"

#### Scenario: @ with identifiers but no body

- **WHEN** the input is `@prompt system\nfn foo() {}`
- **THEN** parser produces diagnostic "expected `` ``` `` or `from` after `@prompt system`"
