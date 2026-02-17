## ADDED Requirements

### Requirement: serve() function starts Node.js HTTP server

The `@agentscript/serve` npm package SHALL export a `serve(app, options?)` function that starts an HTTP server on Node.js using `@hono/node-server`. The `app` parameter SHALL be a Hono-compatible application (any object with a `.fetch()` method). The `options` parameter SHALL accept `{ port: int }`.

#### Scenario: Default port

- **WHEN** JS code calls `serve(app)`
- **THEN** the server SHALL start on port `3000` and log a startup message

#### Scenario: Custom port

- **WHEN** JS code calls `serve(app, { port: 8080 })`
- **THEN** the server SHALL start on port `8080`

#### Scenario: Startup message

- **WHEN** the server starts successfully
- **THEN** a message SHALL be printed to stdout: `AgentScript server running at http://localhost:<port>`

### Requirement: CLI compiles and runs .ag files

The `@agentscript/serve` package SHALL provide a CLI command accessible via `npx @agentscript/serve <entry.ag>`. The CLI SHALL compile the `.ag` file to JavaScript using the `asc` compiler, then start the server with the compiled output.

#### Scenario: Basic CLI usage

- **WHEN** the user runs `npx @agentscript/serve app.ag`
- **THEN** the CLI SHALL:
  1. Run `asc build app.ag` to compile to JS
  2. Dynamically import the compiled JS module
  3. Find the default export (App instance)
  4. Call `serve(app)` on port 3000

#### Scenario: CLI with port flag

- **WHEN** the user runs `npx @agentscript/serve app.ag --port 8080`
- **THEN** the CLI SHALL compile and start the server on port `8080`

#### Scenario: Missing .ag file

- **WHEN** the user runs `npx @agentscript/serve nonexistent.ag`
- **THEN** the CLI SHALL print an error message and exit with code 1

#### Scenario: Compilation error

- **WHEN** the .ag file has syntax errors
- **THEN** the CLI SHALL print the compilation errors from `asc` and exit with code 1

#### Scenario: No default export

- **WHEN** the compiled JS module has no default export or the default export is not an App
- **THEN** the CLI SHALL print an error: "No App instance found as default export" and exit with code 1

### Requirement: Dev mode with file watching

The CLI SHALL support a `--dev` flag that enables development mode: watching `.ag` source files for changes, re-compiling on change, and restarting the server automatically.

#### Scenario: Dev mode startup

- **WHEN** the user runs `npx @agentscript/serve app.ag --dev`
- **THEN** the CLI SHALL compile, start the server, and begin watching for `.ag` file changes

#### Scenario: File change triggers restart

- **WHEN** in dev mode, a watched `.ag` file is modified
- **THEN** the CLI SHALL re-compile, stop the current server, and start a new server with the updated code

#### Scenario: Compilation error in dev mode

- **WHEN** in dev mode, a file change introduces a syntax error
- **THEN** the CLI SHALL print the error but keep the previous server running (not crash)

### Requirement: Package structure and configuration

The `@agentscript/serve` SHALL be a standalone npm package at `packages/agentscript-serve/`. It SHALL declare `@hono/node-server` as a dependency and `asc` (the AgentScript compiler) as a peer dependency. The `package.json` SHALL configure a `bin` entry for the CLI.

#### Scenario: Package.json bin entry

- **WHEN** inspecting the package.json
- **THEN** it SHALL have `"bin": { "agentscript-serve": "./cli.js" }` (or equivalent)

#### Scenario: Dependencies

- **WHEN** inspecting the package.json dependencies
- **THEN** `@hono/node-server` SHALL be a regular dependency, and the AgentScript compiler SHALL be a peer dependency

### Requirement: Graceful shutdown

The `serve()` function SHALL handle `SIGINT` and `SIGTERM` signals to gracefully shut down the HTTP server.

#### Scenario: Ctrl+C stops server

- **WHEN** the user presses Ctrl+C while the server is running
- **THEN** the server SHALL shut down gracefully and the process SHALL exit with code 0

#### Scenario: SIGTERM stops server

- **WHEN** the process receives a SIGTERM signal
- **THEN** the server SHALL shut down gracefully
