## 1. ag-dsl-core crate 搭建

- [x] 1.1 创建 `crates/ag-dsl-core/` crate，添加到 workspace members
- [x] 1.2 定义 `Span` 类型（`start: u32, end: u32`），或从 `ag-ast` re-export
- [x] 1.3 定义 `DslError { message: String, span: Option<Span> }`
- [x] 1.4 定义 `DslPart` 枚举：`Text(String, Span)`, `Capture(Box<dyn Any>, Span)`
- [x] 1.5 定义 `DslContent` 枚举：`Inline { parts: Vec<DslPart> }`, `FileRef { path: String, span: Span }`
- [x] 1.6 定义 `DslBlock { kind: String, name: String, content: DslContent, span: Span }`
- [x] 1.7 定义 `CodegenContext` trait：`fn translate_expr(&mut self, expr: &dyn Any) -> swc_ecma_ast::Expr`
- [x] 1.8 定义 `DslHandler` trait：`fn handle(&self, block: &DslBlock, ctx: &mut dyn CodegenContext) -> Result<Vec<ModuleItem>, DslError>`
- [x] 1.9 确认 `ag-dsl-core` 不依赖 `ag-ast`，仅依赖 `swc_ecma_ast`
- [x] 1.10 `cargo build` 验证 crate 编译通过

## 2. ag-dsl-prompt：Prompt AST 定义

- [x] 2.1 创建 `crates/ag-dsl-prompt/` crate，依赖 `ag-dsl-core` + SWC crates
- [x] 2.2 定义 `PromptTemplate { name: String, sections: Vec<PromptSection>, model: Option<ModelSpec>, output: Option<OutputSpec>, constraints: Option<Constraints> }`
- [x] 2.3 定义 `PromptSection` 枚举：`Role { role: RoleName, body: Vec<PromptPart> }`, `Examples(Vec<Example>)`, `Messages { capture_index: usize }`
- [x] 2.4 定义 `PromptPart` 枚举：`Text(String)`, `Capture(usize)`
- [x] 2.5 定义 `RoleName` 枚举：`System`, `User`, `Assistant`, `Custom(String)`
- [x] 2.6 定义 `Example { pairs: Vec<(RoleName, String)> }`
- [x] 2.7 定义 `ModelSpec { models: Vec<String> }`
- [x] 2.8 定义 `OutputSpec` + `OutputKind`：`CaptureRef(usize)`, `Inline(Vec<OutputField>)`
- [x] 2.9 定义 `Constraints { fields: Vec<(String, ConstraintValue)> }` + `ConstraintValue` 枚举

## 3. ag-dsl-prompt：Prompt Lexer

- [x] 3.1 定义 `PromptToken` 枚举（DirectiveRole, DirectiveModel, DirectiveExamples, DirectiveOutput, DirectiveConstraints, DirectiveMessages, Text, Capture, BraceOpen/Close, Colon, Pipe, StringLiteral, NumberLiteral, ArrayOpen/Close, Ident, Eof）
- [x] 3.2 实现 `lex(parts: &[DslPart]) -> Vec<PromptToken>` 入口
- [x] 3.3 实现 Text 段内的指令识别：行首 `@` + 已知关键字 → directive token，否则为 Text
- [x] 3.4 实现 `@role <name>` 解析：提取 role name（system/user/assistant/自定义）
- [x] 3.5 实现 `@model` 后的 ident + `|` 分隔模型名解析
- [x] 3.6 实现 `@examples` / `@constraints` 后的 `{...}` 结构体 lexing：string literals, number literals, idents, colons, array brackets
- [x] 3.7 实现 `@output` 后的 capture 或 `{...}` 内联 schema lexing
- [x] 3.8 实现 `@messages` 后的 capture token
- [x] 3.9 实现 `DslPart::Capture` → `PromptToken::Capture(index)` 透传
- [x] 3.10 编写 lexer 单元测试：覆盖所有 prompt-dsl-parser spec 中的 lexer scenario

## 4. ag-dsl-prompt：Prompt Parser

- [x] 4.1 实现 `parse(tokens: &[PromptToken]) -> Result<PromptTemplate, Vec<Diagnostic>>` 入口
- [x] 4.2 实现状态机主循环：根据当前 token 分发到 directive parser 或文本收集
- [x] 4.3 实现 `parse_role_section()`：收集后续 Text/Capture 直到下一个 directive
- [x] 4.4 实现默认 system role：无 `@role` 前的文本归入 implicit system section
- [x] 4.5 实现 `parse_model()`：解析 ident (| ident)* 序列为 `ModelSpec`
- [x] 4.6 实现 `parse_examples()`：解析 `{ role: "content", ... }` 为 `Example`
- [x] 4.7 实现 `parse_output()`：区分 capture ref 和内联 `{ field: type }` schema
- [x] 4.8 实现 `parse_constraints()`：解析 `{ key: value }` 为 `Constraints`
- [x] 4.9 实现 `parse_messages()`：期望 capture token，否则报错
- [x] 4.10 实现错误处理：examples 缺 `{`, messages 缺 capture, 空 prompt 等 diagnostic
- [x] 4.11 编写 parser 单元测试：覆盖所有 prompt-dsl-parser spec scenario

## 5. ag-dsl-prompt：Validator

- [x] 5.1 实现 `validate(template: &PromptTemplate) -> Vec<Diagnostic>` 入口
- [x] 5.2 检查 @model 重复（error）
- [x] 5.3 检查 @output 重复（error）
- [x] 5.4 检查 @constraints 重复（error）
- [x] 5.5 检查无 @role 时的 implicit system warning
- [x] 5.6 编写 validator 单元测试

## 6. ag-dsl-prompt：Codegen

- [x] 6.1 实现 `PromptCodegen` 结构体，持有 captures 列表（原始 `DslPart::Capture`）和 `&mut dyn CodegenContext`
- [x] 6.2 实现 `generate(template: &PromptTemplate, ...) -> Vec<swc_ecma_ast::ModuleItem>` 入口
- [x] 6.3 实现 runtime import 生成：`import { PromptTemplate } from "@agentscript/prompt-runtime"`，去重（单 module 只 emit 一次）
- [x] 6.4 实现 Role section codegen：纯 Text → string literal, 含 Capture → `(ctx) => \`...\${ctx.name}...\`` 箭头函数
- [x] 6.5 实现 capture 变量命名：简单 Ident → `ctx.<name>`, 复杂表达式 → `ctx.__capture_<index>`
- [x] 6.6 实现 ModelSpec codegen：`model: ["model1", "model2"]` 数组
- [x] 6.7 实现 Examples codegen：`examples: [{ role: "...", content: "..." }, ...]` 数组
- [x] 6.8 实现 Messages codegen：`messagesPlaceholder: "<name>"` 属性
- [x] 6.9 实现 OutputSpec codegen（inline）：AgentScript type → JSON Schema 对象（str→string, num→number, int→integer, bool→boolean, [T]→array, 嵌套 object）
- [x] 6.10 实现 OutputSpec codegen（capture ref）：通过 `ctx.translate_expr()` 翻译为 JS 表达式
- [x] 6.11 实现 Constraints codegen：`constraints: { key: value }` 对象字面量
- [x] 6.12 实现 FileRef codegen：生成文件读取 + PromptTemplate 构造
- [x] 6.13 组装完整输出：`const <name> = new PromptTemplate({ messages: [...], model: [...], ... });`
- [x] 6.14 编写 codegen 单元测试：覆盖所有 prompt-dsl-codegen spec scenario

## 7. ag-dsl-prompt：DslHandler 实现

- [x] 7.1 实现 `PromptDslHandler` struct，impl `DslHandler`
- [x] 7.2 `handle()` 方法：DslBlock → lex parts → parse → validate → codegen → 返回 ModuleItem
- [x] 7.3 处理 `DslContent::FileRef`：读文件 → 作为 Text 包装 → 走 codegen 路径
- [x] 7.4 错误映射：将内部 Diagnostic 转为 `DslError`

## 8. 集成和注册

- [x] 8.1 在 `ag-codegen` 中将 `DslHandler` trait 改为 re-export from `ag-dsl-core`（避免重复定义）
- [x] 8.2 在 `ag-cli` 中注册 `PromptDslHandler`：`translator.register_dsl_handler("prompt", Box::new(PromptDslHandler))`
- [x] 8.3 编写端到端集成测试：完整 `.ag` 文件含 `@prompt` 块 → 编译 → 验证 JS 输出含 `PromptTemplate` 构造
- [x] 8.4 编写错误场景集成测试：invalid prompt 语法、missing handler、unterminated block
