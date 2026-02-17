## Context

基于 `extensible-dsl-system` change 建立的通用 DSL 框架（`@kind name ``` ... ````），本 change 实现第一个具体的 DSL handler：prompt DSL。

用户要求每个 DSL 是独立 crate，有完整的编译工具链（lexer → parser → AST → validator → codegen），可脱离 ag-lang 独立测试和使用。

## Goals / Non-Goals

**Goals:**
- prompt DSL 作为独立 crate `ag-dsl-prompt`，完全可独立测试
- 抽取 `ag-dsl-core` 作为所有 DSL 的共享基础
- 对标 LangChain.js 的 prompt 工程能力：角色、few-shot、消息占位、输出格式、模型选择、约束参数
- prompt 编译为结构化 JS 对象（非字符串拼接），支持 partial application 和 pipe 组合
- 编译产物依赖轻量 JS runtime `@agentscript/prompt-runtime`

**Non-Goals:**
- 不实现 LLM 调用逻辑（那是 runtime 的事）
- 不实现 example selector（动态示例选择）— 后续扩展
- 不实现 prompt caching / token counting — 后续扩展
- 不实现多模态 prompt（图片等）— 后续扩展

## Decisions

### 1. Crate 结构和依赖关系

```
crates/
├── ag-dsl-core/              # 共享 trait + 类型
│   └── src/lib.rs
├── ag-dsl-prompt/            # prompt DSL 完整实现
│   ├── src/
│   │   ├── lib.rs            # 公开 API
│   │   ├── lexer.rs          # prompt 内部语法 lexer
│   │   ├── ast.rs            # prompt AST 定义
│   │   ├── parser.rs         # prompt 内部语法 parser
│   │   ├── validator.rs      # 结构验证
│   │   ├── codegen.rs        # prompt AST → SWC JS AST
│   │   └── handler.rs        # impl DslHandler
│   └── tests/
│       ├── lexer_tests.rs
│       ├── parser_tests.rs
│       ├── codegen_tests.rs
│       └── integration_tests.rs
```

**依赖方向**：

```
ag-dsl-prompt → ag-dsl-core
ag-dsl-prompt → swc_ecma_ast, swc_ecma_codegen  (codegen 输出)
ag-codegen    → ag-dsl-core                       (handler trait)
ag-cli        → ag-dsl-prompt                     (注册 handler)
```

`ag-dsl-prompt` **不依赖** `ag-lexer`/`ag-parser`/`ag-ast`——它有自己的 lexer/parser，输入是 DSL 框架传来的 `Vec<DslPart>`（text + capture）。

### 2. ag-dsl-core 设计

从 `extensible-dsl-system` 的 design 中抽离出来：

```rust
// crates/ag-dsl-core/src/lib.rs

/// 所有 DSL handler 实现此 trait
pub trait DslHandler {
    fn handle(
        &self,
        block: &DslBlock,
        ctx: &mut dyn CodegenContext,
    ) -> Result<Vec<swc_ecma_ast::ModuleItem>, DslError>;
}

/// codegen 提供的表达式翻译能力
pub trait CodegenContext {
    fn translate_expr(&mut self, expr: &ag_ast::Expr) -> swc_ecma_ast::Expr;
}

/// DSL 框架传给 handler 的数据
pub struct DslBlock {
    pub kind: String,
    pub name: String,
    pub content: DslContent,
    pub span: Span,
}

pub enum DslContent {
    Inline { parts: Vec<DslPart> },
    FileRef { path: String, span: Span },
}

pub enum DslPart {
    Text(String, Span),
    Capture(Box<dyn std::any::Any>, Span),  // 类型擦除的宿主 AST 表达式
}

pub struct DslError {
    pub message: String,
    pub span: Option<Span>,
}
```

**关键设计**：`DslPart::Capture` 用 `Box<dyn Any>` 做类型擦除，使得 `ag-dsl-core` 不依赖 `ag-ast`。handler 在拿到 capture 后 downcast 到 `ag_ast::Expr`，或者通过 `CodegenContext::translate_expr` 直接翻译。

### 3. Prompt DSL 内部 Lexer

`ag-dsl-prompt` 的 lexer 接收 `Vec<DslPart>` 作为输入（不是原始源码）。它只需要解析 `DslPart::Text` 中的指令语法。

**指令识别规则**：在 Text 段内，行首的 `@` 后跟已知关键字（`role`/`model`/`examples`/`output`/`constraints`/`messages`）为指令开始。非指令行是正文文本。

**Token 类型**：

```rust
enum PromptToken {
    // 指令
    DirectiveRole(String),        // @role system
    DirectiveModel,               // @model
    DirectiveExamples,            // @examples
    DirectiveOutput,              // @output
    DirectiveConstraints,         // @constraints
    DirectiveMessages,            // @messages

    // 内容
    Text(String),                 // prompt 正文文本
    Capture(usize),               // 对 DslPart::Capture 的索引引用

    // 结构
    BraceOpen,                    // {
    BraceClose,                   // }
    Colon,                        // :
    Pipe,                         // | (model fallback 分隔)
    StringLiteral(String),        // "..."
    NumberLiteral(f64),           // 0.7
    ArrayOpen,                    // [
    ArrayClose,                   // ]
    Ident(String),                // 标识符
    Newline,                      // 段落分隔

    Eof,
}
```

**理由**：prompt 内部的指令语法很简单（类似 YAML/TOML），不需要完整的表达式 parser。lexer + 轻量 parser 就够了。

### 4. Prompt AST

```rust
/// 一个完整的 prompt 模板
struct PromptTemplate {
    name: String,
    sections: Vec<PromptSection>,
    model: Option<ModelSpec>,
    output: Option<OutputSpec>,
    constraints: Option<Constraints>,
}

enum PromptSection {
    /// @role <role_name> 后的内容段
    Role {
        role: RoleName,             // System, User, Assistant
        body: Vec<PromptPart>,      // text + captures 混合
    },
    /// @examples { ... }
    Examples(Vec<Example>),
    /// @messages #{expr}
    Messages {
        capture_index: usize,       // 引用的 capture 表达式
    },
}

enum PromptPart {
    Text(String),
    Capture(usize),                 // 索引到原始 DslPart::Capture
}

enum RoleName {
    System,
    User,
    Assistant,
    Custom(String),                 // 支持自定义 role
}

struct Example {
    pairs: Vec<(RoleName, String)>, // role → content 对
}

struct ModelSpec {
    models: Vec<String>,            // 按 fallback 顺序排列
}

struct OutputSpec {
    kind: OutputKind,
}

enum OutputKind {
    CaptureRef(usize),              // #{ResponseSchema} 引用宿主类型
    Inline(Vec<OutputField>),       // 内联 { field: type } 定义
}

struct OutputField {
    name: String,
    ty: String,                     // "str", "num", "[str]" 等
}

struct Constraints {
    fields: Vec<(String, ConstraintValue)>,
}

enum ConstraintValue {
    Number(f64),
    String(String),
    Array(Vec<ConstraintValue>),
    Bool(bool),
}
```

### 5. Prompt DSL Parser

接收 `Vec<PromptToken>` → 产出 `PromptTemplate`。

**解析策略**：逐 token 扫描，遇到指令 token 进入对应的指令解析器，遇到 Text/Capture 收集为当前 role section 的 body。

**状态机**：

```
初始状态 → 等待 @role 或 @model 等指令
@role X  → 进入角色段，收集后续 text/capture 直到下一个指令
@model   → 解析模型列表 (ident | ident | ident)
@examples → 解析 { role: "content", ... } 对象
@output  → 解析 capture ref 或内联 schema
@constraints → 解析 { key: value, ... } 对象
@messages → 解析 capture ref
```

**无 @role 前的文本**：如果 prompt 开头没有 `@role`，默认归入 `@role system`。

### 6. Codegen：Prompt AST → JavaScript

编译产物是一个结构化的 prompt 对象，依赖 `@agentscript/prompt-runtime`：

```javascript
// @prompt system_prompt ```
//   @role system
//   @model claude-sonnet | gpt-4o
//   You are #{role}, an expert in #{domain}.
//   @examples { user: "hello" assistant: "hi there" }
//   @constraints { temperature: 0.7 }
// ```
//
// 编译为：

import { PromptTemplate } from "@agentscript/prompt-runtime";

const system_prompt = new PromptTemplate({
  model: ["claude-sonnet", "gpt-4o"],
  messages: [
    {
      role: "system",
      content: (ctx) => `You are ${ctx.role}, an expert in ${ctx.domain}.`,
    },
  ],
  examples: [
    { role: "user", content: "hello" },
    { role: "assistant", content: "hi there" },
  ],
  constraints: {
    temperature: 0.7,
  },
});
```

**关键点**：
- 含 capture 的 message content 编译为**箭头函数** `(ctx) => ...`，实现延迟求值
- 无 capture 的 content 编译为**纯字符串**
- `@messages` 编译为 `messagesPlaceholder: "history"` 配置项
- `@output` 编译为 `outputSchema: { ... }` JSON Schema 对象
- `prompt.with()` 由 runtime 的 `PromptTemplate.with()` 方法实现

### 7. Runtime 接口设计（JS 侧）

`@agentscript/prompt-runtime` 提供的核心 API：

```typescript
class PromptTemplate {
  constructor(config: PromptConfig);

  // 部分填充 capture 变量
  with(vars: Record<string, any>): PromptTemplate;

  // 构建最终的 message 序列
  format(vars: Record<string, any>): Message[];

  // 访问器
  get messages(): MessageTemplate[];
  get model(): string[];
  get schema(): object | null;
  get constraints(): Record<string, any>;
}

interface Message {
  role: "system" | "user" | "assistant" | string;
  content: string;
}

interface PromptConfig {
  model?: string[];
  messages: MessageTemplate[];
  examples?: Message[];
  messagesPlaceholder?: string;
  outputSchema?: object;
  constraints?: Record<string, any>;
}
```

本 change 只定义 runtime 接口，不实现完整的 LLM 调用——那是 agent runtime 的职责。

### 8. 独立测试策略

`ag-dsl-prompt` 可完全独立测试：

```rust
#[test]
fn test_prompt_parse_and_codegen() {
    let parts = vec![
        DslPart::Text("@role system\nYou are ".into(), span(0, 28)),
        DslPart::Capture(/* mock expr */, span(28, 34)),
        DslPart::Text(".\n@constraints {\n  temperature: 0.7\n}".into(), span(34, 72)),
    ];

    let tokens = prompt_lexer::lex(&parts);
    let ast = prompt_parser::parse(&tokens).unwrap();
    let js = prompt_codegen::generate(&ast, &mut mock_context());

    assert!(js.contains("role: \"system\""));
    assert!(js.contains("temperature: 0.7"));
}
```

不需要 `.ag` 文件、不需要宿主 lexer/parser、不需要完整编译管线。

## Risks / Trade-offs

- **ag-dsl-core 的 `Box<dyn Any>` 类型擦除**：handler 需要 downcast，失去编译时类型安全 → 可接受，因为 handler 和宿主编译器在同一进程，downcast 失败是 bug 而非用户错误。未来可用泛型改善。
- **Runtime 依赖**：编译产物依赖 `@agentscript/prompt-runtime` npm 包 → 需要同步维护 Rust codegen 和 JS runtime。第一版 runtime 尽量轻量（< 200 行），降低维护负担。
- **指令语法和 Markdown 冲突**：prompt 内容中 `@role` 可能被误认为指令 → 规则明确：只有**行首** `@` + **已知关键字**才是指令，其他 `@` 是正文文本。
- **capture 在 content 函数中的绑定**：`(ctx) => \`...\${ctx.role}...\`` 需要 capture 变量名映射到 `ctx` 属性名 → codegen 需要从宿主 AST 提取变量名。如果 capture 是复杂表达式（如 `#{a + b}`），生成为 `ctx.__capture_0` 之类的合成名称。
- **@output 的类型引用**：`@output #{ResponseSchema}` 需要在 codegen 时将 AgentScript 类型转为 JSON Schema → 需要 `CodegenContext` 提供类型→schema 翻译能力。第一版可简化：只支持内联 schema 定义，capture ref 后续支持。
