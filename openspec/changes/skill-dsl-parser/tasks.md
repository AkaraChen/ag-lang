## 1. Crate Setup

- [ ] 1.1 Create `crates/ag-dsl-skill/` crate with `Cargo.toml`, depend on `ag-dsl-core`
- [ ] 1.2 Add `ag-dsl-skill` to workspace members in root `Cargo.toml`
- [ ] 1.3 Create `src/lib.rs` with public module declarations (`ast`, `lexer`, `parser`, `validator`)
- [ ] 1.4 `cargo build` to verify crate compiles

## 2. AST Definitions

- [ ] 2.1 Define `SkillTemplate { name: String, description: Option<String>, input_fields: Vec<SkillField>, steps: Vec<SkillStep>, output_fields: Vec<SkillField> }`
- [ ] 2.2 Define `SkillField { name: String, type_name: String, default: Option<String> }`
- [ ] 2.3 Define `SkillStep { number: u32, text: String, captures: Vec<String> }`
- [ ] 2.4 Add `has_description`, `has_input`, `has_steps`, `has_output` tracking flags to parser state (for duplicate detection)

## 3. Lexer

- [ ] 3.1 Define `SkillToken` enum: `DirectiveDescription`, `DirectiveInput`, `DirectiveSteps`, `DirectiveOutput`, `Text(String)`, `Capture(usize)`, `BraceOpen`, `BraceClose`, `Colon`, `Equals`, `StringLiteral(String)`, `NumberLiteral(f64)`, `ArrayOpen`, `ArrayClose`, `Ident(String)`, `Eof`
- [ ] 3.2 Implement `lex(parts: &[DslPart]) -> Vec<SkillToken>` entry point
- [ ] 3.3 Implement Text segment directive recognition: line-start `@` + known keyword (`description`, `input`, `steps`, `output`)
- [ ] 3.4 Implement `@description` lexing: emit `DirectiveDescription`, then lex the following quoted string literal with escape handling
- [ ] 3.5 Implement `@input` / `@output` lexing: emit directive token, then lex `{...}` brace block with idents, colons, equals, string/number literals, array brackets
- [ ] 3.6 Implement `@steps` lexing: emit `DirectiveSteps`, then collect all following text as `Text` tokens until next line-start `@` directive
- [ ] 3.7 Implement `DslPart::Capture` passthrough as `Capture(index)` tokens
- [ ] 3.8 Write lexer unit tests covering all spec scenarios: directive recognition, field blocks, steps text, captures, unknown-@ as text

## 4. Parser

- [ ] 4.1 Implement `parse(tokens: &[SkillToken], name: &str) -> Result<SkillTemplate, Vec<Diagnostic>>` entry point
- [ ] 4.2 Implement directive dispatch loop: match on directive tokens, route to sub-parsers
- [ ] 4.3 Implement `parse_description()`: expect `StringLiteral` after `DirectiveDescription`, error if missing
- [ ] 4.4 Implement `parse_field_block()`: parse `{ name: type (= default)?, ... }` into `Vec<SkillField>`, shared by @input and @output
- [ ] 4.5 Implement array type parsing in fields: `[ident]` -> `type_name = "[ident]"`
- [ ] 4.6 Implement default value parsing: `= ident`, `= StringLiteral`, `= NumberLiteral`
- [ ] 4.7 Implement `parse_steps()`: collect `Text` and `Capture` tokens, split on numbered line prefixes (`N.`), extract `#{}` capture names
- [ ] 4.8 Implement parser error diagnostics: missing braces, missing string after @description, missing type in field
- [ ] 4.9 Track directive occurrence counts for duplicate detection (pass to validator)
- [ ] 4.10 Write parser unit tests covering all spec scenarios: full template, fields with defaults, steps with captures, error cases

## 5. Validator

- [ ] 5.1 Implement `validate(template: &SkillTemplate, directive_counts: &DirectiveCounts) -> Vec<Diagnostic>`
- [ ] 5.2 Check `@description` required: error if `description` is `None`
- [ ] 5.3 Check `@input` required: error if no `@input` directive was parsed
- [ ] 5.4 Check `@steps` required: error if no `@steps` directive was parsed
- [ ] 5.5 Check duplicate directives: error if any directive count > 1
- [ ] 5.6 Write validator unit tests: missing required directives, duplicate directives, valid template passes

## 6. Integration Tests

- [ ] 6.1 End-to-end test: full skill DSL text -> lex -> parse -> validate -> assert SkillTemplate fields
- [ ] 6.2 End-to-end test: skill with defaults and array types -> verify SkillField values
- [ ] 6.3 End-to-end test: skill with captures in steps -> verify SkillStep captures extracted
- [ ] 6.4 End-to-end test: missing required directive -> verify error diagnostic
- [ ] 6.5 End-to-end test: duplicate directive -> verify error diagnostic
