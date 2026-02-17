## Context

AgentScript 编译到 JS，需要调用 JS 生态的 API。当前编译器（`basic-language-core`）只处理纯 AG 代码，没有跨语言边界的能力。本 change 建立 AG ↔ JS 的桥梁，并在此基础上构建标准库模块体系。

目标运行时对标 Cloudflare Workers / Deno Deploy 的模型：拥抱 Web 标准 API（fetch, Request, Response, streams 等），在此之上提供薄封装。

## Goals / Non-Goals

**Goals:**
- `extern` 声明机制：AG 代码声明 JS 侧的函数/类型，编译器信任并类型检查
- `@js("module")` 注解：指定 extern 来源，codegen 生成 import
- 类型映射：AG 基础类型 ↔ JS 类型的双向规则
- `Promise<T>` 作为内置类型，async 互操作桥梁
- 标准库分层架构：Layer A (Web 标准 extern) + Layer B (AG 封装)
- `std:*` 模块路径在编译器中可解析

**Non-Goals:**
- 不实现 JS → AG 的 FFI 回调（JS 调 AG 函数）— 后续 change
- 不实现运行时类型检查/validation（`int` 的 runtime guard 除外）
- 不实现具体的 HTTP server/client — 那是下一个 change
- 不实现 agent runtime API（`std:llm`, `std:memory`）— 后续 change

## Decisions

### 1. `extern` 语法设计

新增 `extern` 关键字（加入 reserved keywords 列表）。

**extern fn**：

```
extern fn fetch(input: str | Request, init: RequestInit?) -> Promise<Response>
```

编译器行为：
- Parser 生成 `ExternFnDecl` AST 节点（无函数体）
- Checker 注册到符号表，调用时按正常函数检查参数/返回类型
- Codegen **不产出函数定义**，调用处直接引用函数名

**extern struct**：

```
extern struct Request {
  method: str
  url: str
  headers: Headers
  fn json() -> Promise<any>
  fn text() -> Promise<str>
}
```

编译器行为：
- Parser 生成 `ExternStructDecl`（字段 + 方法签名，无方法体）
- Checker 注册类型，字段访问和方法调用按正常检查
- Codegen **完全擦除**——JS 侧这些类已存在

**extern type**：

```
extern type Headers
extern type URL
extern type ReadableStream
```

不透明类型——AG 只知道名称，不知道内部结构。可以传递引用，不能访问字段。

### 2. `@js("module")` 注解

`@js` 注解附着在 extern 声明前，指定 JS 来源：

```
// 全局可用，无需 import
extern fn fetch(...) -> ...

// 从特定模块 import
@js("node:fs/promises")
extern fn readFile(path: str, encoding: str) -> Promise<str>

// 从模块 import 并重命名
@js("node:path", name = "join")
extern fn path_join(parts: ...str) -> str
```

**Codegen 规则**：

| 场景 | AG 声明 | JS 输出 |
|------|---------|---------|
| 无 `@js` | `extern fn fetch(...)` | 不产出 import，直接引用 `fetch` |
| `@js("mod")` | `@js("mod") extern fn foo(...)` | `import { foo } from "mod";` |
| `@js("mod", name="bar")` | `@js("mod", name="bar") extern fn foo(...)` | `import { bar as foo } from "mod";` |
| `@js("mod")` on struct | `@js("mod") extern struct Foo {}` | `import { Foo } from "mod";` |
| 多个声明同一模块 | 两个 `@js("mod")` extern | 合并为一条 `import { a, b } from "mod";` |

**AST 表示**：

```rust
struct JsAnnotation {
    module: Option<String>,     // None = global
    js_name: Option<String>,    // None = same as AG name
    span: Span,
}
```

`@js` 注解在 Parser 层解析，附着在下一个 extern 声明上。

### 3. 类型映射规则

**直通类型**（AG → JS 零开销，同一运行时表示）：

| AG | JS | 备注 |
|---|---|---|
| `str` | `string` | 完全相同 |
| `num` | `number` | 完全相同 |
| `int` | `number` | AG 侧是语义标记，JS 侧是 number |
| `bool` | `boolean` | 完全相同 |
| `nil` | `null` | AG 的 `nil` 编译为 JS 的 `null` |
| `any` | `any` | 逃逸阀门 |
| `[T]` | `Array<T>` | 直通 |
| `{K: V}` | object / `Record<K, V>` | 直通 |
| `T?` | `T \| null` | 直通 |
| `fn(A) -> B` | `(a) => b` | 函数引用直通 |

**结构化类型映射**（有规则的映射）：

| AG | JS | 规则 |
|---|---|---|
| `struct Foo { a: str, b: int }` | `{ a: "...", b: 42 }` | 结构化子类型，字段名对应 |
| `enum Status { A, B(x: int) }` | `{ tag: "A" }` / `{ tag: "B", x: 42 }` | tagged union |

**特殊类型**：

| AG | JS | 规则 |
|---|---|---|
| `Promise<T>` | `Promise<T>` | 新增内置泛型类型 |
| `Error` | `Error` | AG 的 Error 编译为 JS Error |

**`Promise<T>` 是唯一允许的泛型内置类型**。不引入通用泛型机制（保持类型系统简单），只为 Promise 开特例——因为 async 互操作是硬需求。

### 4. `Promise<T>` 和 async 边界

```
// extern 返回 Promise —— AG 侧用 await 消费
extern fn fetch(url: str) -> Promise<Response>

let resp = await fetch("https://example.com")
// resp 的类型是 Response，await 自动解包 Promise<Response> → Response
```

Checker 规则：
- `await expr` 的 `expr` 类型必须是 `Promise<T>`，结果类型为 `T`
- `async fn foo() -> T` 的实际返回类型是 `Promise<T>`，调用者看到 `Promise<T>`
- 在非 async 上下文中 `await` 产生 diagnostic

Codegen：
- `await expr` → `await expr`（JS 原生）
- `async fn` → `async function`（已在 basic-language-core 中处理）

### 5. 标准库模块架构

```
crates/ag-stdlib/
├── Cargo.toml
├── src/
│   └── lib.rs              # 模块路径解析逻辑
├── modules/
│   ├── web/
│   │   ├── fetch.ag        # extern: fetch, Request, Response, Headers
│   │   ├── crypto.ag       # extern: crypto.subtle, randomUUID
│   │   ├── encoding.ag     # extern: TextEncoder, TextDecoder, atob, btoa
│   │   ├── streams.ag      # extern: ReadableStream, WritableStream
│   │   └── timers.ag       # extern: setTimeout, setInterval, ...
│   ├── log.ag              # AG-style logging API
│   ├── encoding.ag         # json/yaml/toml 便捷封装
│   ├── env.ag              # 环境变量
│   └── fs.ag               # 文件系统
```

**模块路径解析**：

`import { fetch } from "std:web/fetch"` 在编译器中解析步骤：
1. 识别 `std:` 前缀 → 标准库模块
2. 查找 `ag-stdlib/modules/web/fetch.ag`
3. 解析该文件中的 extern 声明
4. 将声明注入当前编译单元的符号表

对于 Layer B 模块（如 `std:log`），`.ag` 文件中的非-extern 函数编译为：
```javascript
import { log } from "@agentscript/stdlib/log";
```

**`@agentscript/stdlib` npm 包结构**：

```
@agentscript/stdlib/
├── package.json
├── log/index.js          # Layer B: structured logging 实现
├── encoding/index.js     # Layer B: json/yaml/toml wrappers
├── env/index.js          # Layer B: env var access
└── fs/index.js           # Layer B: fs 封装
```

### 6. ag-stdlib Rust crate 的角色

`crates/ag-stdlib/` 是一个 Rust crate，但主要内容是 `.ag` 声明文件（嵌入为 `include_str!` 或从文件系统读取）。它向编译器提供：

```rust
pub fn resolve_std_module(path: &str) -> Option<&str> {
    match path {
        "std:web/fetch" => Some(include_str!("../modules/web/fetch.ag")),
        "std:log" => Some(include_str!("../modules/log.ag")),
        // ...
        _ => None,
    }
}
```

编译器在处理 `import ... from "std:..."` 时调用此函数获取模块源码，然后正常 lex/parse 该模块。

### 7. Variadic 参数 (`...T`) 处理

spec 中提到了 `fn join(parts: ...str) -> str`。为了支持 extern 声明中的 variadic，新增 `...T` 语法表示 rest 参数：

```
extern fn console_log(args: ...any) -> nil
```

编译为 JS 的 rest 参数 `(...args)`。仅 extern 声明和函数最后一个参数可用。

## Risks / Trade-offs

- **`Promise<T>` 作为唯一泛型特例**：打破了"无泛型"的设计原则 → 可接受，因为 async 是硬需求。不引入通用泛型——`Promise` 在 checker 中硬编码处理。
- **`int` 无运行时保证**：AG 的 `int` 编译后就是 JS `number`，不做 `Math.trunc` 检查 → 可接受，这是类型系统的语义标记而非运行时保证。如果需要真正的整数，后续可加 `@checked` 注解。
- **extern 声明的正确性**：编译器信任 extern 声明，如果声明和 JS 实际 API 不匹配，运行时才会出错 → 和 TypeScript 的 `.d.ts` 是同样的 trust 模型。标准库由我们维护，外部 extern 是用户责任。
- **标准库 .ag 文件嵌入**：将 .ag 文件嵌入 Rust binary 增加编译体积 → 可接受，标准库 .ag 文件体积很小（几 KB），且避免了运行时文件查找。
- **Layer B 的 npm 包维护**：每个 Layer B 模块需要同时维护 .ag 声明和 JS 实现 → 通过代码生成或约定降低不一致风险。
