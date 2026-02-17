## 1. Example 文件创建

- [x] 1.1 创建 `examples/http-server/` 目录
- [x] 1.2 编写 `examples/http-server/app.ag`：完整示例代码（import、函数定义、App 创建、4 个路由、pub fn setup）
- [x] 1.3 编写 `examples/http-server/README.md`：运行说明（前置依赖、启动命令、端点文档、curl 示例）

## 2. 编译验证

- [x] 2.1 验证 `asc build examples/http-server/app.ag` 编译成功，无 parse/type/codegen 错误
- [x] 2.2 检查编译输出 JS 包含 `import { App } from "@agentscript/stdlib/http/server"`
- [x] 2.3 检查编译输出 JS 包含 `function add`、`function subtract`、`function calculate` 声明
- [x] 2.4 检查编译输出 JS 包含 `export function setup`（适配：使用 pub fn setup 替代 export default）

## 3. 运行时端点测试

- [x] 3.1 测试 `GET /`：返回 200 + server info JSON（name、version、endpoints 数组）
- [x] 3.2 测试 `POST /echo`：发送 JSON 对象，验证原样返回
- [x] 3.3 测试 `POST /echo`：发送 JSON 数组，验证原样返回
- [x] 3.4 测试 `POST /calc` add：`{ op: "add", a: 10, b: 3 }` → `result: 13`
- [x] 3.5 测试 `POST /calc` subtract：`{ op: "subtract", a: 10, b: 3 }` → `result: 7`
- [x] 3.6 测试 `POST /calc` multiply：`{ op: "multiply", a: 4, b: 5 }` → `result: 20`
- [x] 3.7 测试 `POST /calc` divide：`{ op: "divide", a: 15, b: 4 }` → `result: 3.75`
- [x] 3.8 测试 `POST /calc` divide by zero：`{ op: "divide", a: 10, b: 0 }` → `result: 0`
- [x] 3.9 测试 `POST /calc` unknown op：`{ op: "modulo", a: 10, b: 3 }` → `result: 0`
- [x] 3.10 测试 `GET /greet/Alice` → `{ message: "Hello, Alice!" }`
- [x] 3.11 测试 `GET /greet/World` → `{ message: "Hello, World!" }`

## 4. CLI 集成验证

- [x] 4.1 验证 `agentscript-serve examples/http-server/app.ag` 可一键启动
- [x] 4.2 验证启动后 server 响应 `GET /` 正确
