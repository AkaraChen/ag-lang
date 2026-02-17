## ADDED Requirements

### Requirement: DSL block kind dispatch in checker
The `ag-checker` `check_dsl_block` method SHALL dispatch DSL blocks by their `kind` field to the appropriate DSL crate's lexer → parser → validator pipeline. Supported kinds: `"prompt"`, `"agent"`, `"skill"`, `"server"`, `"component"`.

#### Scenario: Prompt DSL validation during check
- **WHEN** a module contains `@prompt sys ``` ``` ` (empty prompt, no @role)
- **THEN** the checker SHALL run the prompt lexer/parser/validator and report a diagnostic about the empty or missing role

#### Scenario: Server DSL validation during check
- **WHEN** a module contains `@server app ``` @port 0 @get /health #{handler} ``` ` with port 0
- **THEN** the checker SHALL run the server validator and report "port must not be 0" as a diagnostic

#### Scenario: Agent DSL validation during check
- **WHEN** a module contains `@agent bot ``` @on init #{h1} @on init #{h2} ``` ` with duplicate hook events
- **THEN** the checker SHALL report a diagnostic about duplicate `@on` hook events

#### Scenario: Skill DSL validation during check
- **WHEN** a module contains `@skill fix ``` @steps 1. Do something ``` ` missing `@description` and `@input`
- **THEN** the checker SHALL report diagnostics for missing `@description` and missing `@input`

#### Scenario: Component DSL validation during check
- **WHEN** a module contains a `@component` block
- **THEN** the checker SHALL call `parse_component()` and validate the resulting `ComponentMeta`

#### Scenario: Unknown DSL kind is silently skipped
- **WHEN** a module contains `@unknown foo ``` content ``` `
- **THEN** the checker SHALL NOT produce a diagnostic (the codegen phase will report "no handler registered")

### Requirement: Diagnostic mapping from DSL to checker
DSL-internal diagnostics (which have `message` and `severity` but no span) SHALL be mapped to `ag_ast::Diagnostic` using the DSL block's `span` field as the location. Only diagnostics with `Severity::Error` SHALL be reported as checker errors. Diagnostics with `Severity::Warning` SHALL also be reported (as the checker has no severity distinction, all become diagnostics).

#### Scenario: DSL error with block span
- **WHEN** a server DSL block at span (100, 250) produces a validator error "port must not be 0"
- **THEN** the checker diagnostic SHALL have message "port must not be 0" and span (100, 250)

#### Scenario: DSL parse error with block span
- **WHEN** a prompt DSL block fails to parse (e.g. empty body)
- **THEN** the checker diagnostic SHALL include the parse error message with the block's span

### Requirement: Existing capture type-checking preserved
The checker SHALL continue to type-check all `DslPart::Capture` expressions via `check_expr`, in addition to the new DSL-internal validation. These two checks are complementary: capture type-checking validates the AG expressions, DSL validation checks the DSL structure.

#### Scenario: Undefined variable in capture still errors
- **WHEN** a prompt DSL block contains `#{undefined_var}`
- **THEN** the checker SHALL still report "undefined variable `undefined_var`"

#### Scenario: Valid capture plus DSL validation
- **WHEN** a server DSL block has valid captures but `@port 0`
- **THEN** the checker SHALL report the port error AND successfully type-check the captures

### Requirement: DslContent::Inline only
DSL-internal validation SHALL only run for `DslContent::Inline` blocks. `DslContent::FileRef` blocks SHALL continue to be skipped (only capture expressions in inline blocks are type-checked).

#### Scenario: FileRef block skips DSL validation
- **WHEN** a module contains `@prompt sys from "./prompt.txt"`
- **THEN** the checker SHALL NOT attempt to run prompt validation on it

### Requirement: ag-checker DSL dependencies
The `ag-checker/Cargo.toml` SHALL include `ag-dsl-prompt`, `ag-dsl-agent`, `ag-dsl-skill`, `ag-dsl-server`, and `ag-dsl-component` as dependencies.

#### Scenario: Checker builds with DSL dependencies
- **WHEN** `cargo build -p ag-checker` is run
- **THEN** the build SHALL succeed
