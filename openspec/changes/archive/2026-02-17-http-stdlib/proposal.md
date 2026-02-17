## Why

AgentScript 需要 HTTP 能力（server + client）来构建 AI agent 应用。Server 用于接收 webhook、暴露 API；Client 用于调用外部 API（LLM provider、第三方服务）。本 change 在 `stdlib-js-interop` 建立的 extern/`@js` 机制之上，提供 `std:http/server` 和 `std:http/client` 两个标准库模块。

Server 侧包装 Hono——一个基于 Web Standards 的轻量框架，原生支持 CF Workers / Deno / Bun / Node.js。Client 侧包装 fetch 的有限子集，提供符合 AG 风格的简化 API。同时提供 `@agentscript/serve` npm 包作为 Node.js server runner，附带 CLI 一键启动。

## What Changes

### 1. `std:http/server` — Hono 封装

基于 Hono 提供 AG 风格的 HTTP server API：

```
import { App, Context } from "std:http/server"

let app = App()

app.get("/", fn(c: Context) -> Response {
  return c.text("Hello!")
})

app.post("/api/data", async fn(c: Context) -> Response {
  let body = await c.req.json()
  return c.json({ received: body })
})

// 中间件
app.use(async fn(c: Context, next: fn() -> Promise<nil>) -> Response {
  let start = Date.now()
  await next()
  c.header("X-Response-Time", str(Date.now() - start))
})

// 路由组合
let api = App()
api.get("/users", fn(c: Context) -> Response { ... })
app.route("/api", api)

// 导出 fetch handler（CF Workers / Deno 直接用）
export default app
```

AG 侧暴露的 API：
- `App()` — 创建应用实例
- `app.get/post/put/delete/patch(path, handler)` — 路由注册
- `app.use(middleware)` / `app.use(path, middleware)` — 中间件
- `app.route(prefix, subApp)` — 子应用挂载
- `Context` — 请求上下文：`c.req`（请求）、`c.text/json/html/redirect`（响应便捷方法）、`c.header`、`c.status`
- `app.fetch` — Web Standards fetch handler，兼容所有 edge runtime

**不暴露的 Hono 特性**（保持简单）：
- RPC mode / `.route()` 类型推导
- Validator middleware（AG 有自己的类型系统）
- JSX/HTML streaming
- WebSocket helper（后续单独模块）

### 2. `std:http/client` — fetch 简化封装

提供比原生 fetch 更简洁的 HTTP client API：

```
import { get, post, put, del } from "std:http/client"

// 简单 GET
let resp = await get("https://api.example.com/data")
let data = await resp.json()

// POST with JSON body
let resp = await post("https://api.example.com/users", {
  body: { name: "Alice", age: 30 },
  headers: { "Authorization": "Bearer token" }
})

// 完整 options
let resp = await get("https://example.com/api", {
  headers: { "Accept": "application/json" },
  timeout: 5000
})
```

**支持的 fetch 子集**：
- HTTP methods: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
- Request headers
- JSON / text / form body
- Response: status, headers, json(), text()
- Timeout（封装为简单的 ms 参数）

**不支持**（简化）：
- AbortSignal / AbortController
- Streaming request/response body
- Cache API
- Redirect mode control（默认 follow）
- Credentials / CORS mode（edge runtime 不需要）

### 3. `@agentscript/serve` — Node.js server runner

npm 包，包装 `@hono/node-server`，提供 Node.js 环境下的 server 启动：

```
// 编译后的 JS 中
import { serve } from "@agentscript/serve"
serve(app, { port: 3000 })
```

附带 CLI 工具用于 DX：
```bash
# 一键启动（编译 + 运行）
npx @agentscript/serve ./app.ag

# 带参数
npx @agentscript/serve ./app.ag --port 8080

# Dev 模式（watch + auto-reload）
npx @agentscript/serve ./app.ag --dev
```

### 4. 类型复用

HTTP 模块复用 `std:web/fetch`（Layer A）中已声明的 Web Standards 类型：
- `Request`, `Response`, `Headers` — 来自 `std:web/fetch` extern 声明
- Hono 的 Context 对象包装了 Request 并提供响应便捷方法

## Capabilities

### New Capabilities

- `http-server`: AG 封装的 HTTP server API——包装 Hono 的应用创建、路由注册、中间件、Context 对象、fetch handler 导出。编译时 `.ag` 声明 + 运行时 `@agentscript/stdlib/http/server` JS 实现
- `http-client`: AG 封装的 HTTP client API——包装 fetch 的简化子集，提供 `get/post/put/del` 便捷函数和 `HttpOptions` 配置。编译时 `.ag` 声明 + 运行时 `@agentscript/stdlib/http/client` JS 实现
- `node-server-runner`: `@agentscript/serve` npm 包——包装 `@hono/node-server` 的 Node.js server 启动器，附带 CLI 工具支持一键启动、dev 模式、port 配置

### Modified Capabilities

（无现有 spec 修改）

## Impact

- **ag-stdlib crate**：新增 `modules/http/server.ag` 和 `modules/http/client.ag` 声明文件；在 `resolve_std_module` 中注册 `std:http/server` 和 `std:http/client` 路径
- **@agentscript/stdlib npm 包**：新增 `http/server/index.js`（Hono 封装）和 `http/client/index.js`（fetch 封装）；新增 `hono` 和 `@hono/node-server` 作为依赖
- **新增 npm 包**：`@agentscript/serve`（Node.js server runner + CLI）
- **依赖关系**：本 change 依赖 `stdlib-js-interop` 的 extern 声明机制和 `std:web/fetch` 的 Web Standards 类型声明
