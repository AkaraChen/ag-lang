## 1. ag-ast：DSL 节点定义

- [x] 1.1 定义 `DslBlock` 结构体：`kind: String`, `name: Ident`, `content: DslContent`, `span: Span`
- [x] 1.2 定义 `DslContent` 枚举：`Inline { parts: Vec<DslPart> }` 和 `FileRef { path: String, span: Span }`
- [x] 1.3 定义 `DslPart` 枚举：`Text(String, Span)` 和 `Capture(Box<Expr>, Span)`
- [x] 1.4 将 `DslBlock` 作为新变体加入 `Item` 枚举

## 2. ag-lexer：DSL 扫描支持

- [x] 2.1 新增 token 类型：`At`, `DslBlockStart`, `DslBlockEnd`, `DslText(String)`, `DslCaptureStart`, `DslCaptureEnd`
- [x] 2.2 实现 `enter_dsl_raw_mode()` 方法：期望 `` ``` `` + 换行，emit `DslBlockStart`，切换到 raw mode 状态
- [x] 2.3 实现 raw mode 文本扫描：逐字符收集，遇到 `#{` 或行首 `` ``` `` 时截断输出 `DslText`
- [x] 2.4 实现 `#{` capture 边界：emit `DslCaptureStart`，切回 normal mode；跟踪 `{}` 嵌套深度，最外层 `}` 时 emit `DslCaptureEnd` 并返回 raw mode
- [x] 2.5 实现行首 `` ``` `` 检测作为 block end：允许前导空白，emit `DslBlockEnd` 并返回 normal mode
- [x] 2.6 实现 `#` 不跟 `{` 时作为普通文本处理
- [x] 2.7 实现 unterminated DSL block 错误：EOF 时仍在 raw mode 则 emit Error token
- [x] 2.8 编写 lexer DSL 模式单元测试：覆盖所有 dsl-lexing spec scenario

## 3. ag-parser：DSL 块解析

- [x] 3.1 在顶层 `parse_module` 中识别 `At` token，调用 `parse_dsl_block()`
- [x] 3.2 实现 `parse_dsl_block()`：consume `@` → ident (kind) → ident (name) → 判断 `` ``` `` 或 `from`
- [x] 3.3 实现内联块解析：调用 `lexer.enter_dsl_raw_mode()`，循环收集 `DslText` → `DslPart::Text`，`DslCaptureStart` → parse expr → `DslPart::Capture`，直到 `DslBlockEnd`
- [x] 3.4 实现 capture 内表达式解析：复用 `parse_expr()`，只允许单一表达式，不允许语句
- [x] 3.5 实现文件引用解析：consume `from` → expect string literal → `DslContent::FileRef(path)`
- [x] 3.6 实现 `@` 后的错误处理：缺少 kind、缺少 name、缺少 body 的三种 diagnostic
- [x] 3.7 编写 parser DSL 块单元测试：覆盖所有 dsl-parsing spec scenario

## 4. ag-checker：Capture 表达式类型检查

- [x] 4.1 在 `check_module` 中遍历 `Item::DslBlock`，进入 `check_dsl_block()`
- [x] 4.2 实现 `check_dsl_block()`：遍历 `DslPart::Capture(expr)`，对每个 expr 调用标准的 `check_expr()`
- [x] 4.3 确认 capture 表达式类型不被约束——只推断、不报类型不匹配错误
- [x] 4.4 编写 checker DSL capture 单元测试：valid capture、undefined var、type inference

## 5. ag-codegen：DslHandler 框架

- [x] 5.1 定义 `DslHandler` trait：`fn handle(&self, block: &DslBlock, ctx: &mut CodegenContext) -> Result<Vec<ModuleItem>, CodegenError>`
- [x] 5.2 在 `CodegenContext` 上实现 `translate_expr(&mut self, expr: &ag_ast::Expr) -> swc_ecma_ast::Expr`
- [x] 5.3 在 `Translator` 中添加 `handlers: HashMap<String, Box<dyn DslHandler>>` 和 `register_dsl_handler()` 方法
- [x] 5.4 在 codegen 主循环中处理 `Item::DslBlock`：查找 handler，找到则调用，未找到则报 CodegenError
- [x] 5.5 实现内置 prompt handler（Inline）：`DslPart::Text` → 模板字面量文本段，`DslPart::Capture` → `${}` 插值，输出 `const <name> = `...`;`
- [x] 5.6 实现内置 prompt handler（FileRef）：生成运行时文件读取代码
- [x] 5.7 在 `ag-cli` 中注册内置 prompt handler 到 Translator
- [x] 5.8 编写 codegen DSL handler 单元测试：覆盖所有 dsl-codegen-framework spec scenario

## 6. 端到端集成测试

- [x] 6.1 编写 `.ag` → `.js` 集成测试：`@prompt` 内联块 + capture → 验证 JS 模板字符串输出
- [x] 6.2 编写 `.ag` → `.js` 集成测试：`@prompt` from 文件引用 → 验证 JS 文件读取输出
- [x] 6.3 编写错误场景集成测试：未注册 handler、unterminated block、invalid capture expression
