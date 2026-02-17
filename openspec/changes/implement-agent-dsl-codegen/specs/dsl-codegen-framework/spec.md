## MODIFIED Requirements

### Requirement: Handler registration

The `Translator` (codegen entry point) SHALL maintain a `HashMap<String, Box<dyn DslHandler>>` mapping DSL kind names to their handlers. Handlers SHALL be registered before codegen begins. The registration API SHALL be `register_dsl_handler(kind: &str, handler: Box<dyn DslHandler>)`. The built-in `codegen()` function SHALL register handlers for both `"prompt"` and `"agent"` kinds.

#### Scenario: Register and use handler

- **WHEN** a handler is registered for kind "prompt" and the AST contains `DslBlock { kind: "prompt", ... }`
- **THEN** codegen invokes the registered "prompt" handler for that block

#### Scenario: Agent handler registered

- **WHEN** the `codegen()` function is called and the AST contains `DslBlock { kind: "agent", ... }`
- **THEN** codegen invokes the registered "agent" handler for that block

#### Scenario: Both prompt and agent in same module

- **WHEN** a module contains both `@prompt` and `@agent` blocks
- **THEN** each block SHALL be dispatched to its respective handler and both produce valid JavaScript
