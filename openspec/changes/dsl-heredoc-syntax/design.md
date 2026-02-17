## Context

DSL blocks currently use triple backticks (` ``` `) as delimiters. The lexer has a `enter_dsl_raw_mode()` method that the parser calls after recognizing `@ <kind> <name>`. The lexer then expects ` ``` `, emits `DslBlockStart`, scans raw text (with `#{ }` captures), and ends on ` ``` ` at a line start. This works well for the language but creates friction when embedding AG code in Markdown documentation.

## Goals / Non-Goals

**Goals:**
- Replace triple backtick delimiters with heredoc-style `<<LABEL ... LABEL` syntax
- Keep all existing DSL semantics intact: raw mode scanning, `#{ }` captures, `from` file references
- Maintain the same token types (`DslBlockStart`, `DslBlockEnd`, `DslText`, `DslCaptureStart`, `DslCaptureEnd`)
- Allow user-chosen heredoc labels (any valid identifier)

**Non-Goals:**
- Changing capture syntax (`#{ }`) — remains unchanged
- Adding indented heredoc variants (like shell `<<-`) — not needed for now
- Supporting quoted labels (like shell `<<'EOF'`) — not needed
- Changing the `from` file reference form — unaffected

## Decisions

### 1. Heredoc label is any valid identifier

The opening delimiter is `<<` followed by an identifier (the label). The closing delimiter is that same identifier at the start of a line (with optional leading whitespace). This mirrors how the lexer already handles line-start detection for backticks.

**Rationale**: Using identifiers reuses existing lexer infrastructure. Common choices will be `EOF`, `END`, `PROMPT`, etc., but any identifier works. This keeps the language flexible.

**Alternatives considered**:
- Fixed keyword (e.g., always `end`) — less flexible, could conflict with future keywords
- Sigil-delimited (e.g., `---`) — still conflicts with some formats

### 2. Lexer stores the label during raw mode

When `enter_dsl_raw_mode()` is called, the lexer reads `<<` and the label identifier, stores the label string, and uses it to detect block end. The `is_backticks_at_line_start()` method becomes `is_heredoc_label_at_line_start()`, checking for the stored label instead of backticks.

**Rationale**: Minimal change to the existing state machine. The lexer already has a raw mode flag; adding a label string is trivial.

### 3. Token types remain unchanged

`DslBlockStart` and `DslBlockEnd` are reused. The parser doesn't need to know whether the block was delimited by backticks or heredoc — the token stream is identical.

**Rationale**: Minimizes parser changes. The parser already works with these tokens; only the lexer's detection logic changes.

### 4. Label matching is exact and case-sensitive

The closing label must match the opening label exactly (case-sensitive, no partial matches). The label must be the only non-whitespace content on its line.

**Rationale**: Prevents ambiguity. If the DSL content happens to contain the label as a substring of other text, it won't accidentally close the block.

## Risks / Trade-offs

- **[Breaking change]** All existing `.ag` files must be updated → Mitigation: This is a pre-1.0 language; no external users yet. Update all examples and tests in the same change.
- **[Label choice confusion]** Users might not know what label to use → Mitigation: Convention examples in docs (`EOF` as default). The parser accepts any identifier.
- **[Label in content]** If DSL content contains the label at line start → Mitigation: User picks a different label. This is the standard heredoc trade-off.
