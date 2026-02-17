## ADDED Requirements

### Requirement: Skill field type name validation
When checking a `@skill` DSL block, the checker SHALL validate that each `SkillField::type_name` in both `input_fields` and `output_fields` is a valid AG type. A type name is valid if it resolves to a known primitive (`str`, `num`, `int`, `bool`, `nil`, `any`), an array type (`[<valid_type>]`), or a type/struct defined in the current module scope.

#### Scenario: Primitive types pass
- **WHEN** a skill has input field `query: str` and output field `count: int`
- **THEN** no diagnostic SHALL be produced for these fields

#### Scenario: Array types pass
- **WHEN** a skill has input field `tags: [str]`
- **THEN** no diagnostic SHALL be produced

#### Scenario: Unknown type name errors
- **WHEN** a skill has input field `data: UnknownType` and `UnknownType` is not defined in the module scope
- **THEN** the checker SHALL produce a diagnostic containing "unknown type" and "UnknownType"

#### Scenario: Struct-typed field passes
- **WHEN** a module defines `struct Config { key: str }` and a skill has input field `cfg: Config`
- **THEN** no diagnostic SHALL be produced since `Config` is a known type in scope

### Requirement: Skill type validation uses checker scope
The skill field type validation SHALL use the checker's existing scope and type alias registry to resolve type names. This ensures user-defined structs, enums, and type aliases are recognized.

#### Scenario: Type alias recognized
- **WHEN** a module defines `type ID = str` and a skill has input field `id: ID`
- **THEN** no diagnostic SHALL be produced since `ID` resolves via type aliases
