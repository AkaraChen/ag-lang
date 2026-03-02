## Purpose

Defines the `JsonSchema` and `ToolSchemaInfo` intermediate representation types in `ag-ast` for passing tool type information from checker to codegen without a direct crate dependency.

## ADDED Requirements

### Requirement: JsonSchema enum in ag-ast

The `ag-ast` crate SHALL define a `JsonSchema` enum representing JSON Schema types. The enum SHALL have variants: `String`, `Number`, `Integer`, `Boolean`, `Null`, `Any`, `Array(Box<JsonSchema>)`, `Object { properties: Vec<(String, JsonSchema)>, required: Vec<String>, additional_properties: Option<Box<JsonSchema>> }`, and `AnyOf(Vec<JsonSchema>)`. The enum SHALL derive `Debug`, `Clone`, and `PartialEq`.

#### Scenario: JsonSchema covers all serializable AG types

- **WHEN** the compiler is built
- **THEN** `JsonSchema` SHALL be available in `ag-ast` and represent all types mappable to JSON Schema: strings, numbers, integers, booleans, null, arrays, objects with properties, objects with additional properties, union types (anyOf), and unconstrained (Any)

#### Scenario: JsonSchema is independent of checker Type

- **WHEN** `ag-ast` is compiled
- **THEN** it SHALL NOT depend on `ag-checker` — `JsonSchema` is a self-contained data type

### Requirement: ToolSchemaInfo struct in ag-ast

The `ag-ast` crate SHALL define a `ToolSchemaInfo` struct with fields: `description: Option<String>` and `params: Vec<(String, JsonSchema)>`. This struct SHALL be the interface for passing tool metadata from the checker to codegen.

#### Scenario: ToolSchemaInfo replaces ToolInfo

- **WHEN** codegen needs tool schema information for a `@tool` function
- **THEN** it SHALL receive a `ToolSchemaInfo` (from `ag-ast`) instead of a `ToolInfo` (from `ag-checker`)

#### Scenario: ToolSchemaInfo carries parameter schemas

- **WHEN** a `@tool fn search(query: str, limit: int?)` is registered
- **THEN** the `ToolSchemaInfo` SHALL contain `params: [("query", JsonSchema::String), ("limit", JsonSchema::Integer)]` and nullable params SHALL be excluded from required in the final output
