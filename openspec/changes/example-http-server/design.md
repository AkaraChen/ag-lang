## Context

这是一个纯用户代码示例，不涉及编译器修改。目的是展示 `basic-language-core` + `stdlib-js-interop` + `http-stdlib` 三个 change 组合后的端到端体验。

## Goals / Non-Goals

**Goals:**
- 一个 `app.ag` 文件演示尽可能多的核心语言特性
- 可通过 `npx @agentscript/serve app.ag` 一键运行
- 每个端点侧重展示不同的语言特性组合

**Non-Goals:**
- 不展示 prompt DSL（后续 example）
- 不展示高级特性（enum、match、pipe、error handling）
- 不做性能测试或压测

## Decisions

### 1. 示例代码设计

```ag
import { App, Context } from "std:http/server"

// --- 演示：函数定义、算术运算、if/else ---

fn add(a: num, b: num) -> num {
  return a + b
}

fn subtract(a: num, b: num) -> num {
  return a - b
}

fn calculate(op: str, a: num, b: num) -> num {
  if op == "add" {
    return add(a, b)
  } else if op == "subtract" {
    return subtract(a, b)
  } else if op == "multiply" {
    return a * b
  } else if op == "divide" {
    if b == 0 {
      return 0
    }
    return a / b
  }
  return 0
}

// --- 演示：let 绑定、App 创建 ---

let app = App()

// --- 演示：GET 路由、对象字面量、数组字面量 ---

app.get("/", fn(c: Context) -> Response {
  return c.json({
    name: "AgentScript Example Server",
    version: "0.1.0",
    endpoints: ["/", "/echo", "/calc", "/greet/:name"]
  })
})

// --- 演示：POST 路由、async/await、JSON body 解析 ---

app.post("/echo", async fn(c: Context) -> Response {
  let body = await c.req.json()
  return c.json(body)
})

// --- 演示：函数调用、对象字段访问、动态计算 ---

app.post("/calc", async fn(c: Context) -> Response {
  let body = await c.req.json()
  let result = calculate(body.op, body.a, body.b)
  return c.json({
    op: body.op,
    a: body.a,
    b: body.b,
    result: result
  })
})

// --- 演示：路径参数、字符串拼接 ---

app.get("/greet/:name", fn(c: Context) -> Response {
  let name = c.req.param("name")
  return c.json({
    message: "Hello, " + name + "!"
  })
})

export default app
```

### 2. 预期编译输出

```javascript
import { App } from "@agentscript/stdlib/http/server";

function add(a, b) {
  return a + b;
}

function subtract(a, b) {
  return a - b;
}

function calculate(op, a, b) {
  if (op === "add") {
    return add(a, b);
  } else if (op === "subtract") {
    return subtract(a, b);
  } else if (op === "multiply") {
    return a * b;
  } else if (op === "divide") {
    if (b === 0) {
      return 0;
    }
    return a / b;
  }
  return 0;
}

const app = App();

app.get("/", (c) => {
  return c.json({
    name: "AgentScript Example Server",
    version: "0.1.0",
    endpoints: ["/", "/echo", "/calc", "/greet/:name"]
  });
});

app.post("/echo", async (c) => {
  const body = await c.req.json();
  return c.json(body);
});

app.post("/calc", async (c) => {
  const body = await c.req.json();
  const result = calculate(body.op, body.a, body.b);
  return c.json({
    op: body.op,
    a: body.a,
    b: body.b,
    result: result
  });
});

app.get("/greet/:name", (c) => {
  const name = c.req.param("name");
  return c.json({
    message: "Hello, " + name + "!"
  });
});

export default app;
```

### 3. 语言特性覆盖矩阵

| 特性 | 端点 | 说明 |
|------|------|------|
| `import` | 顶层 | `std:http/server` 模块导入 |
| `fn` 定义 | 顶层 | `add`, `subtract`, `calculate` |
| 参数类型注解 | 顶层函数 | `a: num, b: num` |
| 返回类型注解 | 顶层函数 | `-> num`, `-> Response` |
| `let` 绑定 | 各处 | `let app`, `let body`, `let name` |
| 算术运算 | `/calc` | `+`, `-`, `*`, `/` |
| 比较运算 | `/calc` | `==` |
| `if`/`else if`/`else` | `/calc` | 四分支 calculate |
| 函数调用 | `/calc` | `add(a, b)`, `calculate(...)` |
| 对象字面量 | `/`, `/calc` | `{ name: "...", ... }` |
| 数组字面量 | `/` | `["/", "/echo", ...]` |
| 字符串字面量 | 各处 | `"Hello, "` |
| 数字字面量 | `/calc` | `0` |
| 字符串拼接 | `/greet` | `"Hello, " + name + "!"` |
| `async fn` | `/echo`, `/calc` | async handler |
| `await` | `/echo`, `/calc` | `await c.req.json()` |
| 方法调用 | 各处 | `c.json(...)`, `c.req.param(...)` |
| 字段访问 | `/calc` | `body.op`, `body.a` |
| 路径参数 | `/greet/:name` | `c.req.param("name")` |
| `export default` | 底部 | 导出 App 实例 |

### 4. 目录结构

```
examples/
└── http-server/
    ├── app.ag          # 主应用代码
    └── README.md       # 运行说明
```

## Risks / Trade-offs

- **依赖全部前置 change**：example 只有在 basic-language-core + stdlib-js-interop + http-stdlib 全部实现后才能运行 → 可先写代码，作为编译器的集成测试目标
- **`body.op` 字段访问的类型**：`await c.req.json()` 返回 `any`，`body.op` 是 `any` 字段访问——需要编译器支持 `any` 的字段访问直通 → 这在 basic-language-core 的 type-system spec 中已覆盖
