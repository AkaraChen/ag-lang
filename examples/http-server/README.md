# HTTP Server Example

A simple HTTP server demonstrating AgentScript's core language features with the `std:http/server` module.

## Features Demonstrated

- **Imports**: `std:http/server` module resolution
- **Functions**: `add`, `subtract`, `calculate` with type annotations
- **If/else expressions**: Multi-branch conditional logic
- **Async/await**: Async route handlers with JSON body parsing
- **Object & array literals**: JSON response construction
- **String concatenation**: Dynamic greeting messages
- **Path parameters**: `/greet/:name` route
- **Method chaining**: `app.get(...)`, `app.post(...)`

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Server info (name, version, endpoints) |
| POST | `/echo` | Echo back the JSON request body |
| POST | `/calc` | Calculate: `{ op, a, b }` → `{ result }` |
| GET | `/greet/:name` | Greeting: `{ message: "Hello, <name>!" }` |

## Compile

```bash
asc build examples/http-server/app.ag -o app.js
```

## Run

```bash
# After compiling, run with Node.js + @agentscript/serve
npx @agentscript/serve examples/http-server/app.ag
```

## Example Requests

```bash
# Server info
curl http://localhost:3000/

# Echo
curl -X POST http://localhost:3000/echo -H "Content-Type: application/json" -d '{"hello":"world"}'

# Calculator
curl -X POST http://localhost:3000/calc -H "Content-Type: application/json" -d '{"op":"add","a":10,"b":3}'

# Greeting
curl http://localhost:3000/greet/Alice
```
