## Why

AgentScript 的多个核心原语（`prompt`、`component`、`skill`、未来的 `agent` 内嵌体）都需要嵌入非 AgentScript 语法的内容。目前 spec 中的 prompt literal 是一个特例——硬编码在语言内的 DSL。但 component 需要嵌入 JSX/HTML，未来可能还有 CSS、SQL、GraphQL 等。需要一个**通用的 DSL 嵌入框架**，让任何 DSL 类型都能使用内联块或外部文件引用，而非为每种 DSL 写一套特化的 lexer/parser 逻辑。

## What Changes

- 引入通用的 **DSL 块语法**，以 `@` 为触发符号，两种形式：
  - **内联块**：`` @<dsl_name> <Name> ``` \n <raw content> ``` `` — `@` 标记 DSL 类型，三反引号包裹原始内容
  - **外部引用**：`@<dsl_name> <Name> from "<path>"` — 复用 `from` 关键字引用外部文件，与 import 语法一致
- 示例：
  ```
  @prompt system ```
    You are #{role}.
  ```

  @component Button from "./button.tsx"
  ```
- `@` 是 DSL 块的唯一触发符，parser 看到顶层 `@` 即进入 DSL 解析，LL(1) 无歧义
- DSL 类型名（`prompt`、`component` 等）不需要是关键字，完全可扩展——`@sql`、`@graphql`、`@css` 开箱即用
- 内联块内容对宿主语言是**不透明的原始文本**，但支持 `#{expr}` **捕获**机制——从宿主作用域捕获表达式值到 DSL 世界
- `#{expr}` 的语义是**捕获**（capture）而非插值（interpolate）：DSL handler 拿到的是 `Capture(Expr)` 节点，可以选择立即求值、延迟求值、或编译为 reactive binding，取决于具体 DSL 的语义
- `#{expr}` 内只允许单一表达式，不允许语句，与 `${expr}` 模板字符串保持一致
- DSL 块在 AST 中表示为 `DslBlock { kind: str, name: str, content: DslContent }` 其中 `DslContent` 是 `Inline(parts)` 或 `FileRef(path)`
- **Lexer 层**：`@` 在顶层上下文中触发 DSL 模式；识别 `` ``` `` 起止，内部只识别 `#{` 捕获入口
- **Parser 层**：`@` → ident（dsl kind）→ ident（name）→ `` ``` `` 或 `from`，解析为 `DslBlock` AST 节点
- **后端可扩展**：codegen 阶段通过注册 `DslHandler` trait 来处理不同 kind 的 DSL 块
- 现有 spec 中的 prompt literal 将变为 DSL 框架的一个**内置 handler**
- **BREAKING**：prompt literal 语法从 `` let x = ``` ... ``` `` 改为 `` @prompt x ``` ... ``` ``

## Capabilities

### New Capabilities

- `dsl-lexing`: DSL 块的词法处理 — `@` 触发识别、三反引号 raw mode、`#{expr}` 捕获边界检测、`from` 文件引用
- `dsl-parsing`: DSL 块的语法分析 — `@ <kind> <name> (``` | from)` 解析为 `DslBlock` AST 节点
- `dsl-codegen-framework`: DSL 代码生成框架 — `DslHandler` trait、handler 注册机制、内置 prompt handler 作为参考实现

### Modified Capabilities

（无现有 spec）

## Impact

- **ag-lexer**：`@` 在顶层触发 DSL 扫描模式；新增三反引号 raw mode
- **ag-ast**：新增 `DslBlock`, `DslContent`, `DslPart` 节点
- **ag-parser**：新增 `parse_dsl_block` 方法，由顶层 `@` token 触发
- **ag-codegen**：新增 `DslHandler` trait 和 handler 注册表，内置 prompt handler
- **ag-checker**：DSL 块内的 `#{expr}` 捕获表达式需要类型检查
- **spec/lang.md**：prompt literal 语义更新为 DSL 框架子集
