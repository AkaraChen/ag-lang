## ADDED Requirements

### Requirement: Server lexer accepts DslPart input

The server lexer SHALL accept `Vec<DslPart>` as input (not raw source code). It SHALL scan `DslPart::Text` segments for directive syntax and pass through `DslPart::Capture` as `Capture(index)` tokens referencing the original capture by position.

#### Scenario: Text with directive and capture

- **WHEN** input is `[Text("@port 3000\n@get /health "), Capture(expr, span)]`
- **THEN** lexer produces `DirectivePort`, `NumberLiteral(3000)`, `DirectiveGet`, `Path("/health")`, `Capture(0)`

### Requirement: Directive recognition

The lexer SHALL recognize the following directives when `@` appears at the beginning of a line followed by a known keyword: `@port`, `@host`, `@middleware`, `@get`, `@post`, `@put`, `@delete`, `@patch`. An `@` not at line start, or followed by an unknown keyword, SHALL be treated as an error.

#### Scenario: Known directive at line start

- **WHEN** a text segment contains `@get /users #{handler}\n`
- **THEN** lexer produces `DirectiveGet`, `Path("/users")`, `Capture(0)`

#### Scenario: Unknown @ is an error

- **WHEN** a text segment contains `@unknown /path\n`
- **THEN** lexer produces an error diagnostic "unknown directive @unknown"

#### Scenario: @ mid-line is an error

- **WHEN** a text segment contains `some text @get /path\n`
- **THEN** lexer produces an error diagnostic for unexpected `@` not at line start

### Requirement: @port directive parsing

The lexer SHALL recognize `@port` followed by an unsigned integer literal. The integer SHALL be captured as a `NumberLiteral(value)` token.

#### Scenario: Valid port number

- **WHEN** input text contains `@port 3000\n`
- **THEN** lexer produces `DirectivePort`, `NumberLiteral(3000)`

#### Scenario: Port with large number

- **WHEN** input text contains `@port 8080\n`
- **THEN** lexer produces `DirectivePort`, `NumberLiteral(8080)`

### Requirement: @host directive parsing

The lexer SHALL recognize `@host` followed by a double-quoted string literal. The string content SHALL be captured as a `StringLiteral(value)` token.

#### Scenario: Host with IP address

- **WHEN** input text contains `@host "0.0.0.0"\n`
- **THEN** lexer produces `DirectiveHost`, `StringLiteral("0.0.0.0")`

#### Scenario: Host with localhost

- **WHEN** input text contains `@host "127.0.0.1"\n`
- **THEN** lexer produces `DirectiveHost`, `StringLiteral("127.0.0.1")`

### Requirement: @middleware directive parsing

The lexer SHALL recognize `@middleware` followed by a capture `#{expr}`. The capture SHALL be passed through as a `Capture(index)` token.

#### Scenario: Single middleware

- **WHEN** input parts are `[Text("@middleware "), Capture(cors_expr, span)]`
- **THEN** lexer produces `DirectiveMiddleware`, `Capture(0)`

#### Scenario: Multiple middlewares

- **WHEN** input parts are `[Text("@middleware "), Capture(cors, span), Text("\n@middleware "), Capture(auth, span)]`
- **THEN** lexer produces `DirectiveMiddleware`, `Capture(0)`, `DirectiveMiddleware`, `Capture(1)`

### Requirement: Route directive parsing

The lexer SHALL recognize `@get`, `@post`, `@put`, `@delete`, `@patch` each followed by a path pattern and a capture. The path pattern SHALL be captured as a `Path(string)` token containing the raw path text. The handler capture SHALL be a `Capture(index)` token.

#### Scenario: GET with simple path

- **WHEN** input parts are `[Text("@get /health "), Capture(handler, span)]`
- **THEN** lexer produces `DirectiveGet`, `Path("/health")`, `Capture(0)`

#### Scenario: POST with parameterized path

- **WHEN** input parts are `[Text("@post /users/:id "), Capture(handler, span)]`
- **THEN** lexer produces `DirectivePost`, `Path("/users/:id")`, `Capture(0)`

#### Scenario: DELETE with nested path

- **WHEN** input parts are `[Text("@delete /api/v1/users/:id "), Capture(handler, span)]`
- **THEN** lexer produces `DirectiveDelete`, `Path("/api/v1/users/:id")`, `Capture(0)`

#### Scenario: PUT with wildcard

- **WHEN** input parts are `[Text("@put /files/* "), Capture(handler, span)]`
- **THEN** lexer produces `DirectivePut`, `Path("/files/*")`, `Capture(0)`

#### Scenario: PATCH route

- **WHEN** input parts are `[Text("@patch /users/:id "), Capture(handler, span)]`
- **THEN** lexer produces `DirectivePatch`, `Path("/users/:id")`, `Capture(0)`

### Requirement: Path pattern segment parsing

The parser SHALL split a `Path(string)` token into a `Vec<PathSegment>` where each segment is one of: `Literal(String)` for static segments, `Param(String)` for `:name` segments, or `Wildcard` for `*` segments.

#### Scenario: Root path

- **WHEN** path is `/`
- **THEN** parser produces empty segment list `[]`

#### Scenario: Single literal segment

- **WHEN** path is `/health`
- **THEN** parser produces `[Literal("health")]`

#### Scenario: Multiple literal segments

- **WHEN** path is `/api/v1/users`
- **THEN** parser produces `[Literal("api"), Literal("v1"), Literal("users")]`

#### Scenario: Named parameter

- **WHEN** path is `/users/:id`
- **THEN** parser produces `[Literal("users"), Param("id")]`

#### Scenario: Multiple parameters

- **WHEN** path is `/users/:userId/posts/:postId`
- **THEN** parser produces `[Literal("users"), Param("userId"), Literal("posts"), Param("postId")]`

#### Scenario: Wildcard segment

- **WHEN** path is `/files/*`
- **THEN** parser produces `[Literal("files"), Wildcard]`

#### Scenario: Mixed segments

- **WHEN** path is `/api/v1/:resource/*`
- **THEN** parser produces `[Literal("api"), Literal("v1"), Param("resource"), Wildcard]`

#### Scenario: Path missing leading slash

- **WHEN** path is `health`
- **THEN** parser produces error "path must start with /"

#### Scenario: Empty segment (double slash)

- **WHEN** path is `/users//posts`
- **THEN** parser produces error "empty path segment"

### Requirement: Parser produces ServerTemplate AST

The parser SHALL accept `Vec<ServerToken>` and produce a `ServerTemplate` AST containing: optional port (`u16`), optional host (`String`), ordered list of middleware capture indices, and ordered list of `Route` entries each with method, parsed path segments, and handler capture index.

#### Scenario: Full server template

- **WHEN** tokens represent a server with `@port 3000`, `@host "0.0.0.0"`, `@middleware #{cors}`, `@get /health #{handler}`, `@post /users #{create}`
- **THEN** parser produces `ServerTemplate` with `port: Some(3000)`, `host: Some("0.0.0.0")`, `middlewares: [0]`, `routes: [Route { Get, [Literal("health")], 1 }, Route { Post, [Literal("users")], 2 }]`

#### Scenario: Minimal server with only routes

- **WHEN** tokens represent a server with only `@get / #{handler}`
- **THEN** parser produces `ServerTemplate` with `port: None`, `host: None`, `middlewares: []`, `routes: [Route { Get, [], 0 }]`

#### Scenario: Server with multiple middlewares

- **WHEN** tokens represent `@middleware #{cors}`, `@middleware #{auth}`, `@middleware #{logging}`
- **THEN** parser produces `middlewares: [0, 1, 2]` preserving declaration order

### Requirement: Parser error on missing handler capture

The parser SHALL produce a diagnostic error when a route directive has a path but is missing the handler capture.

#### Scenario: Route without handler

- **WHEN** tokens are `DirectiveGet`, `Path("/health")`, followed by another directive or Eof
- **THEN** parser produces error "expected handler capture after path in @get"

### Requirement: Parser error on missing path

The parser SHALL produce a diagnostic error when a route directive is missing the path pattern.

#### Scenario: Route without path

- **WHEN** tokens are `DirectiveGet`, `Capture(0)` (no Path token)
- **THEN** parser produces error "expected path pattern after @get"

### Requirement: Parser error on @port missing number

The parser SHALL produce a diagnostic error when `@port` is not followed by a number literal.

#### Scenario: Port without value

- **WHEN** tokens are `DirectivePort` followed by another directive or Eof
- **THEN** parser produces error "expected port number after @port"

#### Scenario: Port with string instead of number

- **WHEN** tokens are `DirectivePort`, `StringLiteral("3000")`
- **THEN** parser produces error "expected port number after @port"

### Requirement: Parser error on @host missing string

The parser SHALL produce a diagnostic error when `@host` is not followed by a string literal.

#### Scenario: Host without value

- **WHEN** tokens are `DirectiveHost` followed by another directive or Eof
- **THEN** parser produces error "expected host string after @host"

### Requirement: Parser error on @middleware missing capture

The parser SHALL produce a diagnostic error when `@middleware` is not followed by a capture.

#### Scenario: Middleware without capture

- **WHEN** tokens are `DirectiveMiddleware` followed by another directive or Eof
- **THEN** parser produces error "expected capture expression after @middleware"

### Requirement: Validator checks duplicate port

The validator SHALL produce an error if `@port` is specified more than once.

#### Scenario: Duplicate port directive

- **WHEN** the parsed server template was produced from input with two `@port` lines
- **THEN** validator produces error "duplicate @port directive"

### Requirement: Validator checks duplicate host

The validator SHALL produce an error if `@host` is specified more than once.

#### Scenario: Duplicate host directive

- **WHEN** the parsed server template was produced from input with two `@host` lines
- **THEN** validator produces error "duplicate @host directive"

### Requirement: Validator checks duplicate routes

The validator SHALL produce an error if two routes have the same HTTP method and the same path pattern.

#### Scenario: Duplicate GET routes

- **WHEN** the parsed server template has two routes with method `Get` and path `[Literal("users")]`
- **THEN** validator produces error "duplicate route: GET /users"

#### Scenario: Same path different methods is allowed

- **WHEN** the parsed server template has `Get /users` and `Post /users`
- **THEN** validator produces no error

### Requirement: Validator checks wildcard position

The validator SHALL produce an error if a `Wildcard` segment is not the last segment in a route path.

#### Scenario: Wildcard not last

- **WHEN** a route has path `[Literal("files"), Wildcard, Literal("info")]`
- **THEN** validator produces error "wildcard must be the last path segment"

### Requirement: Validator checks port range

The validator SHALL produce an error if the port value is 0 or greater than 65535.

#### Scenario: Port zero

- **WHEN** the parsed server template has `port: Some(0)`
- **THEN** validator produces error "port must be between 1 and 65535"

#### Scenario: Port too large

- **WHEN** the parsed server template has `port: Some(70000)`
- **THEN** validator produces error "port must be between 1 and 65535"

### Requirement: Validator warns on no routes

The validator SHALL produce a warning if the server template contains no route directives.

#### Scenario: Server with no routes

- **WHEN** the parsed server template has `routes: []`
- **THEN** validator produces warning "server has no routes defined"
