## 1. Crate Setup

- [x] 1.1 Create `crates/ag-dsl-component/` with `Cargo.toml` — depend on `ag-dsl-core`, `swc_ecma_parser`, `swc_ecma_ast`, `swc_common`
- [x] 1.2 Add `ag-dsl-component` to workspace `Cargo.toml` members
- [x] 1.3 Create `src/lib.rs` with skeleton `ComponentDslHandler` implementing `DslHandler`, and `ComponentMeta`/`ComponentProp` structs
- [x] 1.4 `cargo build -p ag-dsl-component` — verify crate compiles

## 2. Core Handler

- [x] 2.1 Implement text assembly: concatenate `DslPart::Text` segments, reject any `DslPart::Capture` with error
- [x] 2.2 Implement SWC JSX parsing: feed assembled text to `swc_ecma_parser` with JSX syntax enabled, map SWC errors to `DslError`
- [x] 2.3 Implement export default validation: walk parsed module items, find `ExportDefaultDecl` or `ExportDefaultExpr`, error if not found

## 3. JSDoc Extraction

- [x] 3.1 Extract leading JSDoc comment from export default function — get the raw comment text from SWC's comment storage
- [x] 3.2 Parse JSDoc description (text before first `@param` tag)
- [x] 3.3 Parse `@param {type} name - description` tags into prop list — handle variants: with/without description, with/without braces
- [x] 3.4 Implement JSDoc type → AG type mapping: `string`→`str`, `number`→`num`, `boolean`→`bool`, `string[]`/`Array<string>`→`[str]`, etc., unknown→`any`
- [x] 3.5 Detect default values from export default function's parameter destructuring pattern (`ObjectPat` with `AssignPatProp` nodes), set `has_default` on matching props
- [x] 3.6 Build `ComponentMeta { name, description, props }` combining JSDoc extraction + destructuring analysis

## 4. Integration

- [x] 4.1 Register `ComponentDslHandler` in `ag-codegen` translator for kind `"component"`
- [x] 4.2 Add `ag-dsl-component` dependency to `ag-codegen/Cargo.toml`

## 5. Tests

- [x] 5.1 Test: simple component with `export default function` and JSDoc — verify ComponentMeta has correct props with types
- [x] 5.2 Test: component with imports — all module items preserved in output
- [x] 5.3 Test: component with no JSDoc — props list empty, types default to `any`
- [x] 5.4 Test: component with defaults in destructuring — `has_default` flags correct
- [x] 5.5 Test: missing `export default` — produces error
- [x] 5.6 Test: `DslPart::Capture` present — produces "captures not supported" error
- [x] 5.7 Test: invalid JSX syntax — produces SWC error as `DslError`
