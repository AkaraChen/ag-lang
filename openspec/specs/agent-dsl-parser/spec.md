# Agent DSL Parser

## Purpose
Defines the lexer, parser, and validator for `@agent` DSL blocks, covering directive recognition, token production, AST generation, and validation rules for agent templates.

## Requirements

### Requirement: Agent lexer accepts DslPart input

The agent lexer SHALL accept `Vec<DslPart>` as input (same as the prompt lexer). It SHALL scan `DslPart::Text` segments for both prompt directives and agent-specific directives, and pass through `DslPart::Capture` as `Capture(index)` tokens referencing the original capture by position.

#### Scenario: Text with agent directive and capture

- **WHEN** input is `[Text("@tools "), Capture(expr, span), Text("\n@role system\nHello\n")]`
- **THEN** lexer produces `DirectiveTools`, `Prompt(Capture(0))`, `Prompt(DirectiveRole("system"))`, `Prompt(Text("Hello\n"))`

### Requirement: Agent lexer recognizes agent-specific directives

The lexer SHALL recognize the following agent-specific directives when `@` appears at the beginning of a line followed by a known keyword: `@tools`, `@skills`, `@agents`, `@on`. These produce `DirectiveTools`, `DirectiveSkills`, `DirectiveAgents`, and `DirectiveOn(event_name)` tokens respectively.

#### Scenario: @tools directive

- **WHEN** a text segment contains `@tools ` at line start
- **THEN** lexer produces `DirectiveTools`

#### Scenario: @skills directive

- **WHEN** a text segment contains `@skills ` at line start
- **THEN** lexer produces `DirectiveSkills`

#### Scenario: @agents directive

- **WHEN** a text segment contains `@agents ` at line start
- **THEN** lexer produces `DirectiveAgents`

#### Scenario: @on directive with event name

- **WHEN** a text segment contains `@on init ` at line start
- **THEN** lexer produces `DirectiveOn("init")`

#### Scenario: @on directive with message event

- **WHEN** a text segment contains `@on message ` at line start
- **THEN** lexer produces `DirectiveOn("message")`

#### Scenario: @on directive with error event

- **WHEN** a text segment contains `@on error ` at line start
- **THEN** lexer produces `DirectiveOn("error")`

### Requirement: Agent lexer passes through prompt directives unchanged

The lexer SHALL recognize all prompt directives (`@role`, `@model`, `@examples`, `@constraints`, `@output`, `@messages`) and produce the corresponding `Prompt(PromptToken::...)` wrapped tokens. Prompt directive lexing behavior SHALL match `ag-dsl-prompt` exactly.

#### Scenario: @role directive in agent block

- **WHEN** a text segment contains `@role system\nYou are helpful.\n`
- **THEN** lexer produces `Prompt(DirectiveRole("system"))`, `Prompt(Text("You are helpful.\n"))`

#### Scenario: @model directive in agent block

- **WHEN** a text segment contains `@model claude-sonnet | gpt-4o\n`
- **THEN** lexer produces `Prompt(DirectiveModel)`, `Prompt(Ident("claude-sonnet"))`, `Prompt(Pipe)`, `Prompt(Ident("gpt-4o"))`

#### Scenario: @constraints directive in agent block

- **WHEN** a text segment contains `@constraints {\n  temperature: 0.3\n}\n`
- **THEN** lexer produces `Prompt(DirectiveConstraints)`, `Prompt(BraceOpen)`, `Prompt(Ident("temperature"))`, `Prompt(Colon)`, `Prompt(NumberLiteral(0.3))`, `Prompt(BraceClose)`

#### Scenario: @examples directive in agent block

- **WHEN** a text segment contains `@examples {\n  user: "Fix this"\n  assistant: "I will analyze..."\n}\n`
- **THEN** lexer produces `Prompt(DirectiveExamples)`, `Prompt(BraceOpen)`, `Prompt(Ident("user"))`, `Prompt(Colon)`, `Prompt(StringLiteral("Fix this"))`, `Prompt(Ident("assistant"))`, `Prompt(Colon)`, `Prompt(StringLiteral("I will analyze..."))`, `Prompt(BraceClose)`

#### Scenario: Unknown @ treated as text

- **WHEN** a text segment contains `email me @alice\n`
- **THEN** lexer produces `Prompt(Text("email me @alice\n"))`

#### Scenario: @ mid-line is text

- **WHEN** a text segment contains `contact @support for help\n`
- **THEN** lexer produces `Prompt(Text("contact @support for help\n"))`

### Requirement: @tools followed by capture produces DirectiveTools + Capture

The `@tools` directive SHALL consume the rest of the text line. The next `DslPart::Capture` in the input SHALL be tokenized as a `Capture(index)` token. The parser expects this capture immediately after `DirectiveTools`.

#### Scenario: @tools with capture

- **WHEN** input parts are `[Text("@tools "), Capture(tools_expr, span)]`
- **THEN** lexer produces `DirectiveTools`, `Prompt(Capture(0))`

### Requirement: @skills followed by capture produces DirectiveSkills + Capture

The `@skills` directive SHALL consume the rest of the text line. The next `DslPart::Capture` in the input SHALL be tokenized as a `Capture(index)` token.

#### Scenario: @skills with capture

- **WHEN** input parts are `[Text("@skills "), Capture(skills_expr, span)]`
- **THEN** lexer produces `DirectiveSkills`, `Prompt(Capture(0))`

### Requirement: @agents followed by capture produces DirectiveAgents + Capture

The `@agents` directive SHALL consume the rest of the text line. The next `DslPart::Capture` in the input SHALL be tokenized as a `Capture(index)` token.

#### Scenario: @agents with capture

- **WHEN** input parts are `[Text("@agents "), Capture(agents_expr, span)]`
- **THEN** lexer produces `DirectiveAgents`, `Prompt(Capture(0))`

### Requirement: @on followed by event name and capture

The `@on` directive SHALL be followed by an event name identifier on the same line. The event name is captured in `DirectiveOn(event_name)`. The next `DslPart::Capture` SHALL be tokenized as a `Capture(index)` token referencing the handler function.

#### Scenario: @on init with capture

- **WHEN** input parts are `[Text("@on init "), Capture(handler_expr, span)]`
- **THEN** lexer produces `DirectiveOn("init")`, `Prompt(Capture(0))`

#### Scenario: @on message with capture

- **WHEN** input parts are `[Text("@on message "), Capture(handler_expr, span)]`
- **THEN** lexer produces `DirectiveOn("message")`, `Prompt(Capture(0))`

#### Scenario: @on error with capture

- **WHEN** input parts are `[Text("@on error "), Capture(handler_expr, span)]`
- **THEN** lexer produces `DirectiveOn("error")`, `Prompt(Capture(0))`

### Requirement: Parser produces AgentTemplate AST

The parser SHALL accept `Vec<AgentToken>` and produce an `AgentTemplate` AST containing: an ordered list of `PromptSection` nodes (reused from ag-dsl-prompt), optional `ModelSpec`, optional `OutputSpec`, optional `Constraints`, optional `tools_capture`, optional `skills_capture`, optional `agents_capture`, and a list of `OnHook` entries.

#### Scenario: Full agent template

- **WHEN** tokens represent an agent with @model, @tools capture, @role system, text, @constraints
- **THEN** parser produces `AgentTemplate` with `model: Some(...)`, `tools_capture: Some(0)`, `sections: [Role { system, ... }]`, `constraints: Some(...)`

### Requirement: AgentTemplate contains model, captures, hooks, and prompt sections

The `AgentTemplate` AST SHALL contain the following fields: `name: String`, `sections: Vec<PromptSection>`, `model: Option<ModelSpec>`, `output: Option<OutputSpec>`, `constraints: Option<Constraints>`, `tools_capture: Option<usize>`, `skills_capture: Option<usize>`, `agents_capture: Option<usize>`, `on_hooks: Vec<OnHook>`.

#### Scenario: Agent with tools and skills

- **WHEN** input has `@tools` capture (index 0) and `@skills` capture (index 1) and `@role system` text
- **THEN** parser produces `AgentTemplate` with `tools_capture: Some(0)`, `skills_capture: Some(1)`, `sections: [Role { system, ... }]`

#### Scenario: Agent with on hooks

- **WHEN** input has `@on init` capture (index 0) and `@on message` capture (index 1)
- **THEN** parser produces `AgentTemplate` with `on_hooks: [OnHook { event: "init", capture_index: 0 }, OnHook { event: "message", capture_index: 1 }]`

### Requirement: Body text outside directives becomes prompt content

Text or captures that appear before any directive or after a `@role` directive SHALL be collected into `PromptSection::Role` entries. If text appears before any `@role` directive, it SHALL be assigned to an implicit `@role system` section (same behavior as prompt parser).

#### Scenario: Implicit system role in agent

- **WHEN** the agent block starts with `You are an expert.\n` without `@role`
- **THEN** parser wraps this in `PromptSection::Role { role: System, body: [Text("You are an expert.\n")] }`

#### Scenario: Explicit role sections

- **WHEN** input has `@role system`, text, `@role user`, capture
- **THEN** parser produces `sections: [Role { system, [Text("...")] }, Role { user, [Capture(n)] }]`

### Requirement: Validator checks duplicate @model

The validator SHALL produce an error if `@model` is specified more than once.

#### Scenario: Duplicate model directive

- **WHEN** the parsed agent has duplicate model directives (parser tracked a count > 1)
- **THEN** validator produces error "duplicate @model directive"

### Requirement: Validator checks duplicate @tools

The validator SHALL produce an error if `@tools` is specified more than once.

#### Scenario: Duplicate tools directive

- **WHEN** the parsed agent has duplicate @tools directives
- **THEN** validator produces error "duplicate @tools directive"

### Requirement: Validator checks duplicate @skills

The validator SHALL produce an error if `@skills` is specified more than once.

#### Scenario: Duplicate skills directive

- **WHEN** the parsed agent has duplicate @skills directives
- **THEN** validator produces error "duplicate @skills directive"

### Requirement: Validator checks duplicate @agents

The validator SHALL produce an error if `@agents` is specified more than once.

#### Scenario: Duplicate agents directive

- **WHEN** the parsed agent has duplicate @agents directives
- **THEN** validator produces error "duplicate @agents directive"

### Requirement: Validator checks duplicate @on with same event

The validator SHALL produce an error if two `@on` hooks reference the same event name.

#### Scenario: Duplicate on init hooks

- **WHEN** the parsed agent has two `OnHook { event: "init", ... }` entries
- **THEN** validator produces error "duplicate @on init hook"

### Requirement: Validator warns on unknown @on event

The validator SHALL produce a warning if `@on` references an event name not in the known set (`init`, `message`, `error`).

#### Scenario: Unknown event name

- **WHEN** the parsed agent has `OnHook { event: "shutdown", ... }`
- **THEN** validator produces warning "unknown event 'shutdown'; known events are: init, message, error"

### Requirement: Validator checks capture-requiring directives

The validator SHALL produce an error if `@tools`, `@skills`, `@agents`, or `@on` appear without a following capture.

#### Scenario: @tools without capture

- **WHEN** tokens are `DirectiveTools` followed by `Prompt(Text("..."))`
- **THEN** parser produces error "expected capture expression after @tools"

#### Scenario: @on without capture

- **WHEN** tokens are `DirectiveOn("init")` followed by `Prompt(Text("..."))`
- **THEN** parser produces error "expected capture expression after @on init"

### Requirement: Parser handles full agent example

The parser SHALL correctly handle a complete agent definition with all directive types.

#### Scenario: Full agent block

- **WHEN** input represents:
  ```
  @agent Coder
  @model claude-sonnet
  @tools #{[read_file, write_file]}
  @skills #{[refactor]}
  @on init #{fn(ctx) { log.info("ready") }}
  @role system
  You are an expert software engineer.
  @constraints {
    temperature: 0.3
  }
  @examples {
    user: "Fix this bug"
    assistant: "I'll analyze the code..."
  }
  ```
- **THEN** parser produces `AgentTemplate` with:
  - `name: "Coder"`
  - `model: Some(ModelSpec { models: ["claude-sonnet"] })`
  - `tools_capture: Some(0)`
  - `skills_capture: Some(1)`
  - `on_hooks: [OnHook { event: "init", capture_index: 2 }]`
  - `sections: [Role { system, [Text("You are an expert software engineer.\n")] }, Examples([...])]`
  - `constraints: Some(Constraints { fields: [("temperature", Number(0.3))] })`

### Requirement: No codegen in scope

The `ag-dsl-agent` crate SHALL NOT include a codegen module or DslHandler implementation. The crate exports lexer, parser, validator, and AST types only.

#### Scenario: Crate public API

- **WHEN** a consumer uses `ag-dsl-agent`
- **THEN** the available modules are `lexer`, `parser`, `validator`, and `ast` (no `codegen`, no `handler`)
