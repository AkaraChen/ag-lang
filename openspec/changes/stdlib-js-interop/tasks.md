## 1. AST 扩展

- [x] 1.1 在 `ag-ast` 中新增 `JsAnnotation` 结构体：`module: Option<String>`, `js_name: Option<String>`, `span: Span`
- [x] 1.2 新增 `ExternFnDecl` AST 节点：`name`, `params` (含类型), `return_type`, `js_annotation: Option<JsAnnotation>`, `variadic: bool`, `span`
- [x] 1.3 新增 `ExternStructDecl` AST 节点：`name`, `fields: Vec<Field>`, `methods: Vec<MethodSignature>`, `js_annotation: Option<JsAnnotation>`, `span`
- [x] 1.4 新增 `ExternTypeDecl` AST 节点：`name`, `js_annotation: Option<JsAnnotation>`, `span`
- [x] 1.5 将 `ExternFnDecl`、`ExternStructDecl`、`ExternTypeDecl` 加入顶层 `Statement`/`Declaration` 枚举
- [x] 1.6 新增 `Type::Promise(Box<Type>)` 变体，作为内置泛型类型
- [x] 1.7 新增 `ParamKind::Variadic` 或在参数节点中增加 `is_variadic: bool` 字段

## 2. Lexer 扩展

- [x] 2.1 将 `extern` 加入保留关键字列表，emit `Token::Extern`
- [x] 2.2 确保 `extern` 不能用作标识符（已有关键字拒绝机制应覆盖）
- [x] 2.3 编写 lexer 测试：`extern` tokenization、`extern` 作为标识符报错

## 3. Parser：extern 声明

- [x] 3.1 实现 `parse_extern_fn_decl()`：解析 `extern fn <name>(<params>) -> <type>`，生成 `ExternFnDecl`，拒绝函数体
- [x] 3.2 实现 `parse_extern_struct_decl()`：解析 `extern struct <Name> { fields + method signatures }`，拒绝方法体
- [x] 3.3 实现 `parse_extern_type_decl()`：解析 `extern type <Name>`
- [x] 3.4 在顶层解析分发中处理 `Token::Extern`：peek 下一个 token 区分 `fn`/`struct`/`type`
- [x] 3.5 实现 variadic 参数解析：`...T` 语法，仅允许在最后一个参数位置，非最后位置报错
- [x] 3.6 编写 parser 测试：extern fn（简单、optional 参数、union 参数、无返回类型、有 body 报错）
- [x] 3.7 编写 parser 测试：extern struct（fields + methods、only fields、only methods、method body 报错）
- [x] 3.8 编写 parser 测试：extern type（简单、多个）
- [x] 3.9 编写 parser 测试：variadic 参数（正常、非末位报错）

## 4. Parser：@js 注解

- [x] 4.1 实现 `parse_js_annotation()`：解析 `@js("module")` 和 `@js("module", name = "jsName")`
- [x] 4.2 在顶层解析中检测 `@` token：如果后跟 `js`，则调用 `parse_js_annotation()`，然后要求下一个必须是 `extern`
- [x] 4.3 `@js` 后跟非 extern 声明时产生诊断错误
- [x] 4.4 将解析的 `JsAnnotation` 附着到后续 `ExternFnDecl`/`ExternStructDecl`/`ExternTypeDecl`
- [x] 4.5 编写 parser 测试：@js with module only、@js with name、无 @js、@js on struct/type
- [x] 4.6 编写 parser 测试：@js before non-extern 报错

## 5. Parser：Promise<T> 类型

- [x] 5.1 在类型解析中识别 `Promise` 标识符 + `<` 触发泛型类型解析
- [x] 5.2 解析 `Promise<T>` 为 `Type::Promise(Box<Type>)`，支持嵌套（`Promise<Promise<str>>`）
- [x] 5.3 确保 `Promise` 仅作为类型使用，不能作为值（不允许 `let p = Promise`）
- [x] 5.4 编写 parser 测试：Promise<Response>、Promise<str>、Promise<Promise<str>>

## 6. Checker：extern 声明注册

- [x] 6.1 处理 `ExternFnDecl`：注册函数签名到符号表（与普通 fn 共享调用逻辑）
- [x] 6.2 处理 `ExternStructDecl`：注册为类型，支持字段访问和方法调用的类型检查
- [x] 6.3 处理 `ExternTypeDecl`：注册为不透明类型，允许传递引用，拒绝字段/方法访问
- [x] 6.4 检查重复 extern 声明（同名同 scope 报错）
- [x] 6.5 编写 checker 测试：extern fn 调用（正确参数、类型不匹配、重复声明）
- [x] 6.6 编写 checker 测试：extern struct（字段访问、未知字段报错、方法调用）
- [x] 6.7 编写 checker 测试：extern type（传递合法、字段访问报错）

## 7. Checker：Promise 和 async 交互

- [x] 7.1 实现 `Promise<T>` 类型推断：识别 `Type::Promise` 并追踪内部类型 `T`
- [x] 7.2 `await expr`：验证 expr 类型为 `Promise<T>`，结果类型为 `T`
- [x] 7.3 `await` 非 Promise 产生类型错误
- [x] 7.4 `await` 在非 async 上下文中产生诊断错误
- [x] 7.5 `async fn foo() -> T`：对外返回类型为 `Promise<T>`，函数体内 return 语句返回 `T`
- [x] 7.6 编写 checker 测试：await unwrap Promise、await non-Promise 报错、await outside async 报错、async fn 返回类型

## 8. Checker：variadic 参数

- [x] 8.1 extern fn variadic 参数：允许 0 或多个参数匹配 `...T`
- [x] 8.2 variadic 参数类型检查：每个实际参数须匹配元素类型 `T`
- [x] 8.3 编写 checker 测试：variadic 正确调用、参数类型不匹配

## 9. Codegen：extern 声明擦除

- [x] 9.1 `ExternFnDecl` 不产出任何 JS 函数定义
- [x] 9.2 `ExternStructDecl` 不产出任何 JS class 定义
- [x] 9.3 `ExternTypeDecl` 不产出任何 JS 输出
- [x] 9.4 extern fn 调用处直接引用函数名（与普通函数调用 codegen 相同）
- [x] 9.5 编写 codegen 测试：extern fn 调用输出、extern struct/type 无输出

## 10. Codegen：@js import 生成

- [x] 10.1 收集所有被引用的 `@js` extern 声明，按 module 分组
- [x] 10.2 生成 `import { name } from "module";` 语句
- [x] 10.3 支持 aliased import：`import { jsName as agName } from "module";`
- [x] 10.4 合并同一 module 的多个 import 为单条 `import { a, b } from "module";`
- [x] 10.5 将所有 import 语句放在输出文件顶部
- [x] 10.6 未引用的 @js extern 不生成 import（dead-code elimination）
- [x] 10.7 无 @js 的 extern（全局 API）不生成 import
- [x] 10.8 编写 codegen 测试：单 import、aliased import、合并 import、未引用不输出、全局无 import、import 置顶

## 11. Codegen：Promise/async 直通

- [x] 11.1 `await expr` → JS `await expr`（确认已有 basic-language-core 实现，或补充）
- [x] 11.2 `async fn` → JS `async function`（确认已有实现，或补充）
- [x] 11.3 编写 codegen 测试：await 与 async fn 输出

## 12. ag-stdlib crate 搭建

- [x] 12.1 创建 `crates/ag-stdlib/` 目录和 `Cargo.toml`（无 ag-lang 依赖）
- [x] 12.2 添加 `ag-stdlib` 到 workspace members
- [x] 12.3 实现 `src/lib.rs`：`pub fn resolve_std_module(path: &str) -> Option<&str>` 使用 `include_str!`
- [x] 12.4 创建 `modules/` 目录结构：`web/`, `log.ag`, `encoding.ag`, `env.ag`, `fs.ag`

## 13. Layer A 标准库模块（Web 标准 extern 声明）

- [x] 13.1 编写 `modules/web/fetch.ag`：extern fn `fetch`、extern struct `Request`/`Response`、extern type `Headers`/`URL`/`URLSearchParams`
- [x] 13.2 编写 `modules/web/crypto.ag`：extern 声明 `crypto.subtle`、`crypto.randomUUID()`
- [x] 13.3 编写 `modules/web/encoding.ag`：extern 声明 `TextEncoder`/`TextDecoder`/`atob`/`btoa`
- [x] 13.4 编写 `modules/web/streams.ag`：extern type `ReadableStream`/`WritableStream`/`TransformStream`
- [x] 13.5 编写 `modules/web/timers.ag`：extern fn `setTimeout`/`setInterval`/`clearTimeout`/`clearInterval`

## 14. Layer B 标准库模块（AG 封装声明）

- [x] 14.1 编写 `modules/log.ag`：声明 `info`/`warn`/`error`/`debug` 函数签名
- [x] 14.2 编写 `modules/encoding.ag`：声明 `json.parse`/`json.stringify` 等函数签名
- [x] 14.3 编写 `modules/env.ag`：声明 `get`/`set` 函数签名
- [x] 14.4 编写 `modules/fs.ag`：声明 `readFile`/`writeFile` 等函数签名
- [x] 14.5 在 `resolve_std_module` 中注册所有 Layer A 和 Layer B 模块路径

## 15. 编译器集成：std: 模块解析

- [x] 15.1 在模块解析器中识别 `std:` 前缀的 import 路径
- [x] 15.2 调用 `ag-stdlib::resolve_std_module()` 获取模块源码
- [x] 15.3 将获取的 `.ag` 源码送入 lexer → parser 管线处理
- [x] 15.4 将解析出的声明注入当前编译单元符号表
- [x] 15.5 支持选择性 import：只将 `import { a, b }` 中指定的符号加入当前 scope
- [x] 15.6 未知 std 模块路径报错、import 不存在的符号报错
- [x] 15.7 Layer B 模块 codegen：函数调用编译为 `import { fn } from "@agentscript/stdlib/<module>"`
- [x] 15.8 在 `ag-codegen` 或 `ag-parser` 的 `Cargo.toml` 中添加 `ag-stdlib` 依赖

## 16. 集成测试

- [x] 16.1 端到端测试：extern fn + @js 注解 → 编译 → 验证 JS 输出含正确 import 和函数调用
- [x] 16.2 端到端测试：extern struct 字段访问和方法调用 → 编译 → 验证 JS 输出
- [x] 16.3 端到端测试：`import { fetch } from "std:web/fetch"` → 编译 → 验证 JS 输出无 stdlib import（全局 API）
- [x] 16.4 端到端测试：`import { info } from "std:log"` → 编译 → 验证 JS 输出含 `@agentscript/stdlib/log` import
- [x] 16.5 端到端测试：async fn + await + extern Promise 返回 → 编译 → 验证完整 JS 输出
- [x] 16.6 错误场景测试：extern fn body、@js on non-extern、await non-Promise、unknown std module
