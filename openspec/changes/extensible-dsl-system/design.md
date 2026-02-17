## Context

`basic-language-core` change 建立了基础编译管线（lexer → parser → checker → codegen）。本 change 在其之上增加 DSL 嵌入能力，使 AgentScript 的 AI 原语（prompt、component、agent 等）能够嵌入异质语法的内容块。这是从"基础编程语言"走向"agent-first 语言"的桥梁。

当前 lexer 已处理常规 token（标识符、运算符、字符串、模板字符串）。需要扩展它来识别 `@` 触发的 DSL 块。

## Goals / Non-Goals

**Goals:**
- `@` 作为 DSL 块的唯一触发符，LL(1) 无歧义
- DSL 类型名不需要是关键字，完全可扩展
- `#{expr}` 作为唯一的宿主语言→DSL 捕获通道，语义由 handler 决定
- `from "<path>"` 作为文件引用语法，与 import 一致
- codegen 层可扩展：通过 `DslHandler` trait 注册不同 DSL 的处理器
- 内置一个 prompt handler 作为参考实现和验证

**Non-Goals:**
- 不实现具体的 component/skill/agent DSL handler（后续 change）
- 不做 DSL 内部的语法检查（那是各 handler 的职责）
- 不做 DSL 块的 LSP 支持（语法高亮、自动补全）
- 不做 `from` 文件的 watch/hot-reload

## Decisions

### 1. `@` 作为 DSL 触发符

**语法**：

```
// 内联块
@prompt system ```
  You are #{role}.
  @constraints { temperature: 0.7 }
```

// 文件引用
@component Button from "./button.tsx"

// 完全可扩展
@sql get_users ```
  SELECT * FROM users WHERE role = #{role}
```
```

**Grammar**:

```ebnf
dsl_decl    = "@" IDENT IDENT dsl_body ;
dsl_body    = "```" dsl_content "```"     (* 内联块 *)
            | "from" STRING               (* 文件引用 *)
            ;
dsl_content = (DSL_TEXT | "#{" expr "}")* ;
```

**Parser 触发**：顶层遇到 `@` → 进入 `parse_dsl_block()`。LL(1) 一步确定。

**替代方案**：`<keyword> <Name> ``` ``` ` 无 `@` 前缀。
**放弃原因**：需要三步前瞻，且 DSL 类型名必须是关键字或预注册列表，不可扩展。

**`@` 与 decorator 的区分**：当前 spec 中 `@` 也用于 decorator/annotation。区分规则：
- 顶层 `@ <ident> <Ident> (``` | from)` → DSL 块（三 token 前瞻确认）
- `@ <ident>` 后面跟 `fn`/`struct` 等声明 → decorator（后续实现）
- 实践中不会冲突，因为 DSL 块的 `@kind Name ``` ` 模式非常独特

### 2. Lexer 中 DSL 块的扫描策略

**决策**：lexer 正常输出 `@` token。parser 在顶层看到 `@` 后，判断是否为 DSL 块（前瞻 `<ident> <ident> (``` | from)`），如果是，则通知 lexer 进入"raw mode"扫描三反引号内容。

**Token 输出序列**（内联块）：

```
At                     // @
Ident("prompt")        // DSL kind
Ident("system")        // DSL name
DslBlockStart          // 开始的 ```
DslText("...")         // 原始文本片段
DslCaptureStart        // #{
<normal tokens>        // AgentScript 表达式 tokens
DslCaptureEnd          // }
DslText("...")         // 更多原始文本
DslBlockEnd            // 结束的 ```
```

**Token 输出序列**（文件引用）：

```
At                     // @
Ident("component")     // DSL kind
Ident("Button")        // DSL name
From                   // from 关键字
StringLiteral("./button.tsx")  // 路径作为字符串字面量
```

**Raw mode 规则**：
- 进入：parser 确认 DSL 块后，调用 `lexer.enter_dsl_raw_mode()`
- 扫描：逐字符收集文本，遇到 `#{` 切回正常模式 lex 表达式，遇到 `}` 切回 raw mode
- 退出：行首（可有前导空白）遇到 `` ``` `` 输出 `DslBlockEnd`

### 3. AST 表示

```rust
// ag-ast 中新增

/// DSL 块节点
struct DslBlock {
    kind: String,           // "prompt", "component", "sql", ...
    name: Ident,            // 块名称
    content: DslContent,    // 内联或文件引用
    span: Span,
}

enum DslContent {
    /// 内联三反引号块
    Inline {
        parts: Vec<DslPart>,
    },
    /// from "<path>" 文件引用
    FileRef {
        path: String,
        span: Span,
    },
}

enum DslPart {
    /// 原始文本片段
    Text(String, Span),
    /// 从宿主作用域捕获的表达式
    Capture(Box<Expr>, Span),
}
```

`DslBlock` 作为新的 `Item` 变体加入顶层 AST。

### 4. Capture 语义和类型检查

**决策**：checker 对 `DslPart::Capture(expr)` 内的表达式做正常类型推断，但**不约束其类型**——因为 checker 不知道具体 DSL 期望什么类型。推断的类型信息附加到 AST 上，供 handler 在 codegen 阶段使用。

**替代方案**：让 handler 声明 capture 的期望类型，checker 对照检查。
**暂不采用**：增加复杂度，且第一版 handler 在 codegen 阶段才运行。将来可以扩展。

### 5. DslHandler trait 和 codegen 集成

```rust
// ag-codegen 中新增

trait DslHandler {
    /// 处理一个 DSL 块，返回 SWC AST 节点
    fn handle(
        &self,
        block: &DslBlock,
        ctx: &mut CodegenContext,
    ) -> Result<Vec<swc_ecma_ast::ModuleItem>, CodegenError>;
}

struct CodegenContext {
    /// 翻译一个 AgentScript 表达式为 SWC 表达式
    fn translate_expr(&mut self, expr: &ag_ast::Expr) -> swc_ecma_ast::Expr;
}
```

handler 注册表在 `Translator` 中：

```rust
struct Translator {
    handlers: HashMap<String, Box<dyn DslHandler>>,
    // ...
}
```

codegen 遍历 AST，遇到 `DslBlock` 时查找 `handlers[block.kind]`：
- 找到 → 调用 handler
- 未找到 → 报错 "no handler registered for DSL kind `xxx`"

**内置 prompt handler**：第一版将 DSL 内容翻译为 JS 模板字符串，capture 编译为 `${}` 插值：

```javascript
// @prompt system ```
//   You are #{role}.
// ```
// →
const system = `You are ${role}.`;
```

后续支持 `@role`/`@constraints` 等 prompt 指令时再引入结构化 runtime。

### 6. FileRef 处理

**决策**：`from "<path>"` 在 codegen 阶段处理。handler 收到 `DslContent::FileRef(path)` 时：
1. 解析路径（相对于当前 `.ag` 文件目录）
2. 读取文件内容
3. 按 handler 逻辑处理（如 component handler 可能直接 import 该 .tsx 文件）

第一版 FileRef **不支持** `#{expr}` capture——文件内容作为纯文本或直接 import。

### 7. `@` 在 DSL 内容中的含义

注意：`@` 在 DSL 块**内部**不由宿主语言处理。上面示例中 prompt 内的 `@constraints` 是 DSL 内部语法，由 prompt handler 解析，不是宿主语言的 `@` token。

宿主 lexer 在 raw mode 中只识别 `#{` 和 `` ``` ``，其他一切（包括 `@`）都是 `DslText`。

## Risks / Trade-offs

- **三反引号歧义**：DSL 块内出现 `` ``` `` 会导致误截断 → 采用"行首 `` ``` `` 才是结束"的规则。未来可支持自定义 fence（如 ``````）。
- **`@` 与 decorator 冲突**：`@foo bar ``` ``` ` 是 DSL 块，`@foo fn bar()` 是 decorator → 通过三 token 前瞻区分。如果用户定义了 `@cache MyData ``` ``` ` 这种 DSL，且同时有 `@cache` decorator，会有歧义 → 实践中极不可能，且 DSL 块要求 name 后跟 `` ``` ``/`from`，decorator 不会。
- **FileRef 无 capture**：`from` 引用的文件不能用 `#{expr}` → 可接受，第一版简化。将来可以扩展为编译时读取文件后再解析 capture。
- **DSL 内容无语法检查**：宿主编译器不验证 DSL 内容正确性 → handler 负责，这是 by design。
