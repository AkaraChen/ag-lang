## 1. ag-stdlib：HTTP 模块声明文件

- [x] 1.1 创建 `crates/ag-stdlib/modules/http/` 目录
- [x] 1.2 编写 `modules/http/server.ag`：extern struct `App`（get/post/put/delete/patch/use/route/fetch 方法）、extern struct `Context`（req/text/json/html/redirect/header/status）、extern struct `HonoRequest`（param/query/header/json/text）、`App()` 工厂函数声明
- [x] 1.3 编写 `modules/http/client.ag`：`HttpOptions` struct 声明（headers/body/timeout）、`get/post/put/del/patch/head/options` 函数声明
- [x] 1.4 在 `resolve_std_module` 中注册 `std:http/server` 和 `std:http/client` 路径
- [x] 1.5 验证 .ag 声明文件可被编译器正确解析（无 parse error）

## 2. @agentscript/stdlib：HTTP Server JS 实现

- [x] 2.1 创建 `@agentscript/stdlib/http/server/` 目录
- [x] 2.2 添加 `hono` 为 `@agentscript/stdlib` 的依赖
- [x] 2.3 实现 `http/server/index.js`：导出 `App()` 工厂函数（内部 `new Hono()`）
- [x] 2.4 编写单元测试：`App()` 创建 Hono 实例、路由注册、handler 调用
- [x] 2.5 编写单元测试：Context 方法（text/json/html/redirect/header/status）
- [x] 2.6 编写单元测试：HonoRequest 方法（param/query/header/json/text）
- [x] 2.7 编写单元测试：中间件注册和 next() 调用
- [x] 2.8 编写单元测试：子应用 route() 挂载
- [x] 2.9 编写单元测试：app.fetch() 返回正确 Response

## 3. @agentscript/stdlib：HTTP Client JS 实现

- [x] 3.1 创建 `@agentscript/stdlib/http/client/` 目录
- [x] 3.2 实现 `http/client/index.js`：内部 `buildInit(options)` 工具函数（headers 处理、timeout → AbortSignal.timeout）
- [x] 3.3 实现 `get(url, options)` 函数
- [x] 3.4 实现 `post(url, options)` 函数（含 body 自动序列化逻辑）
- [x] 3.5 实现 `put(url, options)` 函数
- [x] 3.6 实现 `del(url, options)` 函数
- [x] 3.7 实现 `patch(url, options)` 函数
- [x] 3.8 实现 `head(url, options)` 和 `options(url, opts)` 函数
- [x] 3.9 实现 body 自动序列化：object/array → JSON.stringify + Content-Type: application/json，string → text/plain，nil → no body
- [x] 3.10 实现 explicit headers 覆盖自动 Content-Type 的逻辑
- [x] 3.11 编写单元测试：各 HTTP method 函数的 fetch 调用验证
- [x] 3.12 编写单元测试：body 自动序列化（object、array、string、nil）
- [x] 3.13 编写单元测试：timeout 参数转换为 AbortSignal
- [x] 3.14 编写单元测试：custom headers 和 header override

## 4. @agentscript/serve 包搭建

- [x] 4.1 创建 `packages/agentscript-serve/` 目录
- [x] 4.2 初始化 `package.json`：name `@agentscript/serve`、bin entry、dependencies（`@hono/node-server`）、peerDependencies（asc compiler）
- [x] 4.3 配置 TypeScript（`tsconfig.json`）
- [x] 4.4 实现 `src/index.ts`：`serve(app, options?)` 函数，包装 `@hono/node-server` 的 `serve()`，默认端口 3000，打印启动信息
- [x] 4.5 实现 SIGINT/SIGTERM 优雅关闭
- [x] 4.6 编写 serve() 单元测试：默认端口、自定义端口、启动信息

## 5. @agentscript/serve CLI

- [x] 5.1 实现 `src/cli.ts`：解析命令行参数（entry file, --port, --dev）
- [x] 5.2 实现编译流程：调用 `asc build <entry.ag>` 子进程
- [x] 5.3 实现模块加载：动态 import 编译后的 JS 模块，检测 default export
- [x] 5.4 实现错误处理：文件不存在、编译错误、无 default export
- [x] 5.5 实现 `--dev` 模式：watch .ag 文件 → 重新编译 → 重启 server
- [x] 5.6 实现 dev 模式编译错误恢复：打印错误但保持上一次 server 运行
- [x] 5.7 编写 CLI 集成测试：基本编译+启动、--port 参数、文件不存在报错
- [x] 5.8 编写 CLI 集成测试：--dev 模式文件变更触发重启

## 6. 编译器集成测试

- [x] 6.1 端到端测试：`import { App, Context } from "std:http/server"` → 编译 → 验证 JS 含 `import { App } from "@agentscript/stdlib/http/server"`
- [x] 6.2 端到端测试：完整 server 应用（App + 路由 + handler）→ 编译 → 验证 JS 输出
- [x] 6.3 端到端测试：`import { get, post } from "std:http/client"` → 编译 → 验证 JS 含 `import { get, post } from "@agentscript/stdlib/http/client"`
- [x] 6.4 端到端测试：client 调用（get + post with body）→ 编译 → 验证 JS 输出
- [x] 6.5 端到端测试：server + client 混合使用 → 编译 → 验证完整 JS 输出
- [x] 6.6 类型检查测试：handler 签名错误（参数类型不匹配）→ 编译 → 验证 checker 报错
