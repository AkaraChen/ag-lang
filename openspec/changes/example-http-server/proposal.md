## Why

需要一个完整的 example 来验证和展示 AgentScript 的核心语言特性 + HTTP 标准库的协同工作。这个 example 作为端到端 smoke test，确保 `basic-language-core`、`stdlib-js-interop`、`http-stdlib` 三个 change 组合后能正常编译和运行。同时作为语言的 "Hello World" 级别入门示例。

## What Changes

### 1. Example Server 应用 (`examples/http-server/`)

一个完整的 `.ag` HTTP server，演示以下语言特性：

**基础语言特性**：
- `let` 变量绑定
- `fn` 函数定义（含参数类型和返回类型）
- 算术运算符（`+`, `-`, `*`, `/`）
- 比较运算符（`==`）
- `if` / `else if` / `else` 控制流
- 字符串、数字、布尔、数组、对象字面量
- 字符串拼接

**HTTP + 标准库特性**：
- `import { ... } from "std:http/server"` 标准库导入
- `App()` 创建 server
- 路由注册（`app.get`, `app.post`）
- 同步和异步 handler
- `Context` 使用（`c.req.json()`, `c.json()`, `c.text()`）
- `await` 异步操作
- `export default app` 导出

**端点设计**：
- `GET /` — 健康检查，返回 server 信息（演示对象字面量、数组）
- `POST /echo` — 原样返回 body（演示 async/await、JSON 解析）
- `POST /calc` — 算术计算（演示函数调用、if/else、运算符）
- `GET /greet/:name` — 带路径参数的问候（演示路径参数、字符串拼接）

### 2. 配套文件

- `examples/http-server/app.ag` — 主应用代码
- `examples/http-server/README.md` — 运行说明

## Capabilities

### New Capabilities

- `example-app`: 完整的示例应用规格——定义 example server 的端点行为、演示的语言特性清单、预期的编译输出和运行时行为

### Modified Capabilities

（无）

## Impact

- **新增目录**：`examples/http-server/`
- **依赖**：依赖 `basic-language-core`（编译器）、`stdlib-js-interop`（extern 机制）、`http-stdlib`（HTTP 模块）全部实现完成
- **无编译器变更**：纯用户代码示例，不修改任何编译器 crate
