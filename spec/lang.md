# AgentScript Language Specification v0.2

> A purpose-built language for authoring AI agents — with first-class DSL blocks for prompts, agents, skills, components, and HTTP serving; annotations for tool metadata and JS interop; and a familiar JS-flavored core language.

---

## 1. Design Philosophy

| Principle | Rationale |
|-----------|-----------|
| **JS-flavored, shorter keywords** | Familiar to web devs, less visual noise |
| **LL(1) grammar, recursive descent** | Simple compiler front-end, no parser generator needed |
| **Extensible DSL blocks** | `@prompt`, `@agent`, `@skill`, `@component`, `@server` — domain constructs as DSL, not keywords |
| **Annotation system** | `@tool`, `@js` — metadata on declarations, not new syntax |
| **Structural typing + basic inference** | Enough type safety to catch wiring bugs; no generics gymnastics |
| **Compiles to Node.js bundle** | Instant production readiness; other languages access via HTTP/SDK |

---

## 2. Lexical Conventions

### 2.1 Keywords (reserved)

```
fn       let      const    mut        if
else     for      in       of         while
match    ret      yield    await      async
import   export   from     as         type
struct   enum     impl     pub        self
true     false    nil      extern     try
catch    emit     use      with       on       _
```

> **Note:** `agent`, `tool`, `skill`, `component`, `prompt`, `server` are **not** keywords. They are identifiers used as DSL kind names or annotation names (e.g., `@agent`, `@tool`). This keeps the keyword set small and the DSL system open to extension.

### 2.2 Operators & Punctuation

```
+  -  *  /  %  **          // arithmetic
== != < > <= >=            // comparison
&& || !                    // logical
|> ?? ?.                   // pipe, nullish coalesce, optional chain
=  +=  -=  *=  /=         // assignment
=> -> ::                   // fat arrow, thin arrow (return type), scope resolution
@                          // DSL block prefix / annotation prefix
#                          // DSL capture prefix (inside DSL blocks)
{ } ( ) [ ] < >           // grouping
, ; : .                    // delimiters
..  ...                    // range, spread / variadic
```

### 2.3 Comments

```
// line comment
/* block comment */
/// doc comment (attaches to next declaration, exported as JSDoc)
```

### 2.4 String Literals

```javascript
"hello"                     // regular string
'hello'                     // also regular string
`hello ${name}`             // template string (JS-style interpolation)
```

### 2.5 DSL Block System

DSL blocks are **top-level declarations** for domain-specific content — prompts, agents, skills, components, servers, and any future DSL kind.

#### Syntax

```
@<kind> <name> ```
  ... DSL content ...
```
```

Or file reference form:

```
@<kind> <name> from "<path>"
```

- `<kind>` — any identifier (e.g., `prompt`, `agent`, `skill`, `component`, `server`, `graphql`, ...)
- `<name>` — the binding name for the compiled output
- The triple-backtick block enters **raw mode** — content is not tokenized as AG code

#### Captures

Inside DSL blocks, `#{ expr }` captures an AgentScript expression (evaluated at runtime):

```
@prompt greeting ```
  Hello, #{user.name}! You have #{messages.len()} new messages.
```
```

Captures can contain any AG expression — identifiers, member access, function calls, arithmetic:

```
@prompt summary ```
  Total items: #{cart.items.len()}
  Subtotal: #{format_currency(cart.total())}
```
```

> **Future:** Statement block captures (`#{ ... stmts ... }`) are planned for DSL blocks that need executable code (e.g., route handlers in `@server`).

#### Directives

Inside DSL blocks, `@keyword` at the start of a line is a **directive** — metadata parsed by the DSL handler:

```
@prompt system_prompt ```
  @role system
  @model claude-sonnet | gpt-4o

  You are a helpful coding assistant.

  @examples {
    user: "Fix this bug"
    assistant: "I'll analyze the code..."
  }

  @constraints {
    temperature: 0.7
    max_tokens: 2048
  }
```
```

Each DSL kind defines its own set of valid directives. The prompt DSL supports: `@role`, `@model`, `@examples`, `@output`, `@constraints`, `@messages`.

#### Extensibility

The DSL system is **open** — any `@kind` identifier is accepted by the parser. A registered `DslHandler` processes the block during codegen. Unregistered kinds produce a compile error. This means new domain constructs (e.g., `@graphql`, `@workflow`, `@test`) can be added without language changes.

### 2.6 Annotations

Annotations decorate declarations with metadata. They use the same `@` prefix as DSL blocks, but are followed by a declaration (not a name + backticks):

```javascript
// @tool marks a function as an LLM-callable tool
@tool
fn read_file(path: str) -> str | Error {
  await fs.read(path)
}

// @js binds an extern declaration to a JavaScript module
@js("node:fs/promises")
extern fn readFile(path: str, encoding: str) -> Promise<str>

// @js with rename
@js("node:path", name = "join")
extern fn path_join(parts: ...str) -> str
```

**Disambiguation:** When the parser sees `@`, it looks ahead:
- `@ IDENT IDENT (``` | from)` → DSL block
- `@ IDENT ( "(" args ")" )? declaration` → Annotation on the following declaration

Built-in annotations:

| Annotation | Applies To | Purpose |
|------------|-----------|---------|
| `@tool` | `fn` | Marks function as an LLM-callable tool; auto-generates JSON Schema |
| `@tool("description")` | `fn` | Same, with inline description (overrides doc comment) |
| `@js("module")` | `extern fn/struct/type` | Specifies the JavaScript module to import from |
| `@js("module", name = "jsName")` | `extern fn/struct/type` | Same, with JS-side name remapping |

---

## 3. Type System

### 3.1 Primitive Types

```
str        // string
num        // number (f64)
int        // integer (i64)
bool       // boolean
nil        // null/undefined equivalent
any        // escape hatch, minimal checking
```

### 3.2 Compound Types

```javascript
[str]                       // array of strings
{str: num}                  // map from string to number
(str, num) -> bool          // function type
str?                        // nullable / optional (sugar for str | nil)
str | num                   // union type
Promise<T>                  // async result (built-in generic)
```

### 3.3 Struct Types

```javascript
struct User {
  name: str
  age: int
  email: str?
}

// Structs are structurally typed — any value with matching shape satisfies the type.
```

### 3.4 Enum Types

```javascript
enum Status {
  Pending
  Active(since: str)
  Error(code: int, msg: str)
}

// Usage:
let s = Status::Active(since: "2025-01-01")

match s {
  Status::Pending => "waiting"
  Status::Active(since) => "active since ${since}"
  Status::Error(code, _) => "error ${code}"
}
```

### 3.5 Type Aliases

```javascript
type ID = str
type Handler = (Request) -> Response
type Result<T> = T | Error      // single generic parameter allowed for type aliases
```

### 3.6 Type Checking Rules

The type checker is **basic and intentionally limited**:

- **Structural subtyping**: `{name: str, age: int}` satisfies `{name: str}`
- **Local type inference**: `let x = 42` infers `x: int`; no global inference
- **No higher-kinded types, no trait/interface system**: keep it simple
- **Explicit annotation required** for function params and return types
- **`any` suppresses checking** for that binding
- **Union narrowing** via `match` and `if` type guards
- **Int-to-num widening**: `int` is assignable to `num`

---

## 4. Declarations & Expressions

### 4.1 Variables

```javascript
let name = "Alice"          // immutable (default)
mut counter = 0             // mutable
const MAX = 100             // compile-time constant
```

### 4.2 Functions

```javascript
fn add(a: int, b: int) -> int {
  a + b                     // implicit return of last expression
}

fn greet(name: str, loud: bool = false) -> str {
  let msg = "Hello, ${name}!"
  if loud { ret msg.upper() }  // explicit return with `ret`
  msg
}

// Arrow functions (for short lambdas)
let double = (x: int) => x * 2
let log = (msg: str) => { console.log(msg) }
```

### 4.3 Pipe Operator

```javascript
let result = data
  |> parse
  |> validate
  |> transform(_, options)     // _ is placeholder for piped value
  |> await send
```

### 4.4 Error Handling

```javascript
// Functions can return Result-style values
fn divide(a: num, b: num) -> num | Error {
  if b == 0 { ret Error("division by zero") }
  a / b
}

// ? operator for early return on error
fn process(input: str) -> str | Error {
  let parsed = parse(input)?        // returns Error early if parse fails
  let validated = validate(parsed)?
  transform(validated)
}

// try/catch for imperative style
try {
  let data = await fetch_data()
} catch e {
  log.error("Failed: ${e.msg}")
}
```

### 4.5 Pattern Matching

```javascript
match value {
  0 => "zero"
  1..10 => "small"
  n if n > 100 => "big: ${n}"
  _ => "other"
}

// Destructuring in match
match response {
  {status: 200, body} => handle_success(body)
  {status: 404} => handle_not_found()
  {status: s} if s >= 500 => handle_server_error(s)
  _ => handle_unknown()
}
```

---

## 5. Extern Declarations

Extern declarations declare **JavaScript bindings** — types and functions that exist in JS but need to be known to the AG type checker. They produce no runtime code (erased during codegen).

### 5.1 Extern Functions

```javascript
// Global JS function (no @js annotation → assumed globally available)
extern fn fetch(input: str, init: RequestInit?) -> Promise<Response>

// Function from a JS module
@js("node:fs/promises")
extern fn readFile(path: str, encoding: str) -> Promise<str>

// Renamed import: AG name differs from JS name
@js("node:path", name = "join")
extern fn path_join(parts: ...str) -> str

// Variadic parameters: ...T as the last parameter
extern fn console_log(args: ...any) -> nil
```

### 5.2 Extern Structs

Extern structs declare JavaScript objects/classes with known fields and methods:

```javascript
@js("@agentscript/stdlib/http/server")
extern struct Context {
  req: HonoRequest
  fn text(text: str) -> Response
  fn json(data: any) -> Response
  fn html(html: str) -> Response
  fn redirect(url: str) -> Response
  fn header(name: str, value: str)
  fn status(code: int)
}
```

Fields are type-checked on access. Methods have signatures but no body.

### 5.3 Extern Types (Opaque)

Extern types declare names the compiler knows about but cannot inspect:

```javascript
extern type Headers
extern type ReadableStream
extern type URL
```

Opaque types can be passed around and used in type positions, but field access and method calls on them are compile errors.

### 5.4 Codegen Behavior

- Extern declarations are **erased** — no JavaScript output for the declaration itself
- `@js("module")` externs generate `import { name } from "module"` when referenced
- Multiple `@js` externs from the same module are merged into one import
- Unreferenced `@js` externs produce no import

```javascript
// These two declarations:
@js("node:fs/promises") extern fn readFile(path: str, encoding: str) -> Promise<str>
@js("node:fs/promises") extern fn writeFile(path: str, data: str) -> Promise<nil>

// Produce a single merged import when both are used:
// import { readFile, writeFile } from "node:fs/promises";
```

---

## 6. Agent System

Agents are declared as **DSL blocks** using `@agent`. The block body serves double duty: it is both the agent's system prompt and its configuration.

### 6.1 Agent Declaration

```
@agent Coder ```
  @model claude-sonnet | gpt-4o

  @tools #{[read_file, write_file, run_tests]}

  @role system
  You are an expert software engineer.

  ## Your capabilities
  You can read files, write code, and run tests.

  ## Style
  - Be concise
  - Prefer #{preferred_lang} idioms
  - Always explain your reasoning

  @constraints {
    temperature: 0.3
    max_tokens: 4096
  }

  @examples {
    user: "Fix this bug"
    assistant: "I'll analyze the code and identify the issue..."
  }
```
```

The `@agent` DSL handler extends the `@prompt` handler with agent-specific directives:

| Directive | Purpose |
|-----------|---------|
| `@model` | Model selection with fallback order |
| `@tools #{[...]}` | List of tool functions this agent can invoke |
| `@skills #{[...]}` | List of skills this agent can use |
| `@agents #{[...]}` | Sub-agents for composition |
| `@on <event> #{handler}` | Lifecycle hooks (captures a handler function) |
| `@role`, `@examples`, `@constraints`, `@output`, `@messages` | Inherited from prompt DSL |

Body text (outside directives) is the system prompt.

### 6.2 Agent Composition

```
@agent TeamLead ```
  @model claude-sonnet
  @agents #{[Coder, Reviewer, Designer]}

  @role system
  You coordinate a team of specialist agents.
  Delegate tasks to the right agent based on the request.

  Available agents:
  - Coder: writes and edits code
  - Reviewer: reviews code for quality and bugs
  - Designer: designs UI components and layouts
```
```

### 6.3 Agent Lifecycle Hooks

```
@agent Coder ```
  @model claude-sonnet
  @tools #{[read_file, write_file]}

  @on init #{fn(ctx: AgentContext) {
    log.info("Agent initialized for session ${ctx.session_id}")
  }}

  @on message #{fn(msg: UserMessage) -> AgentResponse {
    let enriched = enrich_context(msg)
    await self.complete(enriched)
  }}

  @on error #{fn(e: Error) {
    log.error("Agent error: ${e.msg}")
  }}

  @role system
  You are an expert software engineer.
```
```

### 6.4 What Agents Compile To

```javascript
// @agent Coder ``` ... ```  compiles to:
import { AgentRuntime } from "@agentscript/runtime"

const Coder = new AgentRuntime({
  model: ["claude-sonnet", "gpt-4o"],
  tools: [read_file, write_file, run_tests],
  messages: [
    { role: "system", content: "You are an expert software engineer..." }
  ],
  constraints: { temperature: 0.3, max_tokens: 4096 },
  examples: [...]
});
```

---

## 7. Tool Declarations

Tools are **annotated functions** that agents can invoke via LLM tool-calling. The `@tool` annotation marks a regular `fn` as a tool.

### 7.1 Basic Tool

```javascript
/// Read the contents of a file at the given path.
/// @param path - Absolute or relative file path
/// @returns The file contents as a string
@tool
fn read_file(path: str) -> str | Error {
  await fs.read(path)
}

/// Search the web for a query.
/// @param query - The search query
/// @param max_results - Maximum number of results (default: 5)
@tool
fn web_search(query: str, max_results: int = 5) -> [SearchResult] {
  let results = await http.get("https://api.search.com/v1", {
    params: { q: query, limit: max_results }
  })?
  results.items
}
```

### 7.2 Tool with Inline Description

```javascript
// When a short description suffices, pass it directly:
@tool("Calculate the area of a geometric shape")
fn calc_area(shape: str, dims: {w: num?, h: num?, r: num?}) -> num {
  match shape {
    "circle" => 3.14159 * (dims.r ?? 0) ** 2
    "rect" => (dims.w ?? 0) * (dims.h ?? 0)
    "triangle" => 0.5 * (dims.w ?? 0) * (dims.h ?? 0)
    _ => Error("unknown shape: ${shape}")
  }
}
```

### 7.3 JSON Schema Generation

The compiler automatically generates JSON Schema for tool-calling from:
1. The function name
2. Parameter names and types
3. Doc comments (`///`) and `@param` / `@returns` annotations
4. Inline `@tool("description")` if provided

```javascript
// The @tool fn above compiles to:
// function calc_area(shape, dims) { ... }
// calc_area.schema = {
//   "name": "calc_area",
//   "description": "Calculate the area of a geometric shape",
//   "parameters": {
//     "type": "object",
//     "properties": {
//       "shape": { "type": "string" },
//       "dims": {
//         "type": "object",
//         "properties": {
//           "w": { "type": "number" }, "h": { "type": "number" }, "r": { "type": "number" }
//         }
//       }
//     },
//     "required": ["shape", "dims"]
//   }
// }
```

---

## 8. Skill Declarations

Skills are **multi-step, compound tool sequences** declared as DSL blocks. They describe a recipe that an agent can follow.

```
@skill refactor ```
  @description "Refactor code in a file to improve quality, readability, and performance."

  @input {
    file_path: str
    goals: [str]
    dry_run: bool = false
  }

  @steps
  1. Read the file at #{input.file_path}
  2. Analyze code for refactoring opportunities based on goals: #{input.goals}
  3. Generate refactored version
  4. If not #{input.dry_run}, write the result and run tests
  5. If tests fail, rollback to original

  @output {
    original: str
    refactored: str
    analysis: str
    applied: bool
  }
```
```

### Skill Directives

| Directive | Purpose |
|-----------|---------|
| `@description` | Description shown to the LLM for skill invocation |
| `@input { ... }` | Input schema (typed fields with optional defaults) |
| `@steps` | Ordered list of steps (natural language with `#{}` captures) |
| `@output { ... }` | Output schema |

### What Skills Compile To

```javascript
import { SkillRuntime } from "@agentscript/runtime"

const refactor = new SkillRuntime({
  description: "Refactor code in a file...",
  inputSchema: { file_path: "string", goals: "string[]", dry_run: "boolean" },
  steps: [...],
  outputSchema: { original: "string", refactored: "string", ... }
});
```

---

## 9. Component Declarations

Components emit **React components** that agents can return as rich UI. Declared as DSL blocks.

### 9.1 Component Definition

```
@component DiffView ```
  @props {
    original: str
    modified: str
    language: str = "typescript"
    show_line_numbers: bool = true
  }

  @state {
    collapsed: bool = false
    side_by_side: bool = true
  }

  @render
  <div class="diff-container">
    <div class="diff-header">
      <span>Diff: #{self.props.language}</span>
      <button on:click=#{fn() { self.state.collapsed = !self.state.collapsed }}>
        #{if self.state.collapsed { "Expand" } else { "Collapse" }}
      </button>
    </div>
    #{if !self.state.collapsed {
      <DiffRenderer
        original=#{self.props.original}
        modified=#{self.props.modified}
        mode=#{if self.state.side_by_side { "split" } else { "unified" }}
        language=#{self.props.language}
      />
    }}
  </div>

  @style {
    .diff-container {
      border: 1px solid #e2e8f0;
      border-radius: 8px;
      overflow: hidden;
    }
    .diff-header {
      display: flex;
      gap: 8px;
      padding: 8px 12px;
      background: #f7fafc;
      border-bottom: 1px solid #e2e8f0;
    }
  }
```
```

### 9.2 Component Directives

| Directive | Purpose |
|-----------|---------|
| `@props { ... }` | Prop types with optional defaults |
| `@state { ... }` | Internal state (compiles to React `useState`) |
| `@render` | JSX-like template with `#{}` captures |
| `@style { ... }` | Scoped CSS |

### 9.3 What Components Compile To

```javascript
// React functional component + TypeScript .d.ts
export function DiffView({ original, modified, language = "typescript", show_line_numbers = true }) {
  const [collapsed, setCollapsed] = useState(false);
  const [side_by_side, setSideBySide] = useState(true);
  return (
    <div className="diff-container">...</div>
  );
}
```

---

## 10. HTTP Server

HTTP servers are declared as DSL blocks using `@server`. Route handlers use `#{}` captures.

### 10.1 Server Declaration

```
@server app ```
  @port 3000
  @host "0.0.0.0"

  @middleware #{cors({ origins: ["*"] })}
  @middleware #{logger}

  @get /health #{fn(c: Context) -> Response {
    c.json({ status: "ok" })
  }}

  @post /chat #{async fn(c: Context) -> Response {
    let body = await c.req.json()
    let agent = Coder()
    let response = await agent.send(UserMessage {
      content: body.message,
      session_id: body.session_id ?? uuid()
    })
    c.json({ reply: response.text })
  }}

  @get /tools #{fn(c: Context) -> Response {
    c.json(Coder.tool_schemas())
  }}

  @post /tools/:name/invoke #{async fn(c: Context) -> Response {
    let tool_name = c.req.param("name")
    let args = await c.req.json()
    let result = await Coder.invoke_tool(tool_name, args)
    c.json(result)
  }}
```
```

### 10.2 Server Directives

| Directive | Purpose |
|-----------|---------|
| `@port` | Server port |
| `@host` | Bind address |
| `@middleware #{expr}` | Middleware registration |
| `@get /path #{handler}` | GET route |
| `@post /path #{handler}` | POST route |
| `@put /path #{handler}` | PUT route |
| `@delete /path #{handler}` | DELETE route |
| `@patch /path #{handler}` | PATCH route |

Path patterns support parameters (`:name`) and wildcards (`*`).

### 10.3 What Servers Compile To

```javascript
// Compiles to Hono-based HTTP server
import { Hono } from "hono";
import { serve } from "@agentscript/serve";

const app = new Hono();
app.get("/health", (c) => c.json({ status: "ok" }));
app.post("/chat", async (c) => { ... });
serve(app, { port: 3000, host: "0.0.0.0" });
```

---

## 11. Module System

### 11.1 Imports / Exports

```javascript
// Named exports (default)
// file: tools/fs.ag
/// Read a file from disk.
@tool
pub fn read_file(path: str) -> str { ... }

// Import
import { read_file, write_file } from "./tools/fs"

// Import all
import * as fs_tools from "./tools/fs"

// Import from packages
import { OpenAI } from "@agentscript/openai"
import { Anthropic } from "@agentscript/anthropic"

// Re-export
export { read_file, write_file } from "./tools/fs"
```

### 11.2 File Extension

`.ag` — AgentScript source files

### 11.3 Project Structure Convention

```
my-agent/
├── agent.toml              # project config (like package.json)
├── src/
│   ├── main.ag             # entry point
│   ├── agents/
│   │   ├── coder.ag
│   │   └── reviewer.ag
│   ├── tools/
│   │   ├── fs.ag
│   │   └── search.ag
│   ├── skills/
│   │   └── refactor.ag
│   ├── components/
│   │   └── diff_view.ag
│   └── server.ag           # HTTP routes
└── dist/                   # compiled output
    ├── index.js             # Node.js entry
    ├── package.json
    └── components/          # compiled React components
```

---

## 12. Standard Library

### 12.1 Layer A — Web API Externs (zero-cost)

These are `extern` declarations wrapping Web Standard APIs. No runtime code — they exist only for type checking.

```javascript
// std:web/fetch — the Fetch API
extern fn fetch(input: str | Request, init: RequestInit?) -> Promise<Response>
extern struct Response {
  status: int
  ok: bool
  headers: Headers
  fn json() -> Promise<any>
  fn text() -> Promise<str>
}
extern type Headers

// std:web/crypto
extern fn crypto_randomUUID() -> str

// std:web/timers
extern fn setTimeout(callback: () -> nil, ms: int) -> int
extern fn setInterval(callback: () -> nil, ms: int) -> int
```

### 12.2 Layer B — Runtime-backed Modules

These are AG code with `@js` extern bindings to a JS runtime package (`@agentscript/stdlib`).

```javascript
// std:http/server — HTTP server (Hono wrapper)
import { createApp, Context } from "std:http/server"

// std:http/client — HTTP client (fetch wrapper)
import { get, post, put, del } from "std:http/client"

// std:log — structured logging
import { log } from "std:log"

// std:fs — file system
import { readFile, writeFile } from "std:fs"

// std:env — environment variables
import { env } from "std:env"

// std:encoding — JSON/YAML/TOML
import { json, yaml, toml } from "std:encoding"
```

### 12.3 Future: Agent Runtime Modules

```javascript
// std:llm — raw LLM access (outside agent context)
import { LLM } from "std:llm"

let response = await LLM.complete({
  model: "claude-sonnet-4-20250514",
  messages: [
    { role: "system", content: "You are helpful." },
    { role: "user", content: "Hello!" }
  ],
  temperature: 0.7,
  max_tokens: 1024
})

// std:memory — conversation memory / vector store
import { Memory } from "std:memory"

// std:events — event bus
import { Events } from "std:events"
```

---

## 13. Grammar (Simplified BNF)

Designed for **recursive descent parsing** — no left recursion, clear lookaheads.

```ebnf
program         = (import_decl | declaration)* ;

(* === Top-level Declarations === *)
declaration     = dsl_decl
                | annotation_decl
                | extern_decl
                | fn_decl
                | struct_decl
                | enum_decl
                | type_decl
                | var_decl ";"
                ;

(* === DSL Blocks === *)
dsl_decl        = "@" IDENT IDENT ( dsl_inline | dsl_fileref ) ;
dsl_inline      = "```" dsl_content "```" ;
dsl_fileref     = "from" STRING ;
dsl_content     = (DSL_TEXT | "#{" expr "}" | "@" IDENT dsl_directive_args?)* ;
dsl_directive_args = STRING | "{" (IDENT ":" expr ","?)* "}" ;

(* === Annotations === *)
annotation_decl = annotation+ (fn_decl | extern_decl) ;
annotation      = "@" IDENT ( "(" annotation_args ")" )? ;
annotation_args = STRING ("," IDENT "=" STRING)* ;

(* === Extern Declarations === *)
extern_decl     = "extern" ( extern_fn | extern_struct | extern_type ) ;
extern_fn       = "fn" IDENT "(" params ")" ("->" type)? ;
extern_struct   = "struct" IDENT "{" extern_members "}" ;
extern_members  = (IDENT ":" type ","? | "fn" IDENT "(" params? ")" ("->" type)?)* ;
extern_type     = "type" IDENT ;

(* === Functions === *)
fn_decl         = "pub"? ("async")? "fn" IDENT "(" params? ")" ("->" type)? block ;
params          = param ("," param)* ;
param           = IDENT ":" type ("=" expr)? ;

(* === Struct / Enum / Type === *)
struct_decl     = "struct" IDENT "{" struct_fields "}" ;
struct_fields   = (IDENT ":" type ("=" expr)? ","?)* ;
enum_decl       = "enum" IDENT "{" enum_variants "}" ;
enum_variants   = (IDENT ("(" struct_fields ")")? ","?)* ;
type_decl       = "type" IDENT ("<" IDENT ">")? "=" type ;

(* === Types === *)
type            = base_type ("?" | "|" type | "[" "]")* ;
base_type       = "str" | "num" | "int" | "bool" | "nil" | "any"
                | IDENT ("<" type ">")?                   (* named type, optional generic *)
                | "(" params_types ")" "->" type          (* function type *)
                | "{" (IDENT ":" type ","?)* "}"          (* object type *)
                | "[" type "]"                             (* array type *)
                | "..." type                               (* variadic *)
                ;

(* === Expressions === *)
expr            = assignment ;
assignment      = pipe (("=" | "+=" | "-=" | "*=" | "/=") pipe)? ;
pipe            = ternary ("|>" ternary)* ;
ternary         = or ("?" expr ":" expr)? ;
or              = and ("||" and)* ;
and             = equality ("&&" equality)* ;
equality        = comparison (("==" | "!=") comparison)* ;
comparison      = addition (("<" | ">" | "<=" | ">=") addition)* ;
addition        = multiplication (("+" | "-") multiplication)* ;
multiplication  = unary (("*" | "/" | "%") unary)* ;
unary           = ("!" | "-") unary | postfix ;
postfix         = primary (call | index | member | "?" | "!")* ;
call            = "(" args? ")" ;
index           = "[" expr "]" ;
member          = "." IDENT | "::" IDENT ;
primary         = NUMBER | STRING | BOOL | "nil"
                | template_string
                | IDENT
                | "(" expr ")"
                | "[" (expr ("," expr)*)? "]"             (* array *)
                | "{" (IDENT ":" expr (",")?)* "}"        (* object *)
                | jsx_expr
                | "fn" "(" params? ")" ("->" type)? block
                | "(" params ")" "=>" (expr | block)      (* arrow fn *)
                | match_expr | if_expr
                ;

(* === Statements === *)
stmt            = var_decl ";"
                | expr ";"
                | ret_stmt
                | if_stmt
                | for_stmt
                | while_stmt
                | match_stmt
                | try_stmt
                | "yield" expr ";"
                | block
                ;
var_decl        = ("let" | "mut" | "const") IDENT (":" type)? "=" expr ;
ret_stmt        = "ret" expr? ";" ;
if_stmt         = "if" expr block ("else" (if_stmt | block))? ;
for_stmt        = "for" IDENT "in" expr block ;
while_stmt      = "while" expr block ;
match_stmt      = "match" expr "{" match_arms "}" ;
match_arms      = (pattern ("if" expr)? "=>" (expr | block))* ;
try_stmt        = "try" block "catch" IDENT block ;
block           = "{" stmt* expr? "}" ;              (* last expr = implicit return *)

(* === JSX === *)
jsx_expr        = "<" IDENT jsx_attrs? ">" jsx_children "</" IDENT ">"
                | "<" IDENT jsx_attrs? "/>"
                ;
jsx_attrs       = (IDENT "=" "{" expr "}" | IDENT "=" STRING | "on:" IDENT "=" "{" expr "}")* ;
jsx_children    = (jsx_expr | "{" expr "}" | JSX_TEXT)* ;

(* === Imports === *)
import_decl     = "import" "{" IDENT ("," IDENT)* "}" "from" STRING
                | "import" "*" "as" IDENT "from" STRING
                ;
```

---

## 14. Compilation Target

### 14.1 Output: Node.js Bundle

```
asc build src/main.ag --target node --outdir dist/
```

Produces:

```
dist/
├── package.json          # auto-generated with dependencies
├── index.js              # entry point (ESM)
├── agents/               # compiled agent runtime configs
├── tools/                # tool functions + JSON schemas
├── skills/               # skill runtime configs
├── components/           # React components (.jsx) + .d.ts
├── server.js             # Hono HTTP server
└── schemas/              # OpenAPI spec, tool schemas
```

### 14.2 Compilation Pipeline

```
Source (.ag)
    │
    ▼
  Lexer          → Token stream (with DSL raw mode for @blocks)
    │
    ▼
  Parser         → AST (recursive descent, LL(1))
    │               includes DslBlock nodes + Annotation nodes
    ▼
  Type Checker   → Annotated AST (basic structural typing)
    │               validates captures inside DSL blocks
    ▼
  Code Gen       → JavaScript (ESM) + TypeScript declarations
    │               dispatches DslBlock to registered DslHandlers
    │               processes @tool annotations for schema gen
    ▼
  Bundler        → Node.js package (with package.json, deps)
```

### 14.3 What Each Construct Compiles To

| AgentScript | JavaScript Output |
|-------------|-------------------|
| `@agent Coder ``` ... ``` ` | `new AgentRuntime({ model, tools, messages, ... })` |
| `@tool fn read_file(...)` | `function read_file(...) { ... }` + `read_file.schema = { ... }` |
| `@skill refactor ``` ... ``` ` | `new SkillRuntime({ description, inputSchema, steps, ... })` |
| `@component DiffView ``` ... ``` ` | React functional component + `.d.ts` |
| `@server app ``` ... ``` ` | Hono route registrations + serve() |
| `@prompt system ``` ... ``` ` | `new PromptTemplate({ messages: [...] })` |
| `struct User { ... }` | TypeScript interface (type-only, erased) |
| `enum Status { ... }` | Tagged union: `{ tag: "Active", since: "..." }` |
| `extern fn fetch(...)` | Erased (import generated if `@js`) |
| `extern struct Response { ... }` | Erased (type-only) |

### 14.4 Runtime Library

The compiled output depends on a small runtime:

```javascript
// @agentscript/runtime (npm package)
import { AgentRuntime } from "@agentscript/runtime"
import { SkillRuntime } from "@agentscript/runtime"
import { PromptTemplate } from "@agentscript/prompt-runtime"

// @agentscript/stdlib (npm package) — Layer B runtime
import { createApp } from "@agentscript/stdlib/http/server"
import { get, post } from "@agentscript/stdlib/http/client"

// @agentscript/serve (npm package) — Node.js server runner
import { serve } from "@agentscript/serve"
```

---

## 15. Complete Example

```javascript
// file: src/main.ag

import { createApp, Context } from "std:http/server"
import { readFile, writeFile } from "std:fs"

// --- Extern: Web Fetch API ---
extern fn fetch(url: str) -> Promise<Response>
extern struct Response {
  status: int
  ok: bool
  fn json() -> Promise<any>
  fn text() -> Promise<str>
}

// --- Tools: annotated functions ---

/// Read a file from disk.
/// @param path - Absolute or relative file path
@tool
fn read_file(path: str) -> str | Error {
  await readFile(path, "utf-8")
}

/// Write content to a file.
/// @param path - File path to write to
/// @param content - Content to write
@tool
fn write_file(path: str, content: str) -> nil | Error {
  await writeFile(path, content)
}

// --- Agent: DSL block ---

@agent CodeBot ```
  @model claude-sonnet-4-20250514
  @tools #{[read_file, write_file]}

  @role system
  You are a concise coding assistant.
  When asked to edit code, always show a diff.

  @constraints {
    temperature: 0.2
    max_tokens: 4096
  }
```

// --- Component: DSL block ---

@component CodeCard ```
  @props {
    code: str
    language: str = "text"
    title: str?
  }

  @render
  <div class="code-card">
    #{if self.props.title {
      <h3>#{self.props.title}</h3>
    }}
    <pre><code class="language-#{self.props.language}">
      #{self.props.code}
    </code></pre>
  </div>
```

// --- Server: DSL block ---

@server app ```
  @port 3000

  @get /health #{fn(c: Context) -> Response {
    c.json({ status: "ok" })
  }}

  @post /chat #{async fn(c: Context) -> Response {
    let body = await c.req.json()
    let bot = CodeBot()
    let response = await bot.send(UserMessage {
      content: body.message
    })
    c.json({ reply: response.text })
  }}

  @get /tools #{fn(c: Context) -> Response {
    c.json(CodeBot.tool_schemas())
  }}
```
```

---

## 16. Compiler Implementation Notes

### For Recursive Descent

The grammar is designed so each production can be identified by **one token of lookahead**:

| Lookahead Token | Production |
|-----------------|------------|
| `@` | DSL block or annotation — peek ahead to disambiguate |
| `extern` | extern_decl |
| `fn` | fn_decl |
| `struct` | struct_decl |
| `enum` | enum_decl |
| `type` | type_decl |
| `let` / `mut` / `const` | var_decl |
| `import` | import_decl |
| `pub` | peek next token for the actual decl |
| `///` | accumulate doc comments, attach to next decl |

**`@` disambiguation:**
- `@ IDENT IDENT (``` | from)` → DSL block (`@prompt sys ``` ... ```)
- `@ IDENT ( "(" ... ")" )? (fn | extern)` → Annotation (`@tool fn ...`, `@js("mod") extern ...`)

**DSL raw mode:** When the parser enters a DSL block (after the opening ` ``` `), the lexer switches to raw mode. It emits `DslText` tokens for regular content and `DslCaptureStart`/`DslCaptureEnd` around `#{ }` captures. Inside captures, normal AG tokenization resumes with brace nesting tracking. The closing ` ``` ` at line start ends raw mode.

**Expression parsing** uses standard Pratt / precedence-climbing — each level is a separate function (`parse_or`, `parse_and`, `parse_equality`, etc.).

### Type Checker Scope

Intentionally minimal:

1. **Resolve declarations** — build symbol table per scope
2. **Infer `let` bindings** — from right-hand side
3. **Check function calls** — param count + basic type compatibility
4. **Check struct field access** — field existence
5. **Narrow unions in `match`** — each arm knows the variant
6. **Validate DSL captures** — type-check `#{}` expressions inside DSL blocks
7. **Validate extern calls** — check arguments against declared signatures
8. **Validate @tool functions** — all params must be serializable types (for JSON Schema)

That's it. No generics inference, no HKT, no variance analysis.

### DSL Handler Pipeline

During codegen, `DslBlock` nodes are dispatched to registered handlers:

| DSL Kind | Handler | Output |
|----------|---------|--------|
| `prompt` | `PromptDslHandler` | `PromptTemplate` constructor |
| `agent` | `AgentDslHandler` | `AgentRuntime` constructor |
| `skill` | `SkillDslHandler` | `SkillRuntime` constructor |
| `component` | `ComponentDslHandler` | React component |
| `server` | `ServerDslHandler` | Hono routes + serve() |

Unregistered DSL kinds produce a compile error.

---

## 17. Reserved for Future

- `trait` / `impl` for shared interfaces
- `test` blocks for inline testing
- `deploy` directive for cloud deployment targets
- `workflow` for multi-agent DAGs with retry/backoff
- DSL statement block captures (`#{ ... statements ... }`)
- WASM compilation target
- Visual graph editor that round-trips to `.ag` source
- Custom DSL handler API for user-defined `@kind` blocks

---

## Appendix A: Keyword Comparison

| AgentScript | JavaScript | Reason |
|-------------|------------|--------|
| `fn` | `function` | 2 chars vs 8 |
| `let` | `const` | immutable by default |
| `mut` | `let` | explicit mutability |
| `ret` | `return` | 3 chars vs 6 |
| `str` | `string` | 3 chars vs 6 |
| `num` | `number` | 3 chars vs 6 |
| `int` | — | explicit integer type |
| `nil` | `null` / `undefined` | unified, 3 chars |
| `match` | `switch` | pattern matching, not fall-through |
| `pub` | `export` | 3 chars vs 6 |
| `extern` | — | declare JS bindings (no body) |
| `@tool` | — | annotation: mark fn as LLM-callable tool |
| `@js("mod")` | — | annotation: bind extern to JS module |
| `@agent` | — | DSL block: agent declaration |
| `@prompt` | — | DSL block: prompt template |
| `@skill` | — | DSL block: multi-step skill recipe |
| `@component` | — | DSL block: React component |
| `@server` | — | DSL block: HTTP server |
