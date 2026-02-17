## Why

AgentScript 编译到 JavaScript，但目前没有调用 JS API 的机制。标准库（`std:http`, `std:fs`, `std:log` 等）需要映射到 JS 运行时。在实现任何具体标准库模块（如 HTTP）之前，必须先解决根本问题：**AG 代码如何声明和调用 JS 侧的函数和类型**。这个胶水层（JS interop）是所有标准库模块的基础。

## What Changes

### 1. `extern` 声明机制

新增 `extern` 关键字，用于声明 JS 侧已存在的函数、类型和值：

```
// 声明 JS 全局函数
extern fn fetch(input: str | Request, init: RequestInit?) -> Promise<Response>
extern fn setTimeout(callback: () -> nil, ms: int) -> int
extern fn console_log(msg: any) -> nil

// 声明 JS 类/接口的类型
extern struct Request {
  method: str
  url: str
  headers: Headers
  fn json() -> Promise<any>
  fn text() -> Promise<str>
}

extern struct Response {
  status: int
  ok: bool
  headers: Headers
  fn json() -> Promise<any>
  fn text() -> Promise<str>
}

// 声明 JS 模块导出
@js("node:fs/promises")
extern fn readFile(path: str, encoding: str) -> Promise<str>

@js("node:path")
extern fn join(parts: ...str) -> str
```

`extern` 声明在**编译时提供类型信息**，在**运行时不产出代码**（类型擦除）。codegen 直接引用声明的名称。

### 2. `@js("module")` 注解

指定 extern 声明的 JS 来源：
- 无 `@js` → 全局可用（如 `fetch`, `console`, `setTimeout`）
- `@js("module_path")` → 编译为 `import { name } from "module_path"`
- `@js("module_path", name="jsName")` → 编译为 `import { jsName as name } from "module_path"`

### 3. 类型映射体系

AG 和 JS 之间的双向类型映射：

| AgentScript | JavaScript | 方向 |
|---|---|---|
| `str` | `string` | 双向直通 |
| `num` | `number` | 双向直通 |
| `int` | `number` | AG→JS 直通，JS→AG 需 runtime 检查 |
| `bool` | `boolean` | 双向直通 |
| `nil` | `null` | 双向直通 |
| `[T]` | `Array<T>` | 双向直通 |
| `{K: V}` | `Record<K, V>` / `Map<K, V>` | 双向直通 |
| `T?` | `T \| null` | 双向直通 |
| `struct Foo {}` | plain object `{}` | AG struct → JS object, JS object → AG struct (structural) |
| `enum Status {}` | tagged union `{ tag: "..." }` | AG enum ↔ JS tagged union |
| `Promise<T>` | `Promise<T>` | 新增内置类型，async 互操作桥梁 |
| `fn(A) -> B` | `(a: A) => B` | 双向直通（函数引用） |

### 4. 标准库模块架构

标准库分两层：

**Layer A — Web 标准 extern 声明**（编译时类型，运行时零开销）：
- `std:web/fetch` — `fetch`, `Request`, `Response`, `Headers`, `URL`, `URLSearchParams`
- `std:web/crypto` — `crypto.subtle`, `crypto.randomUUID()`
- `std:web/encoding` — `TextEncoder`, `TextDecoder`, `atob`, `btoa`
- `std:web/streams` — `ReadableStream`, `WritableStream`, `TransformStream`
- `std:web/timers` — `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`

**Layer B — AG 封装模块**（编译为 `@agentscript/stdlib` npm import）：
- `std:log` — 结构化 logging，AG 风格 API
- `std:encoding` — `json.parse/stringify`, `yaml`, `toml` 便捷封装
- `std:env` — 环境变量访问，类型安全的 `env.get<str>("KEY")`
- `std:fs` — 文件系统（封装 `node:fs/promises`，统一 API）

**实现方式**：
- Layer A 模块是 `.ag` 文件，只包含 `extern` 声明，编译时完全擦除
- Layer B 模块是 `.ag` 文件 + 对应的 JS runtime 实现（在 `@agentscript/stdlib` npm 包中）

### 5. `Promise<T>` 和 async 互操作

新增 `Promise<T>` 作为内置类型：
- `extern fn` 返回 `Promise<T>` 时，AG 侧可用 `await` 消费
- AG 的 `async fn` 编译为 JS `async function`，返回类型自动包装为 `Promise<T>`
- 这是 AG ↔ JS async 世界的桥梁

## Capabilities

### New Capabilities

- `extern-declarations`: `extern fn`/`extern struct`/`extern type` 声明机制——让 AG 代码引用 JS 侧存在的函数和类型，编译时类型检查，运行时类型擦除
- `js-annotation`: `@js("module")` 注解——指定 extern 的 JS 来源模块，codegen 生成对应的 import 语句
- `type-mapping`: AG ↔ JS 类型映射体系——基本类型直通，struct/enum/Promise 的互操作规则，`int` 的运行时检查
- `stdlib-architecture`: 标准库模块架构——Layer A (extern declarations) + Layer B (JS runtime wrappers) 的分层模型，`std:*` 模块路径解析

### Modified Capabilities

（无现有 spec）

## Impact

- **ag-ast**：新增 `ExternFnDecl`, `ExternStructDecl`, `ExternTypeDecl` AST 节点；新增 `Promise<T>` 类型
- **ag-lexer**：新增 `extern` 关键字
- **ag-parser**：新增 `extern fn/struct/type` 解析；新增 `@js(...)` 注解解析
- **ag-checker**：extern 声明注册到符号表；`Promise<T>` 类型推断；async/await 与 Promise 交互检查
- **ag-codegen**：extern fn → 不产出函数体，仅在调用时引用名称；`@js("mod")` → 生成 import 语句；`Promise<T>` → 直通
- **新增 crate**：`ag-stdlib`（Rust crate，包含所有 `std:*` 模块的 `.ag` 声明文件）
- **新增 npm 包**：`@agentscript/stdlib`（Layer B 模块的 JS runtime 实现）
- **Cargo workspace**：新增 `crates/ag-stdlib/`
