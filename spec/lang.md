# AgentScript Language Specification v0.1

> A purpose-built language for authoring AI agents — with first-class primitives for prompts, tools, skills, components, and HTTP serving.

---

## 1. Design Philosophy

| Principle | Rationale |
|-----------|-----------|
| **JS-flavored, shorter keywords** | Familiar to web devs, less visual noise |
| **LL(1) grammar, recursive descent** | Simple compiler front-end, no parser generator needed |
| **Agent-first primitives** | `agent`, `tool`, `skill`, `component`, `prompt` are keywords, not library patterns |
| **Structural typing + basic inference** | Enough type safety to catch wiring bugs; no generics gymnastics |
| **Compiles to Node.js bundle** | Instant production readiness; other languages access via HTTP/SDK |

---

## 2. Lexical Conventions

### 2.1 Keywords (reserved)

```
agent    tool     skill    component  prompt
fn       let      const    mut        if
else     for      in       of         while
match    ret      yield    await      async
import   export   from     as         type
struct   enum     impl     pub        self
true     false    nil      http       route
emit     use      with     on         _
```

### 2.2 Operators & Punctuation

```
+  -  *  /  %  **          // arithmetic
== != < > <= >=            // comparison
&& || !                    // logical
|> ?? ?.                   // pipe, nullish coalesce, optional chain
=  +=  -=  *=  /=         // assignment
=> -> ::                   // fat arrow, thin arrow (return type), scope resolution
@                          // decorator / annotation
#                          // prompt interpolation
{ } ( ) [ ] < >           // grouping
, ; : .                    // delimiters
..  ...                    // range, spread
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

### 2.5 Prompt Literals (new!)

Triple-backtick blocks are **prompt literals** — a DSL for structured LLM instructions:

```
let system = ```
  @role system
  @model gpt-4o | claude-3.5-sonnet

  You are a helpful coding assistant.

  ## Context
  The user is working on: #{project.description}

  ## Rules
  - Always respond in #{lang}
  - Keep responses under #{max_tokens} tokens

  @examples {
    user: "Fix this bug"
    assistant: "I'll analyze the code..."
  }

  @constraints {
    temperature: 0.7
    max_tokens: 2048
  }
```

Inside prompt literals:
- `#{ expr }` — expression interpolation (evaluated at runtime)
- `@keyword` — prompt metadata directives
- Everything else is verbatim text sent to the model

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

## 5. Agent System (Core Primitive)

### 5.1 Agent Declaration

```javascript
/// A coding assistant that can read and write files.
/// @version 1.0
agent Coder {
  /// The system prompt — uses prompt DSL
  prompt = ```
    @role system
    You are an expert software engineer.

    ## Your capabilities
    You can read files, write code, and run tests.

    ## Style
    - Be concise
    - Prefer #{self.preferred_lang} idioms
    - Always explain your reasoning

    @constraints {
      model: "claude-sonnet-4-20250514"
      temperature: 0.3
      max_tokens: 4096
    }
  ```

  /// Agent-level state
  preferred_lang: str = "TypeScript"
  context_window: int = 128000

  /// Tools this agent can use
  use read_file
  use write_file
  use run_tests
  use web_search

  /// Skills (compound tool sequences)
  use refactor_skill
  use debug_skill

  /// Lifecycle hooks
  on init(ctx: AgentContext) {
    log.info("Coder agent initialized for session ${ctx.session_id}")
  }

  on message(msg: UserMessage) -> AgentResponse {
    // Custom pre/post processing around the LLM call
    let enriched = enrich_context(msg, self.context_window)
    let response = await self.complete(enriched)
    response
  }

  on error(e: Error) {
    log.error("Agent error: ${e.msg}")
    emit event::agent_error(e)
  }
}
```

### 5.2 Agent Composition

```javascript
/// Orchestrator that delegates to sub-agents
agent TeamLead {
  prompt = ```
    @role system
    You coordinate a team of specialist agents.
    Delegate tasks to the right agent based on the request.
  ```

  // Sub-agents
  use agent Coder
  use agent Reviewer
  use agent Designer

  on message(msg: UserMessage) -> AgentResponse {
    let plan = await self.complete(```
      Given this request: #{msg.content}
      Which agent should handle it? Respond with: coder, reviewer, or designer.
    ```)

    match plan.choice {
      "coder" => await Coder.send(msg)
      "reviewer" => await Reviewer.send(msg)
      "designer" => await Designer.send(msg)
      _ => await self.complete(msg)  // handle directly
    }
  }
}
```

---

## 6. Tool Declarations

Tools are typed functions that agents can invoke via tool-calling.

### 6.1 Basic Tool

```javascript
/// Read the contents of a file at the given path.
/// @param path - Absolute or relative file path
/// @returns The file contents as a string
tool read_file(path: str) -> str | Error {
  let content = await fs.read(path)?
  content
}

/// Search the web for a query.
/// @param query - The search query
/// @param max_results - Maximum number of results (default: 5)
tool web_search(query: str, max_results: int = 5) -> [SearchResult] {
  let results = await http.get("https://api.search.com/v1", {
    params: { q: query, limit: max_results }
  })?
  results.items
}
```

### 6.2 Tool Metadata (for LLM function-calling schema)

The compiler automatically generates JSON Schema for tool-calling from:
1. The function signature (param names, types)
2. Doc comments (`///`)
3. `@param` / `@returns` annotations

```javascript
// This tool declaration:
/// Calculate the area of a shape.
/// @param shape - The type of shape: "circle", "rect", or "triangle"
/// @param dims - Dimensions object
tool calc_area(shape: str, dims: {w: num?, h: num?, r: num?}) -> num {
  match shape {
    "circle" => 3.14159 * (dims.r ?? 0) ** 2
    "rect" => (dims.w ?? 0) * (dims.h ?? 0)
    "triangle" => 0.5 * (dims.w ?? 0) * (dims.h ?? 0)
    _ => Error("unknown shape: ${shape}")
  }
}

// Compiles to this JSON Schema automatically:
// {
//   "name": "calc_area",
//   "description": "Calculate the area of a shape.",
//   "parameters": {
//     "type": "object",
//     "properties": {
//       "shape": { "type": "string", "description": "The type of shape: \"circle\", \"rect\", or \"triangle\"" },
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

## 7. Skill Declarations

Skills are **multi-step, compound tool sequences** — like macros over tools.

```javascript
/// Skill to refactor code: reads, analyzes, rewrites, and tests.
skill refactor_skill {
  /// Description shown to the LLM so it knows when to invoke this skill
  description = "Refactor code in a file to improve quality, readability, and performance."

  /// Input schema
  input {
    file_path: str
    goals: [str]          // e.g. ["reduce complexity", "improve naming"]
    dry_run: bool = false
  }

  /// The execution steps
  steps(input) {
    // Step 1: Read the file
    let code = await read_file(input.file_path)?

    // Step 2: Ask the agent to analyze
    let analysis = await self.agent.complete(```
      Analyze this code for refactoring opportunities.
      Goals: #{input.goals.join(", ")}

      ```#{code}```
    ```)

    // Step 3: Generate refactored version
    let new_code = await self.agent.complete(```
      Apply these refactorings to the code:
      #{analysis.content}

      Original:
      ```#{code}```
    ```)

    // Step 4: Write and test
    if !input.dry_run {
      await write_file(input.file_path, new_code.content)?
      let test_result = await run_tests()?
      if test_result.failed > 0 {
        // Rollback
        await write_file(input.file_path, code)?
        ret Error("Refactoring broke ${test_result.failed} tests, rolled back.")
      }
    }

    {
      original: code,
      refactored: new_code.content,
      analysis: analysis.content,
      applied: !input.dry_run
    }
  }
}
```

---

## 8. Component Declarations

Components emit **React components** that agents can return as rich UI.

### 8.1 Component Definition

```javascript
/// A card that displays a code diff.
/// @prop original - The original code
/// @prop modified - The modified code
/// @prop language - Syntax highlighting language
component DiffView {
  props {
    original: str
    modified: str
    language: str = "typescript"
    show_line_numbers: bool = true
  }

  /// Optional state (compiled to React useState)
  state {
    collapsed: bool = false
    side_by_side: bool = true
  }

  /// The render body — JSX-like syntax
  render {
    <div class="diff-container">
      <div class="diff-header">
        <span>"Diff: #{self.props.language}"</span>
        <button on:click={() => self.state.collapsed = !self.state.collapsed}>
          {if self.state.collapsed { "Expand" } else { "Collapse" }}
        </button>
        <button on:click={() => self.state.side_by_side = !self.state.side_by_side}>
          {if self.state.side_by_side { "Unified" } else { "Side by Side" }}
        </button>
      </div>
      {if !self.state.collapsed {
        <DiffRenderer
          original={self.props.original}
          modified={self.props.modified}
          mode={if self.state.side_by_side { "split" } else { "unified" }}
          language={self.props.language}
        />
      }}
    </div>
  }

  /// Styles (scoped CSS)
  style {
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
}
```

### 8.2 Component Usage in Agents

```javascript
agent Coder {
  // ...

  on message(msg: UserMessage) -> AgentResponse {
    let result = await self.complete(msg)

    // Agent can return components as rich responses
    if result.has_diff {
      ret AgentResponse {
        text: result.explanation,
        component: <DiffView
          original={result.original_code}
          modified={result.new_code}
          language="typescript"
        />
      }
    }

    result
  }
}
```

### 8.3 Component Type Generation

The compiler generates TypeScript definitions from component declarations:

```typescript
// Auto-generated: DiffView.d.ts
export interface DiffViewProps {
  /** The original code */
  original: string;
  /** The modified code */
  modified: string;
  /** Syntax highlighting language */
  language?: string;
  /** Whether to show line numbers */
  show_line_numbers?: boolean;
}

export declare const DiffView: React.FC<DiffViewProps>;
```

---

## 9. HTTP Server

A minimal, built-in HTTP layer — no framework needed.

### 9.1 Route Declarations

```javascript
/// Declare an HTTP server
http server {
  port: 3000
  host: "0.0.0.0"

  /// Middleware
  use cors({ origins: ["*"] })
  use auth_bearer(validate: verify_token)
  use logger

  /// Health check
  route GET /health {
    { status: "ok", uptime: process.uptime() }
  }

  /// Agent chat endpoint
  route POST /chat {
    let body = req.json()?
    let agent = Coder()
    let response = await agent.send(UserMessage {
      content: body.message,
      session_id: body.session_id ?? uuid()
    })
    response
  }

  /// Streaming agent response
  route POST /chat/stream {
    let body = req.json()?
    let agent = Coder()

    // SSE streaming
    res.stream(content_type: "text/event-stream") {
      await agent.stream(body.message) |> on chunk {
        emit "data: ${json(chunk)}\n\n"
      }
    }
  }

  /// Serve a component
  route GET /ui/diff {
    res.component(<DiffView
      original={req.query.original ?? ""}
      modified={req.query.modified ?? ""}
    />)
  }

  /// Static files
  route GET /static/* {
    res.static("./public")
  }

  /// SDK endpoint: list available tools
  route GET /tools {
    agent::Coder.tool_schemas()
  }

  /// SDK endpoint: invoke a tool directly
  route POST /tools/:name/invoke {
    let tool_name = req.params.name
    let args = req.json()?
    let result = await agent::Coder.invoke_tool(tool_name, args)?
    result
  }
}
```

### 9.2 Request / Response Types

```javascript
// Built-in types (no import needed)
struct Request {
  method: str
  path: str
  headers: {str: str}
  query: {str: str}
  params: {str: str}       // route params like :name
  body: any                // raw body

  fn json() -> any | Error    // parse JSON body
  fn text() -> str | Error    // read as text
}

struct Response {
  status: int = 200
  headers: {str: str} = {}
  body: any

  fn json(data: any) -> Response
  fn text(s: str) -> Response
  fn stream(content_type: str, handler: fn) -> Response
  fn component(c: Component) -> Response    // SSR a component
  fn static(dir: str) -> Response
}
```

---

## 10. Module System

### 10.1 Imports / Exports

```javascript
// Named exports (default)
// file: tools/fs.as
pub tool read_file(path: str) -> str { ... }
pub tool write_file(path: str, content: str) -> nil { ... }

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

### 10.2 File Extension

`.as` — AgentScript source files

### 10.3 Project Structure Convention

```
my-agent/
├── agent.toml              # project config (like package.json)
├── src/
│   ├── main.as             # entry point
│   ├── agents/
│   │   ├── coder.as
│   │   └── reviewer.as
│   ├── tools/
│   │   ├── fs.as
│   │   └── search.as
│   ├── skills/
│   │   └── refactor.as
│   ├── components/
│   │   └── diff_view.as
│   └── server.as           # HTTP routes
└── dist/                   # compiled output
    ├── index.js             # Node.js entry
    ├── package.json
    └── components/          # compiled React components
```

---

## 11. Standard Library

### 11.1 Built-in Modules

```javascript
// Core
import { log } from "std:log"            // structured logging
import { json, yaml, toml } from "std:encoding"
import { uuid, hash, base64 } from "std:crypto"
import { sleep, timeout } from "std:time"

// IO
import { fs } from "std:fs"              // file system
import { http } from "std:http"          // HTTP client
import { env } from "std:env"            // environment variables

// Agent runtime
import { LLM } from "std:llm"            // raw LLM access
import { Memory } from "std:memory"      // conversation memory / vector store
import { Events } from "std:events"      // event bus
```

### 11.2 LLM Abstraction

```javascript
import { LLM } from "std:llm"

// Direct LLM call (outside agent context)
let response = await LLM.complete({
  model: "claude-sonnet-4-20250514",
  messages: [
    { role: "system", content: "You are helpful." },
    { role: "user", content: "Hello!" }
  ],
  temperature: 0.7,
  max_tokens: 1024
})

// Streaming
await LLM.stream({ ... }) |> on chunk {
  process.stdout.write(chunk.text)
}
```

---

## 12. Grammar (Simplified BNF)

Designed for **recursive descent parsing** — no left recursion, clear lookaheads.

```ebnf
program         = (import_decl | declaration)* ;

(* === Top-level Declarations === *)
declaration     = agent_decl
                | tool_decl
                | skill_decl
                | component_decl
                | fn_decl
                | struct_decl
                | enum_decl
                | type_decl
                | http_decl
                | var_decl ";"
                ;

(* === Agent === *)
agent_decl      = "agent" IDENT "{" agent_body "}" ;
agent_body      = (agent_member)* ;
agent_member    = "prompt" "=" prompt_literal
                | IDENT ":" type ("=" expr)?             (* state field *)
                | "use" ("agent")? IDENT                 (* tool/skill/sub-agent binding *)
                | "on" IDENT "(" params? ")" ("->" type)? block
                ;

(* === Tool === *)
tool_decl       = doc_comments? "pub"? "tool" IDENT "(" params ")" "->" type block ;

(* === Skill === *)
skill_decl      = "skill" IDENT "{" skill_body "}" ;
skill_body      = "description" "=" STRING
                | "input" "{" struct_fields "}"
                | "steps" "(" IDENT ")" block
                ;

(* === Component === *)
component_decl  = "component" IDENT "{" component_body "}" ;
component_body  = "props" "{" struct_fields "}"
                | "state" "{" struct_fields "}"
                | "render" "{" jsx_expr "}"
                | "style" "{" css_block "}"
                ;

(* === HTTP === *)
http_decl       = "http" "server" "{" http_body "}" ;
http_body       = http_field | http_use | route_decl ;
http_field      = IDENT ":" expr ;
http_use        = "use" call_expr ;
route_decl      = "route" HTTP_METHOD path_pattern block ;
HTTP_METHOD     = "GET" | "POST" | "PUT" | "DELETE" | "PATCH" ;
path_pattern    = ("/" (IDENT | ":" IDENT | "*"))+ ;

(* === Functions === *)
fn_decl         = "pub"? ("async")? "fn" IDENT "(" params? ")" ("->" type)? block ;
params          = param ("," param)* ;
param           = IDENT ":" type ("=" expr)? ;

(* === Struct / Enum / Type === *)
struct_decl     = "struct" IDENT "{" struct_fields "}" ;
struct_fields   = (IDENT ":" type ("=" expr)? ","?)* ;
enum_decl       = "enum" IDENT "{" enum_variants "}" ;
enum_variants   = (IDENT ("(" struct_fields ")")? ","?)* ;
type_decl       = "type" IDENT ("< " IDENT ">")? "=" type ;

(* === Types === *)
type            = base_type ("?" | "|" type | "[" "]")* ;
base_type       = "str" | "num" | "int" | "bool" | "nil" | "any"
                | IDENT
                | "(" params_types ")" "->" type       (* function type *)
                | "{" (IDENT ":" type ","?)* "}"       (* object type *)
                | "[" type "]"                          (* array type *)
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
                | template_string | prompt_literal
                | IDENT
                | "(" expr ")"
                | "[" (expr ("," expr)*)? "]"          (* array *)
                | "{" (IDENT ":" expr ("," )?)* "}"    (* object *)
                | jsx_expr
                | "fn" "(" params? ")" ("->" type)? block
                | "(" params ")" "=>" (expr | block)   (* arrow fn *)
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

(* === Prompt Literal === *)
prompt_literal  = "```" prompt_content "```" ;
prompt_content  = (PROMPT_TEXT | "#{" expr "}" | "@" IDENT prompt_meta)* ;
prompt_meta     = STRING | "{" (IDENT ":" expr)* "}" ;

(* === Imports === *)
import_decl     = "import" "{" IDENT ("," IDENT)* "}" "from" STRING
                | "import" "*" "as" IDENT "from" STRING
                ;
```

---

## 13. Compilation Target

### 13.1 Output: Node.js Bundle

```
asc build src/main.as --target node --outdir dist/
```

Produces:

```
dist/
├── package.json          # auto-generated with dependencies
├── index.js              # entry point (ESM)
├── agents/               # compiled agent classes
├── tools/                # tool functions + JSON schemas
├── skills/               # skill executors
├── components/           # React components (.jsx) + .d.ts
├── server.js             # Express/Fastify HTTP server
└── schemas/              # OpenAPI spec, tool schemas
```

### 13.2 Compilation Pipeline

```
Source (.as)
    │
    ▼
  Lexer          → Token stream
    │
    ▼
  Parser         → AST (recursive descent, LL(1))
    │
    ▼
  Type Checker   → Annotated AST (basic structural typing)
    │
    ▼
  IR             → Normalized intermediate form
    │
    ▼
  Code Gen       → JavaScript (ESM) + TypeScript declarations
    │
    ▼
  Bundler        → Node.js package (with package.json, deps)
```

### 13.3 What Each Construct Compiles To

| AgentScript | JavaScript Output |
|-------------|-------------------|
| `agent Coder { ... }` | `class Coder extends AgentRuntime { ... }` |
| `tool read_file(...)` | `function read_file(...)` + `read_file.schema = { ... }` |
| `skill refactor_skill { ... }` | `class RefactorSkill extends SkillRuntime { ... }` |
| `component DiffView { ... }` | React functional component + `.d.ts` |
| `http server { ... }` | Fastify/Express route registrations |
| `prompt = \`\`\`...\`\`\`` | Template literal function with metadata |
| `struct User { ... }` | TypeScript interface (type-only, erased) |
| `enum Status { ... }` | Tagged union: `{ tag: "Active", since: "..." }` |

### 13.4 Runtime Library

The compiled output depends on a small runtime:

```javascript
// @agentscript/runtime (npm package)
import {
  AgentRuntime,     // base class for agents
  SkillRuntime,     // base class for skills
  ToolRegistry,     // tool registration + schema extraction
  PromptBuilder,    // prompt literal evaluation
  createServer      // minimal HTTP server factory
} from "@agentscript/runtime"
```

---

## 14. Complete Example

```javascript
// file: src/main.as

import { Anthropic } from "@agentscript/anthropic"
import { read_file, write_file } from "./tools/fs"

/// A minimal code assistant agent.
agent CodeBot {
  prompt = ```
    @role system
    @model claude-sonnet-4-20250514

    You are a concise coding assistant.
    When asked to edit code, always show a diff.

    @constraints {
      temperature: 0.2
      max_tokens: 4096
    }
  ```

  use read_file
  use write_file

  on message(msg: UserMessage) -> AgentResponse {
    await self.complete(msg)
  }
}

/// Read a file from disk.
tool read_file(path: str) -> str | Error {
  await fs.read(path)
}

/// Write content to a file.
tool write_file(path: str, content: str) -> nil | Error {
  await fs.write(path, content)
}

/// A simple card component for displaying code.
component CodeCard {
  props {
    code: str
    language: str = "text"
    title: str?
  }

  render {
    <div class="code-card">
      {if self.props.title {
        <h3>{self.props.title}</h3>
      }}
      <pre><code class="language-#{self.props.language}">
        {self.props.code}
      </code></pre>
    </div>
  }
}

/// HTTP server — the entry point
http server {
  port: env.PORT ?? 3000

  use cors({ origins: ["*"] })

  route GET /health {
    { status: "ok" }
  }

  route POST /chat {
    let body = req.json()?
    let bot = CodeBot()
    let response = await bot.send(UserMessage {
      content: body.message
    })
    { reply: response.text }
  }

  route GET /tools {
    CodeBot.tool_schemas()
  }
}
```

---

## 15. Compiler Implementation Notes

### For Recursive Descent

The grammar is designed so each production can be identified by **one token of lookahead**:

| Lookahead Token | Production |
|-----------------|------------|
| `agent` | agent_decl |
| `tool` | tool_decl |
| `skill` | skill_decl |
| `component` | component_decl |
| `fn` | fn_decl |
| `struct` | struct_decl |
| `enum` | enum_decl |
| `type` | type_decl |
| `http` | http_decl |
| `let` / `mut` / `const` | var_decl |
| `import` | import_decl |
| `pub` | peek next token for the actual decl |
| `///` | accumulate doc comments, attach to next decl |

**Expression parsing** uses standard Pratt / precedence-climbing — each level is a separate function (`parse_or`, `parse_and`, `parse_equality`, etc.).

### Type Checker Scope

Intentionally minimal:

1. **Resolve declarations** — build symbol table per scope
2. **Infer `let` bindings** — from right-hand side
3. **Check function calls** — param count + basic type compatibility
4. **Check struct field access** — field existence
5. **Narrow unions in `match`** — each arm knows the variant
6. **Tool schema validation** — all params must be serializable types
7. **Component prop checking** — JSX usage matches declared props

That's it. No generics inference, no HKT, no variance analysis.

---

## 16. Reserved for Future

- `trait` / `impl` for shared interfaces
- `test` blocks for inline testing
- `deploy` directive for cloud deployment targets
- `workflow` for multi-agent DAGs with retry/backoff
- WASM compilation target
- Visual graph editor that round-trips to `.as` source

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
| `use` | — | agent binding (new concept) |
| `agent` | — | first-class agent (new) |
| `tool` | — | first-class tool (new) |
| `skill` | — | first-class skill (new) |
| `component` | — | first-class component (new) |
| `prompt` | — | first-class prompt DSL (new) |
