## ADDED Requirements

### Requirement: Skill lexer accepts DslPart input

The skill lexer SHALL accept `Vec<DslPart>` as input (not raw source code). It SHALL scan `DslPart::Text` segments for directive syntax and pass through `DslPart::Capture` as `Capture(index)` tokens referencing the original capture by position.

#### Scenario: Text with directive and capture

- **WHEN** input is `[Text("@description "), Capture(expr, span)]`
- **THEN** lexer produces `DirectiveDescription`, `Capture(0)`

### Requirement: Directive recognition

The lexer SHALL recognize the following directives when `@` appears at the beginning of a line followed by a known keyword: `@description`, `@input`, `@steps`, `@output`. An `@` not at line start, or followed by an unknown keyword, SHALL be treated as regular text.

#### Scenario: Known directive at line start

- **WHEN** a text segment contains `@description "Summarize text"\n`
- **THEN** lexer produces `DirectiveDescription`, `StringLiteral("Summarize text")`

#### Scenario: Unknown @ treated as text

- **WHEN** a text segment contains `email @admin for access\n`
- **THEN** lexer produces `Text("email @admin for access\n")`

#### Scenario: @ mid-line is text

- **WHEN** a text segment contains `contact @support for help\n`
- **THEN** lexer produces `Text("contact @support for help\n")`

### Requirement: @description directive parsing

The lexer SHALL recognize `@description` followed by a quoted string literal. The string content SHALL be captured in a `StringLiteral` token. Escape sequences within the string (`\"`, `\\`, `\n`, `\t`) SHALL be handled.

#### Scenario: Simple description

- **WHEN** input text contains `@description "Refactor code for readability"\n`
- **THEN** lexer produces `DirectiveDescription`, `StringLiteral("Refactor code for readability")`

#### Scenario: Description with escapes

- **WHEN** input text contains `@description "Fix the \"bug\" in parser"\n`
- **THEN** lexer produces `DirectiveDescription`, `StringLiteral("Fix the \"bug\" in parser")`

### Requirement: @input directive parsing

The lexer SHALL recognize `@input` followed by a `{...}` block containing field declarations. Each field declaration SHALL have the form `name: type` with an optional `= default`. Field names and type names SHALL be identifiers. Type names MAY include bracket syntax for arrays (e.g., `[str]`).

#### Scenario: Simple input fields

- **WHEN** input text contains `@input {\n  query: str\n  max_results: int\n}\n`
- **THEN** lexer produces `DirectiveInput`, `BraceOpen`, `Ident("query")`, `Colon`, `Ident("str")`, `Ident("max_results")`, `Colon`, `Ident("int")`, `BraceClose`

#### Scenario: Input field with default value

- **WHEN** input text contains `@input {\n  dry_run: bool = false\n}\n`
- **THEN** lexer produces `DirectiveInput`, `BraceOpen`, `Ident("dry_run")`, `Colon`, `Ident("bool")`, `Equals`, `Ident("false")`, `BraceClose`

#### Scenario: Input field with array type

- **WHEN** input text contains `@input {\n  tags: [str]\n}\n`
- **THEN** lexer produces `DirectiveInput`, `BraceOpen`, `Ident("tags")`, `Colon`, `ArrayOpen`, `Ident("str")`, `ArrayClose`, `BraceClose`

#### Scenario: Input field with string default

- **WHEN** input text contains `@input {\n  language: str = "english"\n}\n`
- **THEN** lexer produces `DirectiveInput`, `BraceOpen`, `Ident("language")`, `Colon`, `Ident("str")`, `Equals`, `StringLiteral("english")`, `BraceClose`

#### Scenario: Input field with numeric default

- **WHEN** input text contains `@input {\n  limit: int = 10\n}\n`
- **THEN** lexer produces `DirectiveInput`, `BraceOpen`, `Ident("limit")`, `Colon`, `Ident("int")`, `Equals`, `NumberLiteral(10.0)`, `BraceClose`

### Requirement: @steps directive parsing

The lexer SHALL recognize `@steps` followed by free-form text content until the next `@` directive at line start or end of input. The text content SHALL be emitted as `Text` tokens. `DslPart::Capture` entries within the steps region SHALL be emitted as `Capture(index)` tokens.

#### Scenario: Steps with numbered lines

- **WHEN** input text contains `@steps\n1. Analyze the code\n2. Identify patterns\n3. Apply changes\n`
- **THEN** lexer produces `DirectiveSteps`, `Text("1. Analyze the code\n2. Identify patterns\n3. Apply changes\n")`

#### Scenario: Steps with captures

- **WHEN** input parts are `[Text("@steps\n1. Read "), Capture(expr, span), Text(" file\n")]`
- **THEN** lexer produces `DirectiveSteps`, `Text("1. Read ")`, `Capture(0)`, `Text(" file\n")`

#### Scenario: Steps end at next directive

- **WHEN** input text contains `@steps\n1. Do something\n@output {\n  result: str\n}\n`
- **THEN** lexer produces `DirectiveSteps`, `Text("1. Do something\n")`, `DirectiveOutput`, `BraceOpen`, `Ident("result")`, `Colon`, `Ident("str")`, `BraceClose`

### Requirement: @output directive parsing

The lexer SHALL recognize `@output` followed by a `{...}` block containing field declarations. The field syntax SHALL be identical to `@input` field syntax except that `= default` SHALL NOT be expected (defaults are only for input fields).

#### Scenario: Simple output fields

- **WHEN** input text contains `@output {\n  summary: str\n  confidence: num\n}\n`
- **THEN** lexer produces `DirectiveOutput`, `BraceOpen`, `Ident("summary")`, `Colon`, `Ident("str")`, `Ident("confidence")`, `Colon`, `Ident("num")`, `BraceClose`

### Requirement: Parser produces SkillTemplate AST

The parser SHALL accept `Vec<SkillToken>` and produce a `SkillTemplate` AST containing: an optional `description` string, a list of `input_fields` (Vec<SkillField>), a list of `steps` (Vec<SkillStep>), and a list of `output_fields` (Vec<SkillField>).

#### Scenario: Full skill template

- **WHEN** tokens represent a skill with @description, @input with two fields, @steps with three numbered steps, @output with one field
- **THEN** parser produces `SkillTemplate` with `description: Some("...")`, `input_fields: [field1, field2]`, `steps: [step1, step2, step3]`, `output_fields: [field1]`

### Requirement: Parser parses @input field declarations with defaults

The parser SHALL parse `@input { name: type, name: type = default }` blocks into `Vec<SkillField>`. Each `SkillField` SHALL have `name`, `type_name`, and optional `default`. For array types like `[str]`, the `type_name` SHALL be stored as `"[str]"`.

#### Scenario: Field with default

- **WHEN** tokens are `DirectiveInput`, `BraceOpen`, `Ident("dry_run")`, `Colon`, `Ident("bool")`, `Equals`, `Ident("false")`, `BraceClose`
- **THEN** parser produces `SkillField { name: "dry_run", type_name: "bool", default: Some("false") }`

#### Scenario: Field with array type

- **WHEN** tokens are `DirectiveInput`, `BraceOpen`, `Ident("tags")`, `Colon`, `ArrayOpen`, `Ident("str")`, `ArrayClose`, `BraceClose`
- **THEN** parser produces `SkillField { name: "tags", type_name: "[str]", default: None }`

#### Scenario: Field with string default

- **WHEN** tokens are `DirectiveInput`, `BraceOpen`, `Ident("lang")`, `Colon`, `Ident("str")`, `Equals`, `StringLiteral("english")`, `BraceClose`
- **THEN** parser produces `SkillField { name: "lang", type_name: "str", default: Some("\"english\"") }`

### Requirement: Parser parses @steps into ordered SkillStep list

The parser SHALL parse the text content after `@steps` into an ordered list of `SkillStep` entries. Lines starting with a number followed by `.` (e.g., `1.`, `2.`) SHALL start new steps. The parser SHALL extract `#{}` capture names from each step's text.

#### Scenario: Numbered steps with captures

- **WHEN** steps text is `"1. Analyze #{language} code\n2. Apply #{strategy}\n"`
- **THEN** parser produces `[SkillStep { number: 1, text: "Analyze #{language} code", captures: ["language"] }, SkillStep { number: 2, text: "Apply #{strategy}", captures: ["strategy"] }]`

#### Scenario: Unnumbered text as single step

- **WHEN** steps text is `"Do the thing\n"`
- **THEN** parser produces `[SkillStep { number: 1, text: "Do the thing", captures: [] }]`

### Requirement: Parser error on invalid structure

The parser SHALL produce a diagnostic error for structurally invalid skill blocks: `@input` without `{...}` block, `@output` without `{...}` block, `@description` without string literal, field declaration missing type after `:`.

#### Scenario: Input without braces

- **WHEN** tokens are `DirectiveInput` followed by `Text("...")`
- **THEN** parser produces error "expected `{` after @input"

#### Scenario: Description without string

- **WHEN** tokens are `DirectiveDescription` followed by `DirectiveInput`
- **THEN** parser produces error "expected string literal after @description"

#### Scenario: Field missing type

- **WHEN** tokens inside @input block are `Ident("query")`, `Colon`, `BraceClose`
- **THEN** parser produces error "expected type name after `:`"

### Requirement: Validator checks required directives

The validator SHALL check the parsed `SkillTemplate` for required directives: `@description` is required (error if missing), `@input` is required (error if missing), `@steps` is required (error if missing). `@output` is optional.

#### Scenario: Missing description

- **WHEN** the parsed template has `description: None`
- **THEN** validator produces error "missing required @description directive"

#### Scenario: Missing input

- **WHEN** the parsed template has empty `input_fields` and no `@input` was parsed
- **THEN** validator produces error "missing required @input directive"

#### Scenario: Missing steps

- **WHEN** the parsed template has empty `steps` and no `@steps` was parsed
- **THEN** validator produces error "missing required @steps directive"

#### Scenario: Output optional

- **WHEN** the parsed template has empty `output_fields` (no @output directive)
- **THEN** validator produces no error for missing output

### Requirement: Validator rejects duplicate directives

The validator SHALL produce an error if any directive appears more than once in a skill block: duplicate `@description`, duplicate `@input`, duplicate `@steps`, or duplicate `@output` SHALL all be errors.

#### Scenario: Duplicate description

- **WHEN** the skill block contains two `@description` directives
- **THEN** validator produces error "duplicate @description directive"

#### Scenario: Duplicate input

- **WHEN** the skill block contains two `@input` blocks
- **THEN** validator produces error "duplicate @input directive"

#### Scenario: Duplicate steps

- **WHEN** the skill block contains two `@steps` sections
- **THEN** validator produces error "duplicate @steps directive"

#### Scenario: Duplicate output

- **WHEN** the skill block contains two `@output` blocks
- **THEN** validator produces error "duplicate @output directive"
