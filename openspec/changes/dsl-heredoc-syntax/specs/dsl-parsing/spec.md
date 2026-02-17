## MODIFIED Requirements

### Requirement: DSL block triggered by top-level @

The parser SHALL recognize a DSL block when it encounters `@` at top-level followed by an identifier (DSL kind), another identifier (block name), and either `<<` followed by an identifier (heredoc inline block) or `from` (file reference). The parser SHALL NOT maintain a list of valid DSL kind names — any identifier after `@` is accepted.

#### Scenario: Inline DSL block with heredoc

- **WHEN** the input is `@prompt system <<EOF\ncontent\nEOF\n`
- **THEN** parser produces `DslBlock { kind: "prompt", name: "system", content: Inline(...) }`

#### Scenario: File reference DSL block

- **WHEN** the input is `@component Button from "./button.tsx"`
- **THEN** parser produces `DslBlock { kind: "component", name: "Button", content: FileRef("./button.tsx") }`

#### Scenario: Unknown DSL kind accepted

- **WHEN** the input is `@graphql GetUsers <<END\nquery { users { id } }\nEND\n`
- **THEN** parser produces `DslBlock { kind: "graphql", name: "GetUsers", content: Inline(...) }` without error

### Requirement: @ with identifiers but no body

If the parser encounters `@` at top-level with two identifiers but neither `<<` nor `from`, the parser SHALL emit a diagnostic.

#### Scenario: @ with identifiers but no body

- **WHEN** the input is `@prompt system\nfn foo() {}`
- **THEN** parser produces diagnostic "expected `<<LABEL` or `from` after `@prompt system`"
