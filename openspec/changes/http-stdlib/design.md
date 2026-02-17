## Context

AgentScript 需要 HTTP server + client 能力。本 change 建立在 `stdlib-js-interop` 之上：

- `std:web/fetch`（Layer A）已声明 Web Standards 类型：`Request`, `Response`, `Headers`, `URL`
- Layer B 模块机制已就绪：`.ag` 声明 + `@agentscript/stdlib` npm 包中的 JS 实现
- extern / `@js` 注解 codegen 管线已可用

Server 侧选择 Hono（而非从头实现），因为：
- 基于 Web Standards（fetch handler 模式），天然兼容 CF Workers / Deno / Bun
- 极轻量（<14KB），无多余抽象
- 路由 + 中间件模型成熟，AG 只需薄封装

Client 侧直接包装全局 `fetch`，不引入额外 HTTP client 库。

## Goals / Non-Goals

**Goals:**
- `std:http/server`：AG 风格的 HTTP server API，包装 Hono 核心功能（路由、中间件、Context）
- `std:http/client`：简化的 HTTP client，便捷函数 `get/post/put/del` + `HttpOptions`
- `@agentscript/serve`：Node.js server runner + CLI
- 所有模块可被 AG 编译器类型检查
- Server 导出标准 fetch handler，可直接部署到 edge runtime

**Non-Goals:**
- 不封装 Hono 的 RPC mode、validator middleware、JSX rendering
- 不支持 AbortSignal / AbortController（client 简化）
- 不支持 streaming request/response body（后续 change）
- 不支持 WebSocket（后续单独模块）
- 不实现自定义 HTTP server（直接用 Hono）
- 不做 Hono 中间件生态的类型声明（用户需要时自己 extern）

## Decisions

### 1. Server API 设计：薄封装 vs 深度抽象

**选择：薄封装** — AG 的 `App` 就是 Hono 实例的类型声明，不额外包一层。

```
// std:http/server 的 .ag 声明
@js("hono")
extern struct App {
  fn get(path: str, handler: fn(Context) -> Response | Promise<Response>) -> App
  fn post(path: str, handler: fn(Context) -> Response | Promise<Response>) -> App
  fn put(path: str, handler: fn(Context) -> Response | Promise<Response>) -> App
  fn delete(path: str, handler: fn(Context) -> Response | Promise<Response>) -> App
  fn patch(path: str, handler: fn(Context) -> Response | Promise<Response>) -> App
  fn use(middleware: fn(Context, fn() -> Promise<nil>) -> Response | Promise<Response>) -> App
  fn use(path: str, middleware: fn(Context, fn() -> Promise<nil>) -> Response | Promise<Response>) -> App
  fn route(prefix: str, app: App) -> App
  fn fetch(request: Request) -> Promise<Response>
}
```

为什么不深度抽象：
- Hono 本身已经是 Web Standards 的薄封装，再包一层没有价值
- AG 用户迁移到/来自 Hono 零学习成本
- 减少维护面——Hono 升级时只需更新类型声明

**例外**：`App()` 构造函数在 JS runtime 中薄封装为工厂函数，因为 AG 暂不支持 `new` 表达式的泛型。

```javascript
// @agentscript/stdlib/http/server/index.js
import { Hono } from "hono";
export function App() { return new Hono(); }
```

### 2. Context 对象设计

Context 是 Hono 的核心 API，AG 侧通过 extern struct 声明其类型：

```
@js("hono")
extern struct Context {
  req: HonoRequest
  fn text(text: str, status: int?) -> Response
  fn json(data: any, status: int?) -> Response
  fn html(html: str, status: int?) -> Response
  fn redirect(url: str, status: int?) -> Response
  fn header(name: str, value: str) -> nil
  fn status(code: int) -> nil
}

@js("hono")
extern struct HonoRequest {
  fn param(key: str) -> str
  fn query(key: str) -> str?
  fn header(name: str) -> str?
  fn json() -> Promise<any>
  fn text() -> Promise<str>
}
```

**设计决策**：
- 不暴露 `c.env`（环境变量通过 `std:env` 访问）
- 不暴露 `c.set/get/var`（类型安全的 context variables 需要泛型，暂不支持）
- `c.req` 声明为 `HonoRequest` 而非 Web Standards `Request`（Hono 的扩展 API 更实用）

### 3. Client API 设计：便捷函数 + Options struct

**选择：函数式 API** — 不包装成 class，直接提供 `get/post/put/del` 函数。

```
// std:http/client 的 .ag 声明

struct HttpOptions {
  headers: {str: str}?
  body: any?
  timeout: int?
}

fn get(url: str, options: HttpOptions?) -> Promise<Response>
fn post(url: str, options: HttpOptions?) -> Promise<Response>
fn put(url: str, options: HttpOptions?) -> Promise<Response>
fn del(url: str, options: HttpOptions?) -> Promise<Response>
fn patch(url: str, options: HttpOptions?) -> Promise<Response>
fn head(url: str, options: HttpOptions?) -> Promise<Response>
fn options(url: str, options: HttpOptions?) -> Promise<Response>
```

为什么函数式而非 class：
- AG 的 agent 用例中 HTTP client 调用通常是一次性的（调 API、查数据）
- 函数式更简洁：`await get("url")` vs `await client.get("url")`
- 不需要维护 client 实例状态（base URL、默认 headers 等可以后续加）

**JS runtime 实现**：

```javascript
// @agentscript/stdlib/http/client/index.js
export async function get(url, options) {
  return fetch(url, { method: "GET", ...buildInit(options) });
}
export async function post(url, options) {
  const init = buildInit(options);
  if (options?.body && typeof options.body === "object") {
    init.body = JSON.stringify(options.body);
    init.headers = { "Content-Type": "application/json", ...init.headers };
  }
  return fetch(url, { method: "POST", ...init });
}
// ...put, del, patch, head, options 同理

function buildInit(options) {
  const init = {};
  if (options?.headers) init.headers = options.headers;
  if (options?.timeout) {
    init.signal = AbortSignal.timeout(options.timeout);
  }
  return init;
}
```

**注意**：timeout 在 JS runtime 中内部使用 `AbortSignal.timeout()`，但 AG 侧不暴露 AbortSignal 概念。对 AG 用户来说就是一个 `timeout: int` 参数（毫秒）。

### 4. Client body 自动序列化

`HttpOptions.body` 类型为 `any`，JS runtime 按以下规则处理：
- 如果是 object/array → `JSON.stringify` + 设置 `Content-Type: application/json`
- 如果是 string → 直接作为 body，`Content-Type: text/plain`
- 如果是 `nil` → 无 body

这样 AG 用户写 `post("url", { body: { key: "value" } })` 就自动是 JSON 请求。

### 5. `@agentscript/serve` 包设计

独立 npm 包（不在 `@agentscript/stdlib` 中），因为：
- 它是 Node.js 特有的（edge runtime 不需要）
- 包含 CLI 工具，体积和依赖不同
- 用户可以选择不用它（直接部署到 CF Workers）

**包结构**：

```
packages/agentscript-serve/
├── package.json          # bin: { "agentscript-serve": "./cli.js" }
├── src/
│   ├── index.ts          # serve(app, options) 函数
│   └── cli.ts            # CLI 入口
├── tsconfig.json
└── README.md
```

**`serve()` API**：

```javascript
import { serve } from "@hono/node-server";

export function startServer(app, options = {}) {
  const port = options.port || 3000;
  serve({ fetch: app.fetch, port }, (info) => {
    console.log(`AgentScript server running at http://localhost:${info.port}`);
  });
}
```

**CLI 工具**：

```bash
npx @agentscript/serve <entry.ag> [options]
```

CLI 流程：
1. 调用 `asc build <entry.ag>` 编译为 JS
2. 动态 import 编译后的 JS 模块
3. 找到 default export（应该是 App 实例）
4. 调用 `startServer(app, { port })`

`--dev` 模式：watch `.ag` 文件变化 → 重新编译 → 重启 server。

### 6. 模块文件结构

**ag-stdlib 中的 .ag 声明文件**：

```
crates/ag-stdlib/modules/
├── http/
│   ├── server.ag     # App, Context, HonoRequest 声明
│   └── client.ag     # get, post, put, del, HttpOptions 声明
```

**@agentscript/stdlib npm 包中的 JS 实现**：

```
@agentscript/stdlib/
├── http/
│   ├── server/index.js    # App() 工厂 + re-export Hono types
│   └── client/index.js    # get/post/put/del 实现
```

**resolve_std_module 新增映射**：

```rust
"std:http/server" => Some(include_str!("../modules/http/server.ag")),
"std:http/client" => Some(include_str!("../modules/http/client.ag")),
```

### 7. 类型复用策略

`std:http/server` 和 `std:http/client` 都返回 `Response` 类型。这个类型已在 `std:web/fetch` 中声明为 extern struct。

**策略**：http 模块的 .ag 文件中直接引用 `Response` 名称（无需 re-declare），因为：
- 同一个编译单元中 `std:web/fetch` 的 `Response` 和 Hono 返回的 `Response` 是同一个 JS 类
- 编译器处理 stdlib 模块时，将 `std:web/fetch` 的类型视为全局可用（Web Standards 预注入）

如果用户没有显式 import `std:web/fetch` 但 import 了 `std:http/server`，编译器应自动将 `Response` 等 Web Standards 类型加入作用域（因为 http/server.ag 内部依赖它们）。

## Risks / Trade-offs

- **Hono 版本锁定**：`@agentscript/stdlib` 锁定某个 Hono 主版本 → 用 semver range `^4.x` 减轻，Hono 的 API 稳定性良好。
- **Context 类型不完整**：AG 的 extern struct 无法表达 Hono Context 的全部泛型 API（如 `c.var` 的类型安全） → 可接受，用户需要高级 Hono 特性时可用 `any` 或自行 extern 声明。
- **Client body 自动序列化的隐式行为**：object 自动 JSON.stringify 可能让用户困惑 → 文档说明清楚，且这是最常见用例。
- **CLI 依赖编译器**：`@agentscript/serve` CLI 需要 `asc` 编译器已安装 → 作为 peer dependency 声明，或在 CLI 中检查并提示安装。
- **Web Standards 类型的隐式依赖**：http 模块隐式依赖 `std:web/fetch` 的类型 → 编译器需处理 stdlib 模块间的内部依赖解析。
