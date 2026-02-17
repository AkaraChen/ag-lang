## Why

AgentScript 的 prompt DSL 是连接语言和 LLM 的核心界面。LangChain.js 提供了 PromptTemplate、ChatPromptTemplate、FewShotPromptTemplate、MessagesPlaceholder、OutputParser 等功能，但都是运行时库，没有编译时检查。AgentScript 要在语言层面提供同等甚至更强的 prompt 工程能力——结构化的角色声明、类型安全的变量捕获、编译时可检查的输出 schema、可组合的 prompt 值——同时作为独立 crate 实现，可脱离 ag-lang 单独测试和使用。

## What Changes

### 架构：独立 DSL crate 体系

- 新建 `crates/ag-dsl-core/`：定义所有 DSL 共享的 trait 和类型
  - `DslHandler` trait（从 ag-codegen 设计中抽离）
  - `DslBlock`, `DslContent`, `DslPart` 共享类型
  - `CodegenContext` trait（抽象的表达式翻译接口）
- 新建 `crates/ag-dsl-prompt/`：prompt DSL 的完整实现
  - 自有 lexer：解析 prompt DSL 内部语法（`@role`, `@model`, `@examples`, `@output`, `@constraints`, `@messages` 等指令）
  - 自有 AST：`PromptTemplate`, `RoleSection`, `Example`, `OutputSpec`, `Constraints` 等
  - 自有 validator：验证 prompt 结构合法性（如 @output 引用的类型是否存在）
  - 自有 codegen：prompt AST → SWC JS AST（生成结构化的 prompt 对象/template literal）
  - 实现 `DslHandler` trait
  - **完全可独立测试**：给一段 prompt DSL 文本 + capture 表达式列表，输出 JS 代码

### Prompt DSL 语法特性

以下指令在 `@prompt <name> ``` ... ``` ` 块内使用：

**角色声明 `@role`**
```
@role system
You are a helpful assistant.

@role user
#{user_input}
```
一个 prompt 可以包含多个 `@role` 段，形成 chat message 序列。

**模型选择 `@model`**
```
@model claude-sonnet | gpt-4o | deepseek-chat
```
`|` 分隔表示 fallback 顺序。编译为模型列表。

**Few-shot 示例 `@examples`**
```
@examples {
  user: "Fix this bug in my code"
  assistant: "I'll analyze the code and suggest a fix..."
}
@examples {
  user: "Explain recursion"
  assistant: "Recursion is when a function calls itself..."
}
```
多个 `@examples` 块按序拼接为示例序列。每个 example 是 role→content 对。

**消息占位 `@messages`**
```
@messages #{conversation_history}
```
注入动态消息列表（对话历史），capture 表达式必须求值为 `[Message]` 类型。

**输出格式 `@output`**
```
@output #{ResponseSchema}
```
引用 AgentScript struct/enum 类型，编译为 JSON Schema 作为 structured output 约束。也支持内联定义：
```
@output {
  answer: str
  confidence: num
  sources: [str]
}
```

**约束参数 `@constraints`**
```
@constraints {
  temperature: 0.7
  max_tokens: 4096
  top_p: 0.9
  stop: ["\n\n"]
}
```

**模板文本和捕获**
- 指令之外的所有文本是 prompt 正文
- `#{expr}` 从宿主作用域捕获值
- 文本内的 `#` 不跟 `{` 则是普通字符

### Prompt 作为一等值

编译后的 prompt 是一个**结构化对象**，而非字符串。支持：
- `prompt.with({ key: value })` — 部分填充 capture，返回新 prompt
- `prompt |> complete` — 发送给 LLM 获取结果
- `prompt |> stream` — 流式输出
- `prompt.messages` — 访问编译后的 message 序列
- `prompt.schema` — 访问 output schema

## Capabilities

### New Capabilities

- `dsl-core`: DSL 共享核心 — `DslHandler` trait、`CodegenContext` trait、共享类型定义，供所有 DSL crate 依赖
- `prompt-dsl-parser`: prompt DSL 内部语法解析 — 独立 lexer/parser，识别 `@role`/`@model`/`@examples`/`@output`/`@constraints`/`@messages` 指令，产出 prompt AST
- `prompt-dsl-codegen`: prompt DSL 代码生成 — prompt AST → JS 结构化 prompt 对象，实现 `DslHandler` trait

### Modified Capabilities

（无现有 spec）

## Impact

- **新增 crate**：`ag-dsl-core`（共享）、`ag-dsl-prompt`（prompt DSL 完整实现）
- **ag-codegen 重构**：`DslHandler` trait 和 `CodegenContext` 抽离到 `ag-dsl-core`，ag-codegen 依赖 ag-dsl-core
- **ag-cli**：注册 `ag-dsl-prompt` 的 handler
- **运行时**：编译产物依赖一个轻量 prompt runtime（构造 message 对象、调用 LLM API），设计为 `@agentscript/prompt-runtime` npm 包
- **Cargo workspace**：新增两个 crate member
