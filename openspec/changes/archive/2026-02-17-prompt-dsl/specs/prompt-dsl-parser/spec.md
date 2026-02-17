## ADDED Requirements

### Requirement: Prompt lexer accepts DslPart input

The prompt lexer SHALL accept `Vec<DslPart>` as input (not raw source code). It SHALL scan `DslPart::Text` segments for directive syntax and pass through `DslPart::Capture` as `Capture(index)` tokens referencing the original capture by position.

#### Scenario: Text with directive and capture

- **WHEN** input is `[Text("@role system\nHello "), Capture(expr, span), Text("!\n")]`
- **THEN** lexer produces `DirectiveRole("system")`, `Text("Hello ")`, `Capture(1)`, `Text("!\n")`

### Requirement: Directive recognition

The lexer SHALL recognize the following directives when `@` appears at the beginning of a line followed by a known keyword: `@role`, `@model`, `@examples`, `@output`, `@constraints`, `@messages`. An `@` not at line start, or followed by an unknown keyword, SHALL be treated as regular text.

#### Scenario: Known directive at line start

- **WHEN** a text segment contains `\n@role user\n`
- **THEN** lexer produces `DirectiveRole("user")`

#### Scenario: Unknown @ treated as text

- **WHEN** a text segment contains `email me @alice\n`
- **THEN** lexer produces `Text("email me @alice\n")`

#### Scenario: @ mid-line is text

- **WHEN** a text segment contains `contact @support for help\n`
- **THEN** lexer produces `Text("contact @support for help\n")`

### Requirement: @role directive parsing

The lexer SHALL recognize `@role <name>` where `<name>` is `system`, `user`, `assistant`, or any identifier. The role name SHALL be captured in the `DirectiveRole(name)` token. All text following the directive line until the next directive or end of input SHALL be treated as that role's content.

#### Scenario: Standard roles

- **WHEN** input text contains `@role system\n@role user\n@role assistant\n`
- **THEN** lexer produces `DirectiveRole("system")`, `DirectiveRole("user")`, `DirectiveRole("assistant")`

#### Scenario: Custom role

- **WHEN** input text contains `@role tool\nTool output here\n`
- **THEN** lexer produces `DirectiveRole("tool")`, `Text("Tool output here\n")`

### Requirement: @model directive parsing

The lexer SHALL recognize `@model` followed by model names separated by `|`. Each model name SHALL be an identifier or hyphenated string. The `|` separator SHALL indicate fallback order.

#### Scenario: Single model

- **WHEN** input text contains `@model claude-sonnet\n`
- **THEN** lexer produces `DirectiveModel`, `Ident("claude-sonnet")`

#### Scenario: Multiple models with fallback

- **WHEN** input text contains `@model claude-sonnet | gpt-4o | deepseek-chat\n`
- **THEN** lexer produces `DirectiveModel`, `Ident("claude-sonnet")`, `Pipe`, `Ident("gpt-4o")`, `Pipe`, `Ident("deepseek-chat")`

### Requirement: @examples directive parsing

The lexer SHALL recognize `@examples` followed by a `{...}` block containing `role: "content"` pairs. Multiple `@examples` blocks SHALL be allowed. String values SHALL support escape sequences.

#### Scenario: Single example block

- **WHEN** input text contains `@examples {\n  user: "hello"\n  assistant: "hi"\n}\n`
- **THEN** lexer produces `DirectiveExamples`, `BraceOpen`, `Ident("user")`, `Colon`, `StringLiteral("hello")`, `Ident("assistant")`, `Colon`, `StringLiteral("hi")`, `BraceClose`

#### Scenario: Multiple example blocks

- **WHEN** input text contains two `@examples { ... }` blocks
- **THEN** lexer produces two sequences of `DirectiveExamples` followed by their content tokens

### Requirement: @output directive parsing

The lexer SHALL recognize `@output` followed by either a capture `#{expr}` (referencing a host type) or an inline object `{ field: type, ... }`.

#### Scenario: Output with capture reference

- **WHEN** input parts are `[Text("@output "), Capture(schema_expr, span)]`
- **THEN** lexer produces `DirectiveOutput`, `Capture(1)`

#### Scenario: Output with inline schema

- **WHEN** input text contains `@output {\n  answer: str\n  confidence: num\n}\n`
- **THEN** lexer produces `DirectiveOutput`, `BraceOpen`, `Ident("answer")`, `Colon`, `Ident("str")`, `Ident("confidence")`, `Colon`, `Ident("num")`, `BraceClose`

### Requirement: @constraints directive parsing

The lexer SHALL recognize `@constraints` followed by a `{...}` block containing `key: value` pairs where values can be numbers, strings, booleans, or arrays.

#### Scenario: Constraints block

- **WHEN** input text contains `@constraints {\n  temperature: 0.7\n  max_tokens: 4096\n  stop: ["\n\n"]\n}\n`
- **THEN** lexer produces `DirectiveConstraints`, `BraceOpen`, `Ident("temperature")`, `Colon`, `NumberLiteral(0.7)`, `Ident("max_tokens")`, `Colon`, `NumberLiteral(4096.0)`, `Ident("stop")`, `Colon`, `ArrayOpen`, `StringLiteral("\n\n")`, `ArrayClose`, `BraceClose`

### Requirement: @messages directive parsing

The lexer SHALL recognize `@messages` followed by a capture `#{expr}`.

#### Scenario: Messages with capture

- **WHEN** input parts are `[Text("@messages "), Capture(history_expr, span)]`
- **THEN** lexer produces `DirectiveMessages`, `Capture(1)`

### Requirement: Parser produces PromptTemplate AST

The parser SHALL accept `Vec<PromptToken>` and produce a `PromptTemplate` AST containing: an ordered list of `PromptSection` nodes, optional `ModelSpec`, optional `OutputSpec`, and optional `Constraints`.

#### Scenario: Full prompt template

- **WHEN** tokens represent a prompt with @model, @role system, text, captures, @examples, @constraints
- **THEN** parser produces `PromptTemplate` with `model: Some(...)`, `sections: [Role { system, ... }, Examples(...)]`, `constraints: Some(...)`

### Requirement: Default role for text before first @role

If text or captures appear before any `@role` directive, the parser SHALL assign them to an implicit `@role system` section.

#### Scenario: No explicit role

- **WHEN** the prompt starts with `You are a helpful assistant.\n` without `@role`
- **THEN** parser wraps this in `PromptSection::Role { role: System, body: [Text("You are a helpful assistant.\n")] }`

### Requirement: Multiple role sections form message sequence

The parser SHALL allow multiple `@role` directives in a single prompt. Each `@role` starts a new `PromptSection::Role`. The sections SHALL be ordered as they appear in the source, forming a chat message sequence.

#### Scenario: Multi-role prompt

- **WHEN** input has `@role system`, text, `@role user`, capture
- **THEN** parser produces `sections: [Role { system, [Text("...")] }, Role { user, [Capture(0)] }]`

### Requirement: Parser error on invalid structure

The parser SHALL produce a diagnostic error for structurally invalid prompts: empty prompt (no content at all), `@examples` without `{...}` block, `@constraints` without `{...}` block, `@messages` without capture.

#### Scenario: Examples without braces

- **WHEN** tokens are `DirectiveExamples` followed by `Text("...")`
- **THEN** parser produces error "expected `{` after @examples"

#### Scenario: Messages without capture

- **WHEN** tokens are `DirectiveMessages` followed by `Text("...")`
- **THEN** parser produces error "expected capture expression after @messages"

### Requirement: Validator checks prompt structure

The validator SHALL check the parsed `PromptTemplate` for semantic issues: warns if no `@role` is defined (using implicit system), errors if `@model` is specified more than once, errors if `@output` is specified more than once, errors if `@constraints` is specified more than once.

#### Scenario: Duplicate model directive

- **WHEN** the parsed prompt has two `ModelSpec` entries
- **THEN** validator produces error "duplicate @model directive"

#### Scenario: No role warning

- **WHEN** the parsed prompt has only text with no `@role`
- **THEN** validator produces warning "no @role directive; content assigned to implicit system role"
