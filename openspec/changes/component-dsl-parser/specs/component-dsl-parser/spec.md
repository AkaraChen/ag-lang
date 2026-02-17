## ADDED Requirements

### Requirement: Component handler parses JSX via SWC

The `ag-dsl-component` crate SHALL implement `DslHandler` for `@component` blocks. The handler SHALL concatenate all `DslPart::Text` segments into a single string, parse it using `swc_ecma_parser` with JSX syntax enabled, and return the parsed module items.

#### Scenario: Simple component

- **WHEN** input is `@component Button ``` export default function Button() { return <button>Click</button> } ```\n`
- **THEN** handler produces `swc::Module` items containing the function declaration with JSX

#### Scenario: Component with imports

- **WHEN** input is `@component Counter ``` import { useState } from "react"\nexport default function Counter() { ... } ```\n`
- **THEN** handler emits all module items including the import AND the function declaration

#### Scenario: Invalid JSX produces error

- **WHEN** input is `@component Broken ``` export default function() { return <div> } ```\n`
- **THEN** handler returns `DslError` with the SWC parse error message

### Requirement: Captures are not supported

The handler SHALL reject `DslPart::Capture` in `@component` blocks. If any capture is present, the handler SHALL return an error.

#### Scenario: Capture in component is error

- **WHEN** input contains a `DslPart::Capture`
- **THEN** handler returns error "captures (#{}) are not supported in @component blocks"

### Requirement: Export default extraction

The handler SHALL validate that the parsed module contains an `export default` declaration. If no `export default` is found, the handler SHALL return an error.

#### Scenario: Missing export default

- **WHEN** input is `@component Foo ``` function Foo() { return <div/> } ```\n`
- **THEN** handler returns error "component must have an export default declaration"

#### Scenario: Export default function

- **WHEN** input is `@component Bar ``` export default function Bar() { return <div/> } ```\n`
- **THEN** handler finds the export default and emits successfully

#### Scenario: Export default expression

- **WHEN** input is `@component Baz ``` const Baz = () => <div/>; export default Baz ```\n`
- **THEN** handler finds the export default and emits successfully

### Requirement: JSDoc extraction for prop types

The handler SHALL extract JSDoc comments from the `export default` function. It SHALL parse `@param {type} name - description` tags to build a prop type map. The JSDoc description (text before `@param` tags) SHALL become the component description.

#### Scenario: JSDoc with typed params

- **WHEN** the export default has JSDoc:
  ```
  /**
   * A counter component.
   * @param {number} initial - Starting value
   * @param {string} label - Display label
   */
  export default function Counter({ initial, label }) { ... }
  ```
- **THEN** handler extracts: description = "A counter component.", props = [{ name: "initial", ty: "num", description: "Starting value" }, { name: "label", ty: "str", description: "Display label" }]

#### Scenario: JSDoc with no params

- **WHEN** the export default has JSDoc `/** A static banner. */`
- **THEN** handler extracts: description = "A static banner.", props = []

#### Scenario: No JSDoc

- **WHEN** the export default has no JSDoc comment
- **THEN** handler extracts: description = None, props = [] (all prop types default to `any`)

#### Scenario: Param without description

- **WHEN** JSDoc contains `@param {string} name`
- **THEN** handler extracts prop with name = "name", ty = "str", description = None

### Requirement: JSDoc type mapping to AG types

The handler SHALL map JSDoc types to AG types as follows: `string` → `str`, `number` → `num`, `boolean` → `bool`, `string[]` / `Array<string>` → `[str]`, `number[]` / `Array<number>` → `[num]`, `boolean[]` / `Array<boolean>` → `[bool]`, `object` → `any`, `*` → `any`. Unrecognized types SHALL map to `any`.

#### Scenario: Basic type mapping

- **WHEN** JSDoc has `@param {string} name` and `@param {number} count`
- **THEN** props have ty = "str" and ty = "num" respectively

#### Scenario: Array type mapping

- **WHEN** JSDoc has `@param {string[]} items`
- **THEN** prop has ty = "[str]"

#### Scenario: Unknown type falls back to any

- **WHEN** JSDoc has `@param {CustomType} data`
- **THEN** prop has ty = "any"

### Requirement: Default value detection from destructuring

The handler SHALL detect default values in the export default function's parameter destructuring pattern. Props with defaults SHALL be marked as `has_default: true` in `ComponentProp`.

#### Scenario: Param with JS default

- **WHEN** export default is `function Counter({ initial = 0, label = "Count" }) { ... }`
- **THEN** props "initial" and "label" both have has_default = true

#### Scenario: Param without default

- **WHEN** export default is `function Counter({ initial, label }) { ... }`
- **THEN** props "initial" and "label" both have has_default = false

### Requirement: ComponentMeta produced alongside module items

The handler SHALL produce a `ComponentMeta` structure containing: `name` (from DSL block name), `description` (from JSDoc), and `props: Vec<ComponentProp>` (from JSDoc + destructuring). This meta SHALL be made available for AG type scope injection.

#### Scenario: Full component meta

- **WHEN** `@component Counter ``` /** A counter. @param {number} initial - Start */ export default function Counter({ initial = 0 }) { ... } ```\n`
- **THEN** ComponentMeta = { name: "Counter", description: Some("A counter."), props: [{ name: "initial", ty: "num", description: Some("Start"), has_default: true }] }

### Requirement: All module items emitted

The handler SHALL emit ALL parsed module items (imports, helpers, the component), not just the export default.

#### Scenario: Component with helpers

- **WHEN** input has import, a helper function, and export default
- **THEN** handler emits all three module items

### Requirement: Handler registered in codegen translator

The `ag-codegen` translator SHALL register the component DSL handler for the `"component"` kind.

#### Scenario: Component block dispatched

- **WHEN** the compiler encounters `@component Name ``` ... ```\n`
- **THEN** the codegen translator dispatches to the component handler
