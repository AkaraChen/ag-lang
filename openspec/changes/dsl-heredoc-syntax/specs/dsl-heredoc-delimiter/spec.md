## ADDED Requirements

### Requirement: Heredoc delimiter syntax

DSL blocks SHALL use heredoc-style delimiters. The opening delimiter SHALL be `<<` immediately followed by an identifier (the label). The closing delimiter SHALL be the same identifier appearing at the start of a line (with optional leading whitespace). The label SHALL be case-sensitive and match exactly.

#### Scenario: Basic heredoc DSL block

- **WHEN** the input is `@prompt greeting <<EOF\nHello, world!\nEOF\n`
- **THEN** parser produces `DslBlock { kind: "prompt", name: "greeting", content: Inline([Text("Hello, world!\n")]) }`

#### Scenario: Custom label

- **WHEN** the input is `@prompt sys <<PROMPT\n@role system\nYou are helpful.\nPROMPT\n`
- **THEN** parser produces `DslBlock` with `content: Inline([Text("@role system\nYou are helpful.\n")])` — the label `PROMPT` is accepted

#### Scenario: Label is case-sensitive

- **WHEN** the input is `@prompt sys <<EOF\ncontent\neof\nEOF\n`
- **THEN** parser produces `DslBlock` with `content: Inline([Text("content\neof\n")])` — lowercase `eof` is treated as content, only `EOF` closes the block

### Requirement: Heredoc with captures

DSL blocks using heredoc syntax SHALL support `#{ expr }` captures identically to the previous backtick syntax. Capture behavior SHALL be unchanged.

#### Scenario: Heredoc block with captures

- **WHEN** the input is `@prompt greet <<EOF\nHello #{name}, welcome!\nEOF\n`
- **THEN** parser produces `DslBlock` with `content: Inline([Text("Hello "), Capture(Ident("name")), Text(", welcome!\n")])`

### Requirement: Label at line start only

The closing label SHALL only be recognized at the start of a line (with optional leading whitespace). The label appearing mid-line SHALL be treated as regular DSL text content.

#### Scenario: Label mid-line is not block end

- **WHEN** the input is `@prompt sys <<EOF\nuse EOF in text\nEOF\n`
- **THEN** parser produces `DslBlock` with `content: Inline([Text("use EOF in text\n")])` — only the `EOF` at line start closes the block

#### Scenario: Indented closing label

- **WHEN** the input is `@prompt sys <<EOF\n  content\n  EOF\n`
- **THEN** parser produces `DslBlock` with `content: Inline([Text("  content\n")])` — leading whitespace before closing label is allowed
