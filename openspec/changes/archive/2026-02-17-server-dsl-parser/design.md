## Context

Based on the `extensible-dsl-system` change's generic DSL framework (`@kind name ``` ... ``` `) and the `prompt-dsl` change's pattern of independent DSL crates, this change implements the server DSL handler: a lexer, parser, and validator for `@server` blocks.

Server DSL blocks declare HTTP servers with routes, middleware, host/port configuration. Each DSL crate is independent with its own lexer/parser/AST/validator pipeline, taking `Vec<DslPart>` as input from the framework.

## Goals / Non-Goals

**Goals:**
- New crate `ag-dsl-server` with lexer, parser, validator
- Parse server directives: `@port`, `@host`, `@middleware`, `@get`, `@post`, `@put`, `@delete`, `@patch`
- Route directives accept path patterns with literal segments, named params, and wildcards
- Route handlers and middleware reference host expressions via `#{}` captures
- Produce a `ServerTemplate` AST suitable for downstream codegen
- Validate structural constraints (no duplicate routes, required fields)

**Non-Goals:**
- No codegen in this change (codegen is a separate follow-up)
- No runtime HTTP server implementation
- No route conflict detection beyond exact method+path duplicates
- No query parameter or header parsing
- No WebSocket or SSE directives

## Decisions

### 1. Crate Structure

```
crates/
├── ag-dsl-server/
│   ├── src/
│   │   ├── lib.rs            # public API
│   │   ├── lexer.rs          # server DSL lexer
│   │   ├── ast.rs            # ServerTemplate AST
│   │   ├── parser.rs         # server DSL parser
│   │   └── validator.rs      # structural validation
│   └── tests/
│       ├── lexer_tests.rs
│       ├── parser_tests.rs
│       └── validator_tests.rs
```

**Dependency direction**:

```
ag-dsl-server → ag-dsl-core
```

`ag-dsl-server` does **not** depend on `ag-ast`, `ag-lexer`, `ag-parser`, or SWC crates. It receives `Vec<DslPart>` from the DSL framework and produces its own AST. Codegen (which will need SWC) is out of scope.

### 2. Server DSL Directives

The server DSL recognizes the following directives inside `@server` blocks:

| Directive     | Syntax                              | Semantics                          |
|---------------|-------------------------------------|------------------------------------|
| `@port`       | `@port 3000`                        | Port number (integer literal)      |
| `@host`       | `@host "0.0.0.0"`                   | Host string (string literal)       |
| `@middleware`  | `@middleware #{expr}`               | Middleware via capture expression  |
| `@get`        | `@get /path/:param #{handler}`      | GET route with path + handler      |
| `@post`       | `@post /path #{handler}`            | POST route                         |
| `@put`        | `@put /path/:id #{handler}`         | PUT route                          |
| `@delete`     | `@delete /path/:id #{handler}`      | DELETE route                       |
| `@patch`      | `@patch /path/:id #{handler}`       | PATCH route                        |

Directive recognition follows the same rule as prompt DSL: line-start `@` followed by a known keyword. Unknown `@` sequences inside a server block are errors (unlike prompt DSL which treats them as text).

### 3. Path Pattern Parsing

Route directives (`@get`, `@post`, `@put`, `@delete`, `@patch`) require a path pattern between the directive keyword and the handler capture. Path patterns are parsed as a sequence of segments:

```rust
enum PathSegment {
    Literal(String),    // /health, /api, /users
    Param(String),      // /:id, /:name
    Wildcard,           // /*
}
```

**Rules:**
- Paths must start with `/`
- Segments are separated by `/`
- `:ident` after `/` is a named parameter
- `*` after `/` is a wildcard (must be the last segment)
- Empty segments (`//`) are forbidden
- Literal segments may contain `a-z`, `A-Z`, `0-9`, `-`, `_`

**Examples:**
- `/health` → `[Literal("health")]`
- `/users/:id` → `[Literal("users"), Param("id")]`
- `/api/v1/:resource/*` → `[Literal("api"), Literal("v1"), Param("resource"), Wildcard]`
- `/` → `[]` (root path, empty segment list)

### 4. Server Template AST

```rust
/// A complete server template
struct ServerTemplate {
    name: String,
    port: Option<u16>,
    host: Option<String>,
    middlewares: Vec<usize>,        // capture indices
    routes: Vec<Route>,
}

struct Route {
    method: HttpMethod,
    path: Vec<PathSegment>,
    handler_capture: usize,         // index into DslPart captures
}

enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

enum PathSegment {
    Literal(String),
    Param(String),
    Wildcard,
}
```

**Design notes:**
- `port` and `host` are optional; defaults are left to codegen/runtime
- `middlewares` is an ordered list; middleware ordering matters for execution
- Each route stores its handler as a capture index referencing the original `DslPart::Capture` position
- `ServerTemplate.name` comes from the DSL block name (`@server my_api ``` ... ```)

### 5. Lexer Design

The server lexer accepts `Vec<DslPart>` and produces `Vec<ServerToken>`:

```rust
enum ServerToken {
    // Directives
    DirectivePort,          // @port
    DirectiveHost,          // @host
    DirectiveMiddleware,    // @middleware
    DirectiveGet,           // @get
    DirectivePost,          // @post
    DirectivePut,           // @put
    DirectiveDelete,        // @delete
    DirectivePatch,         // @patch

    // Values
    NumberLiteral(u64),     // 3000, 8080
    StringLiteral(String),  // "0.0.0.0"
    Path(String),           // /users/:id/posts — raw path text

    // Captures
    Capture(usize),         // index into original DslPart captures

    Eof,
}
```

**Key decisions:**
- Path patterns are captured as a single `Path(String)` token by the lexer. The parser is responsible for splitting the path into segments and validating the structure.
- Number literals for `@port` are unsigned integers only (no floats).
- The lexer tracks a capture counter that increments for each `DslPart::Capture` encountered, producing `Capture(index)` tokens.

### 6. Parser Design

The parser consumes `Vec<ServerToken>` and produces `Result<ServerTemplate, Vec<Diagnostic>>`.

**Parsing strategy:** Top-level loop consumes directive tokens one at a time. Each directive keyword dispatches to a specialized parse function.

```
@port       → expect NumberLiteral → set port
@host       → expect StringLiteral → set host
@middleware → expect Capture → push to middlewares
@get/@post/... → expect Path → parse path segments → expect Capture → push Route
```

**Error recovery:** On unexpected token, emit a diagnostic and skip to the next line-start directive.

### 7. Validator Rules

The validator checks the parsed `ServerTemplate` for semantic issues:

| Rule | Severity | Condition |
|------|----------|-----------|
| Duplicate port | Error | `@port` specified more than once |
| Duplicate host | Error | `@host` specified more than once |
| Duplicate route | Error | Same method + same path pattern defined twice |
| Wildcard not last | Error | `*` segment is not the final path segment |
| Empty path | Error | Route directive with no path before capture |
| Port out of range | Error | Port value not in 1..65535 |
| No routes | Warning | Server block has no route directives |

### 8. Independent Testing Strategy

`ag-dsl-server` can be tested completely standalone:

```rust
#[test]
fn test_server_parse() {
    let parts = vec![
        DslPart::Text("@port 3000\n@get /health ".into(), span(0, 24)),
        DslPart::Capture(/* mock expr */, span(24, 30)),
        DslPart::Text("\n".into(), span(30, 31)),
    ];

    let tokens = server_lexer::lex(&parts);
    let ast = server_parser::parse(&tokens).unwrap();

    assert_eq!(ast.port, Some(3000));
    assert_eq!(ast.routes.len(), 1);
    assert_eq!(ast.routes[0].method, HttpMethod::Get);
}
```

No `.ag` file, no host lexer/parser, no codegen pipeline required.

## Risks / Trade-offs

- **Path pattern as single token**: The lexer emits the whole path as one `Path(String)` token rather than individual segment tokens. This simplifies the lexer but means the parser must re-parse the path string. Acceptable trade-off since path parsing is straightforward.
- **No regex or complex path patterns**: Only `:param` and `*` wildcard are supported. Express-style regex patterns (e.g., `/users/:id(\d+)`) are not supported. Can be extended later.
- **No codegen**: This change deliberately stops at the validated AST. A follow-up `server-dsl-codegen` change will handle JS output. This keeps the scope focused and testable.
- **Capture type erasure**: Same as prompt DSL — handler receives `Box<dyn Any>` captures and must downcast. Acceptable since captures are opaque at the DSL level.
- **Middleware ordering**: Middlewares execute in declaration order. No priority or explicit ordering mechanism. If users need reordering, they reorder the `@middleware` lines.
