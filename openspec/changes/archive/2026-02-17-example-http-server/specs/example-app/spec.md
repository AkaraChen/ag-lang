## ADDED Requirements

### Requirement: Example compiles without errors

The `examples/http-server/app.ag` file SHALL compile successfully using `asc build` with no parse errors, type errors, or codegen errors.

#### Scenario: Clean compilation

- **WHEN** the compiler processes `examples/http-server/app.ag`
- **THEN** it SHALL produce a valid JavaScript output file with exit code 0 and no diagnostics

### Requirement: GET / returns server info

The root endpoint SHALL return a JSON response containing the server name, version, and list of available endpoints.

#### Scenario: Health check response

- **WHEN** a GET request is made to `/`
- **THEN** the response SHALL have status 200, Content-Type `application/json`, and body `{ "name": "AgentScript Example Server", "version": "0.1.0", "endpoints": ["/", "/echo", "/calc", "/greet/:name"] }`

### Requirement: POST /echo returns request body

The echo endpoint SHALL return the exact JSON body it receives, demonstrating async/await and JSON body parsing.

#### Scenario: Echo JSON object

- **WHEN** a POST request is made to `/echo` with body `{ "hello": "world" }`
- **THEN** the response SHALL have status 200 and body `{ "hello": "world" }`

#### Scenario: Echo array

- **WHEN** a POST request is made to `/echo` with body `[1, 2, 3]`
- **THEN** the response SHALL have status 200 and body `[1, 2, 3]`

#### Scenario: Echo nested object

- **WHEN** a POST request is made to `/echo` with body `{ "a": { "b": [1, 2] } }`
- **THEN** the response SHALL have status 200 and body `{ "a": { "b": [1, 2] } }`

### Requirement: POST /calc performs arithmetic

The calc endpoint SHALL accept `{ "op": string, "a": number, "b": number }` and return the computed result, demonstrating function calls, if/else branching, and arithmetic operators.

#### Scenario: Addition

- **WHEN** a POST request is made to `/calc` with body `{ "op": "add", "a": 10, "b": 3 }`
- **THEN** the response SHALL contain `{ "op": "add", "a": 10, "b": 3, "result": 13 }`

#### Scenario: Subtraction

- **WHEN** a POST request is made to `/calc` with body `{ "op": "subtract", "a": 10, "b": 3 }`
- **THEN** the response SHALL contain `{ "op": "subtract", "a": 10, "b": 3, "result": 7 }`

#### Scenario: Multiplication

- **WHEN** a POST request is made to `/calc` with body `{ "op": "multiply", "a": 4, "b": 5 }`
- **THEN** the response SHALL contain `{ "op": "multiply", "a": 4, "b": 5, "result": 20 }`

#### Scenario: Division

- **WHEN** a POST request is made to `/calc` with body `{ "op": "divide", "a": 15, "b": 4 }`
- **THEN** the response SHALL contain `{ "op": "divide", "a": 15, "b": 4, "result": 3.75 }`

#### Scenario: Division by zero

- **WHEN** a POST request is made to `/calc` with body `{ "op": "divide", "a": 10, "b": 0 }`
- **THEN** the response SHALL contain `{ "op": "divide", "a": 10, "b": 0, "result": 0 }`

#### Scenario: Unknown operation

- **WHEN** a POST request is made to `/calc` with body `{ "op": "modulo", "a": 10, "b": 3 }`
- **THEN** the response SHALL contain `{ "op": "modulo", "a": 10, "b": 3, "result": 0 }`

### Requirement: GET /greet/:name returns personalized greeting

The greet endpoint SHALL use the path parameter to construct a greeting message, demonstrating path parameters and string concatenation.

#### Scenario: Greet by name

- **WHEN** a GET request is made to `/greet/Alice`
- **THEN** the response SHALL have status 200 and body `{ "message": "Hello, Alice!" }`

#### Scenario: Greet with different name

- **WHEN** a GET request is made to `/greet/World`
- **THEN** the response SHALL have status 200 and body `{ "message": "Hello, World!" }`

### Requirement: Compiled JS output matches expected structure

The compiled JavaScript output SHALL contain: an import from `@agentscript/stdlib/http/server`, function declarations for `add`/`subtract`/`calculate`, route registrations on the app instance, and a default export.

#### Scenario: Import statement

- **WHEN** the compiled JS is inspected
- **THEN** it SHALL contain `import { App } from "@agentscript/stdlib/http/server";`

#### Scenario: Function declarations

- **WHEN** the compiled JS is inspected
- **THEN** it SHALL contain `function add(a, b)`, `function subtract(a, b)`, and `function calculate(op, a, b)` as top-level function declarations

#### Scenario: Default export

- **WHEN** the compiled JS is inspected
- **THEN** it SHALL end with `export default app;` (or equivalent)

### Requirement: Example runs with @agentscript/serve

The example SHALL be runnable via `npx @agentscript/serve examples/http-server/app.ag` and respond to HTTP requests on the default port.

#### Scenario: Start and respond

- **WHEN** `npx @agentscript/serve examples/http-server/app.ag` is executed
- **THEN** the server SHALL start on port 3000 and respond to `GET /` with the server info JSON
