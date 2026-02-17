## ADDED Requirements

### Requirement: App factory function creates Hono instance

The `std:http/server` module SHALL export an `App()` factory function that creates and returns a new Hono application instance. The function SHALL NOT require any arguments.

#### Scenario: Create app

- **WHEN** AG code calls `let app = App()`
- **THEN** the compiled JS SHALL execute `App()` which internally calls `new Hono()` and returns the instance

#### Scenario: Multiple apps

- **WHEN** AG code creates two apps: `let app1 = App()` and `let app2 = App()`
- **THEN** each SHALL be an independent Hono instance

### Requirement: Route registration methods

The `App` type SHALL provide route registration methods for standard HTTP methods: `get`, `post`, `put`, `delete`, `patch`. Each method SHALL accept a path string and a handler function, and SHALL return the `App` instance for chaining.

#### Scenario: GET route

- **WHEN** AG code has `app.get("/", fn(c: Context) -> Response { return c.text("Hello") })`
- **THEN** the compiled JS SHALL register a GET handler at path `/` on the Hono instance

#### Scenario: POST route with async handler

- **WHEN** AG code has `app.post("/api", async fn(c: Context) -> Response { let body = await c.req.json(); return c.json(body) })`
- **THEN** the compiled JS SHALL register an async POST handler at path `/api`

#### Scenario: Route with path parameters

- **WHEN** AG code has `app.get("/users/:id", fn(c: Context) -> Response { let id = c.req.param("id"); return c.text(id) })`
- **THEN** the compiled JS SHALL register a GET handler with path parameter `:id`, and `c.req.param("id")` SHALL return the matched value

#### Scenario: Method chaining

- **WHEN** AG code has `app.get("/a", handler1).post("/b", handler2)`
- **THEN** both routes SHALL be registered on the same app instance

### Requirement: Handler function signature

Route handlers SHALL accept a `Context` parameter and return `Response` or `Promise<Response>`. The checker SHALL validate the handler function signature at compile time.

#### Scenario: Sync handler type check

- **WHEN** AG code has `app.get("/", fn(c: Context) -> Response { return c.text("ok") })`
- **THEN** the checker SHALL accept this as a valid handler signature

#### Scenario: Async handler type check

- **WHEN** AG code has `app.post("/", async fn(c: Context) -> Response { return c.json({}) })`
- **THEN** the checker SHALL accept this — async fn returning `Response` means external type is `Promise<Response>`

#### Scenario: Invalid handler signature

- **WHEN** AG code has `app.get("/", fn(x: str) -> str { return x })`
- **THEN** the checker SHALL produce a type error — handler must accept `Context` and return `Response | Promise<Response>`

### Requirement: Context response helper methods

The `Context` type SHALL provide response helper methods: `text(text: str, status: int?) -> Response`, `json(data: any, status: int?) -> Response`, `html(html: str, status: int?) -> Response`, `redirect(url: str, status: int?) -> Response`.

#### Scenario: c.text()

- **WHEN** a handler calls `c.text("Hello")`
- **THEN** the compiled JS SHALL call Hono's `c.text("Hello")`, producing a `text/plain` Response

#### Scenario: c.json()

- **WHEN** a handler calls `c.json({ name: "Alice" })`
- **THEN** the compiled JS SHALL call Hono's `c.json(...)`, producing an `application/json` Response

#### Scenario: c.text() with status

- **WHEN** a handler calls `c.text("Created", 201)`
- **THEN** the compiled JS SHALL call `c.text("Created", 201)`, producing a Response with status 201

#### Scenario: c.html()

- **WHEN** a handler calls `c.html("<h1>Hello</h1>")`
- **THEN** the compiled JS SHALL call Hono's `c.html(...)`, producing a `text/html` Response

#### Scenario: c.redirect()

- **WHEN** a handler calls `c.redirect("/login")`
- **THEN** the compiled JS SHALL call Hono's `c.redirect("/login")`, producing a 302 redirect Response

#### Scenario: c.redirect() with status

- **WHEN** a handler calls `c.redirect("/new-url", 301)`
- **THEN** the compiled JS SHALL produce a 301 permanent redirect

### Requirement: Context header and status methods

The `Context` type SHALL provide `header(name: str, value: str) -> nil` to set response headers and `status(code: int) -> nil` to set the response status code.

#### Scenario: Set response header

- **WHEN** a handler calls `c.header("X-Custom", "value")`
- **THEN** the compiled JS SHALL call Hono's `c.header("X-Custom", "value")`

#### Scenario: Set status code

- **WHEN** a handler calls `c.status(404)`
- **THEN** the compiled JS SHALL call Hono's `c.status(404)`

### Requirement: Context request access

The `Context` type SHALL expose a `req` field of type `HonoRequest`. `HonoRequest` SHALL provide: `param(key: str) -> str`, `query(key: str) -> str?`, `header(name: str) -> str?`, `json() -> Promise<any>`, `text() -> Promise<str>`.

#### Scenario: Access path parameter

- **WHEN** a handler calls `c.req.param("id")` on a route `/users/:id`
- **THEN** the compiled JS SHALL call Hono's `c.req.param("id")` and return the matched string value

#### Scenario: Access query parameter

- **WHEN** a handler calls `c.req.query("page")`
- **THEN** the compiled JS SHALL call Hono's `c.req.query("page")`, returning `str?` (nil if not present)

#### Scenario: Access request header

- **WHEN** a handler calls `c.req.header("Authorization")`
- **THEN** the compiled JS SHALL call Hono's `c.req.header("Authorization")`, returning `str?`

#### Scenario: Parse JSON body

- **WHEN** a handler calls `let body = await c.req.json()`
- **THEN** the compiled JS SHALL call `await c.req.json()`, returning the parsed body as `any`

#### Scenario: Read text body

- **WHEN** a handler calls `let text = await c.req.text()`
- **THEN** the compiled JS SHALL call `await c.req.text()`, returning the body as `str`

### Requirement: Middleware registration

The `App` type SHALL provide a `use` method that accepts middleware functions. Middleware SHALL accept `(Context, fn() -> Promise<nil>)` and return `Response | Promise<Response>`. `use` SHALL support both global middleware (no path) and path-scoped middleware.

#### Scenario: Global middleware

- **WHEN** AG code has `app.use(async fn(c: Context, next: fn() -> Promise<nil>) -> Response { await next(); ... })`
- **THEN** the compiled JS SHALL register global middleware via Hono's `app.use(...)`

#### Scenario: Path-scoped middleware

- **WHEN** AG code has `app.use("/api/*", authMiddleware)`
- **THEN** the compiled JS SHALL register middleware scoped to `/api/*`

#### Scenario: Middleware calls next

- **WHEN** middleware calls `await next()` in its body
- **THEN** the compiled JS SHALL call the next middleware/handler in the chain

### Requirement: Sub-application routing

The `App` type SHALL provide `route(prefix: str, app: App) -> App` to mount a sub-application at a given path prefix.

#### Scenario: Mount sub-app

- **WHEN** AG code has:
  ```
  let api = App()
  api.get("/users", handler)
  app.route("/api", api)
  ```
- **THEN** the compiled JS SHALL call Hono's `app.route("/api", api)`, making `/api/users` accessible

### Requirement: Fetch handler for edge deployment

The `App` type SHALL provide a `fetch(request: Request) -> Promise<Response>` method that conforms to the Web Standards fetch handler signature. This allows the app to be exported as a default export for edge runtimes.

#### Scenario: Export as default for CF Workers

- **WHEN** AG code has `export default app` where `app` is an `App` instance
- **THEN** the compiled JS SHALL export the Hono instance, which has a `.fetch()` method compatible with Cloudflare Workers' expected interface

#### Scenario: Direct fetch handler call

- **WHEN** AG code calls `let resp = await app.fetch(request)`
- **THEN** the compiled JS SHALL call Hono's `app.fetch(request)` and return a `Promise<Response>`

### Requirement: AG declaration file for std:http/server

The `std:http/server` module SHALL be declared in `crates/ag-stdlib/modules/http/server.ag`. The file SHALL contain extern struct declarations for `App`, `Context`, and `HonoRequest`, plus the `App()` factory function. The module SHALL be registered in `resolve_std_module` at path `std:http/server`.

#### Scenario: Import from std:http/server

- **WHEN** AG code has `import { App, Context } from "std:http/server"`
- **THEN** the compiler SHALL resolve this via `ag-stdlib`, and `App` and `Context` SHALL be available in scope

#### Scenario: Codegen produces correct imports

- **WHEN** `App()` is called in user code that imports from `std:http/server`
- **THEN** the compiled JS SHALL contain `import { App } from "@agentscript/stdlib/http/server";`

### Requirement: JS runtime implementation for std:http/server

The `@agentscript/stdlib` npm package SHALL contain `http/server/index.js` that exports the `App` factory function. The module SHALL import from `hono` and re-export the wrapped constructor.

#### Scenario: App factory implementation

- **WHEN** the JS runtime's `App()` is called
- **THEN** it SHALL return `new Hono()` — a Hono instance with all standard methods available
