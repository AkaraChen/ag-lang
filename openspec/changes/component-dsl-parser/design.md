## Context

`@component` DSL blocks embed pure React JSX code. Unlike `@prompt`/`@agent` which use `#{}` captures for AG expression interpolation, components are self-contained JS — no captures needed. The handler uses SWC to parse the JSX and extracts the `export default` as the component.

For AG type integration, JSDoc comments on the export default function provide prop types and documentation. This mirrors how `@tool` extracts type information from AG `fn` signatures — the component's JSDoc becomes the type interface visible to AG code.

Example:

```
@component Counter ```
import { useState } from "react"

/**
 * A simple counter component.
 * @param {number} initial - The starting count value
 * @param {string} label - Display label for the counter
 */
export default function Counter({ initial = 0, label = "Count" }) {
  const [count, setCount] = useState(initial)
  return (
    <div>
      <p>{label}: {count}</p>
      <button onClick={() => setCount(count + 1)}>+</button>
    </div>
  )
}
```

In AG scope, this produces type info equivalent to:
```
// injected into AG type scope:
// Counter: component { initial: num, label: str }
```

## Goals / Non-Goals

**Goals:**
- Parse `@component` DSL block content as JSX using `swc_ecma_parser`
- Extract the `export default` declaration as the component
- Extract JSDoc `@param` tags from the export default to derive prop names, types, descriptions
- Inject extracted prop types into AG's type scope (like `@tool` injects fn type info)
- Emit the parsed SWC module items as JS output
- No `#{}` capture support — content is pure JSX

**Non-Goals:**
- No custom directive syntax
- No AG expression interpolation inside JSX
- No CSS processing
- No runtime component framework

## Decisions

### Decision 1: SWC parses the content directly, no captures

The handler concatenates all `DslPart::Text` segments into a single string (any `DslPart::Capture` is an error — captures not supported in `@component`). The string is fed to `swc_ecma_parser` with JSX syntax enabled.

**Rationale**: Components are self-contained React code. AG expressions don't belong inside JSX — the component communicates with AG through its props interface.

### Decision 2: `export default` is the component

The handler walks parsed `swc::Module` items to find `export default`. If not found, emit error. The block name from `@component Name` is the AG-level identifier and does not need to match the JS function name.

### Decision 3: JSDoc provides prop types for AG scope

The handler extracts the leading JSDoc comment block on the `export default` function. It parses `@param {type} name - description` tags to build a prop type map:

```
@param {string} label - Display label   →  label: str
@param {number} count - Item count      →  count: num
@param {boolean} visible - Show/hide    →  visible: bool
@param {string[]} items - List of items →  items: [str]
```

JSDoc type mapping to AG types:
| JSDoc type | AG type |
|-----------|---------|
| `string` | `str` |
| `number` | `num` |
| `boolean` | `bool` |
| `string[]`, `Array<string>` | `[str]` |
| `number[]`, `Array<number>` | `[num]` |
| `object` | `any` |
| `*`, unrecognized | `any` |

The top-level JSDoc description (before `@param` tags) becomes the component description.

**Rationale**: This mirrors `@tool`'s approach — `@tool` extracts schema from AG fn signatures + doc comments; `@component` extracts schema from JSDoc + JS function params. Both inject type info into AG's scope so the rest of the compiler can type-check usage.

### Decision 4: Prop info stored as ComponentMeta in handler output

The handler produces both the SWC module items (for JS codegen) and a `ComponentMeta` structure containing extracted prop info. This meta is made available to the AG checker/type system, similar to how `ToolAnnotation` metadata is attached to `FnDecl`.

```rust
struct ComponentMeta {
    name: String,
    description: Option<String>,
    props: Vec<ComponentProp>,
}

struct ComponentProp {
    name: String,
    ty: String,          // AG type string: "str", "num", "bool", "[str]", "any"
    description: Option<String>,
    has_default: bool,   // from JS destructuring default
}
```

### Decision 5: Minimal crate structure

`ag-dsl-component` has `lib.rs` (handler + JSDoc extraction logic). No separate modules unless JSDoc parsing grows complex enough to warrant it.

## Risks / Trade-offs

- **[JSDoc is optional]** If a component has no JSDoc, all props are typed as `any`. This is acceptable for gradual adoption — components work without JSDoc, just without AG type checking on props.
- **[JSDoc type subset]** Only basic JSDoc types are mapped. Complex TypeScript-style JSDoc (`@param {{x: number, y: number}} point`) is not supported initially — falls back to `any`.
- **[No captures]** Components can't interpolate AG expressions. This is intentional — the boundary between AG and React is the props interface.
- **[SWC parse errors]** SWC errors reference line/column within the DSL block. Need to offset to original source span.
