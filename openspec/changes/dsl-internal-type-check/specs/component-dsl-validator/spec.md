## ADDED Requirements

### Requirement: Component validator module
The `ag-dsl-component` crate SHALL have a `validate()` function that accepts a `&ComponentMeta` and returns `Vec<Diagnostic>` (using a local `Diagnostic` type with `message` and `severity`, matching the pattern in other DSL crates).

#### Scenario: Valid component passes validation
- **WHEN** a `ComponentMeta` has a name, at least one prop with a known AG type, and no duplicates
- **THEN** `validate()` SHALL return an empty diagnostics vector

### Requirement: Duplicate prop names detection
The validator SHALL report an error when two or more props share the same name.

#### Scenario: Duplicate prop name
- **WHEN** a `ComponentMeta` has two props both named "title"
- **THEN** the validator SHALL return a diagnostic with severity Error and message containing "duplicate prop" and "title"

### Requirement: Unknown prop type warning
The validator SHALL report a warning when a prop's `ty` field is not a recognized AG primitive type (`str`, `num`, `int`, `bool`, `nil`, `any`) and is not an array type (`[<type>]`).

#### Scenario: Known type passes
- **WHEN** a prop has type "str", "num", "int", "bool", or "[str]"
- **THEN** no diagnostic SHALL be produced for that prop

#### Scenario: Unknown type warned
- **WHEN** a prop has type "SomeCustomType" (not a primitive or array)
- **THEN** the validator SHALL return a diagnostic with severity Warning containing "unknown type" and "SomeCustomType"

### Requirement: Empty component warning
The validator SHALL report a warning when a component has no props (zero-prop components are valid but unusual).

#### Scenario: No props
- **WHEN** a `ComponentMeta` has zero props
- **THEN** the validator SHALL return a warning diagnostic about no props defined
