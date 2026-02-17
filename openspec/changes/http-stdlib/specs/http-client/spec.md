## ADDED Requirements

### Requirement: HTTP method convenience functions

The `std:http/client` module SHALL export convenience functions for common HTTP methods: `get`, `post`, `put`, `del`, `patch`, `head`, `options`. Each function SHALL accept a URL string and optional `HttpOptions`, and return `Promise<Response>`.

#### Scenario: Simple GET request

- **WHEN** AG code has `let resp = await get("https://api.example.com/data")`
- **THEN** the compiled JS SHALL call `fetch("https://api.example.com/data", { method: "GET" })` and return the Response

#### Scenario: POST with JSON body

- **WHEN** AG code has `let resp = await post("https://api.example.com/users", { body: { name: "Alice" } })`
- **THEN** the compiled JS SHALL call `fetch(url, { method: "POST", body: JSON.stringify({name: "Alice"}), headers: {"Content-Type": "application/json"} })`

#### Scenario: PUT request

- **WHEN** AG code has `let resp = await put("https://api.example.com/users/1", { body: { name: "Bob" } })`
- **THEN** the compiled JS SHALL call `fetch(url, { method: "PUT", ... })` with JSON body

#### Scenario: DELETE request

- **WHEN** AG code has `let resp = await del("https://api.example.com/users/1")`
- **THEN** the compiled JS SHALL call `fetch(url, { method: "DELETE" })`

#### Scenario: HEAD request

- **WHEN** AG code has `let resp = await head("https://example.com")`
- **THEN** the compiled JS SHALL call `fetch(url, { method: "HEAD" })`

### Requirement: HttpOptions struct

The `std:http/client` module SHALL export an `HttpOptions` struct with optional fields: `headers: {str: str}?`, `body: any?`, `timeout: int?`.

#### Scenario: Request with custom headers

- **WHEN** AG code has `await get("url", { headers: { "Authorization": "Bearer token" } })`
- **THEN** the compiled JS SHALL include the headers in the fetch call

#### Scenario: Request with timeout

- **WHEN** AG code has `await get("url", { timeout: 5000 })`
- **THEN** the JS runtime SHALL internally use `AbortSignal.timeout(5000)` to enforce the timeout

#### Scenario: No options

- **WHEN** AG code has `await get("url")`
- **THEN** the JS runtime SHALL call fetch with only the method, no extra options

### Requirement: Body auto-serialization

When `HttpOptions.body` is provided, the JS runtime SHALL auto-serialize based on type: objects/arrays SHALL be serialized via `JSON.stringify` with `Content-Type: application/json`; strings SHALL be sent as-is with `Content-Type: text/plain`; `nil` body SHALL result in no request body.

#### Scenario: Object body becomes JSON

- **WHEN** AG code has `post("url", { body: { key: "value" } })`
- **THEN** the JS runtime SHALL set `body: JSON.stringify({key: "value"})` and `Content-Type: application/json`

#### Scenario: Array body becomes JSON

- **WHEN** AG code has `post("url", { body: [1, 2, 3] })`
- **THEN** the JS runtime SHALL set `body: JSON.stringify([1,2,3])` and `Content-Type: application/json`

#### Scenario: String body sent as text

- **WHEN** AG code has `post("url", { body: "raw text" })`
- **THEN** the JS runtime SHALL set `body: "raw text"` and `Content-Type: text/plain`

#### Scenario: Nil body means no body

- **WHEN** AG code has `post("url", { body: nil })` or no body field
- **THEN** the JS runtime SHALL not set a request body

#### Scenario: Explicit headers override auto Content-Type

- **WHEN** AG code has `post("url", { body: { a: 1 }, headers: { "Content-Type": "text/plain" } })`
- **THEN** the JS runtime SHALL use the explicitly provided `Content-Type: text/plain` instead of auto-detecting `application/json`

### Requirement: Response is Web Standards Response

All client functions SHALL return `Promise<Response>` where `Response` is the Web Standards `Response` type (same as declared in `std:web/fetch`). Users SHALL be able to call `resp.json()`, `resp.text()`, access `resp.status`, `resp.ok`, `resp.headers`.

#### Scenario: Read JSON response

- **WHEN** AG code has `let data = await (await get("url")).json()`
- **THEN** the compiled JS SHALL call `.json()` on the fetch Response, returning parsed JSON as `any`

#### Scenario: Check response status

- **WHEN** AG code has `let resp = await get("url"); if resp.ok { ... }`
- **THEN** the checker SHALL allow `resp.ok` as `bool` and `resp.status` as `int`

### Requirement: Timeout produces rejected Promise

When a request exceeds the `timeout` value in milliseconds, the returned Promise SHALL reject. The AG code SHALL be able to handle this via error handling mechanisms.

#### Scenario: Request times out

- **WHEN** AG code has `await get("url", { timeout: 100 })` and the server takes longer than 100ms
- **THEN** the Promise SHALL reject with a timeout error

### Requirement: AG declaration file for std:http/client

The `std:http/client` module SHALL be declared in `crates/ag-stdlib/modules/http/client.ag`. The file SHALL contain the `HttpOptions` struct declaration and function declarations for all HTTP method convenience functions. The module SHALL be registered in `resolve_std_module` at path `std:http/client`.

#### Scenario: Import from std:http/client

- **WHEN** AG code has `import { get, post } from "std:http/client"`
- **THEN** the compiler SHALL resolve this via `ag-stdlib`, and `get` and `post` SHALL be available in scope

#### Scenario: Codegen produces correct imports

- **WHEN** `get` and `post` are called in user code that imports from `std:http/client`
- **THEN** the compiled JS SHALL contain `import { get, post } from "@agentscript/stdlib/http/client";`

### Requirement: JS runtime implementation for std:http/client

The `@agentscript/stdlib` npm package SHALL contain `http/client/index.js` that exports all HTTP method functions. Each function SHALL wrap the global `fetch` with the appropriate HTTP method and options processing.

#### Scenario: get() implementation

- **WHEN** the JS runtime's `get(url, options)` is called
- **THEN** it SHALL call `fetch(url, { method: "GET", headers: options?.headers, signal: options?.timeout ? AbortSignal.timeout(options.timeout) : undefined })`

#### Scenario: post() implementation with body

- **WHEN** the JS runtime's `post(url, { body: { a: 1 } })` is called
- **THEN** it SHALL call `fetch(url, { method: "POST", body: JSON.stringify({a: 1}), headers: {"Content-Type": "application/json"} })`
