## Context

从零构建 AgentScript 编译器。当前项目是空白 Rust 项目，需要建立 workspace 结构和完整编译管线。用户要求：
- 使用 `crates/` 目录拆包
- 源文件扩展名 `.ag`
- 用 SWC 做 JS 代码生成
- 先实现纯编程语言子集（不含 agent/tool/skill/component/prompt/http/JSX）

## Goals / Non-Goals

**Goals:**
- 端到端可工作：`asc build hello.ag` → `hello.js`
- 清晰的模块边界：lexer / ast / parser / checker / codegen / cli 各自独立 crate
- 覆盖 spec 中所有基础语言特性（变量、函数、类型、控制流、模式匹配、模块）
- 利用 SWC 的 AST + codegen 输出正确的 ESM JavaScript

**Non-Goals:**
- AI 原语（agent/tool/skill/component/prompt/http）— 后续 change
- LSP / IDE 支持
- 优化 pass
- Source map
- 增量编译
- WASM target

## Decisions

### 1. Workspace 结构

```
ag-lang/
├── Cargo.toml              # workspace root
├── crates/
│   ├── ag-ast/             # AST 节点定义
│   ├── ag-lexer/           # 词法分析
│   ├── ag-parser/          # 语法分析
│   ├── ag-checker/         # 类型检查
│   ├── ag-codegen/         # ag-ast → swc_ecma_ast → JS
│   └── ag-cli/             # CLI 入口 (bin: asc)
```

**依赖方向**（单向、无循环）：

```
ag-cli → ag-codegen → ag-checker → ag-parser → ag-lexer
                  ↘                      ↘         ↗
                   ag-ast ←←←←←←←←←←←←←←←
```

`ag-ast` 是共享基础包，被 parser/checker/codegen 依赖。lexer 只产出 token，不依赖 ast。

**理由**：每个阶段职责独立，可单独测试，未来可替换。比如 codegen 后续可加 WASM 后端，不影响前端。

### 2. Lexer 设计 (`ag-lexer`)

手写 lexer，逐字符扫描，零外部依赖。

- 输出：`Vec<Token>` 其中 `Token { kind: TokenKind, span: Span, text: &str }`
- `Span` = `{ start: u32, end: u32 }` — 字节偏移量
- 关键字通过查表区分（先匹配标识符，再在 keyword map 中查找）
- 模板字符串 `` `...${expr}...` `` 处理：lexer 产出 `TemplatePart` 序列，parser 组装
- 注释保留为 token（`LineComment` / `BlockComment` / `DocComment`），方便后续 doc 提取

**不处理 prompt literal（`` ``` ``）**——留给后续 change。

### 3. AST 设计 (`ag-ast`)

定义 AgentScript 自己的 AST，而非直接用 SWC 的 AST。

```rust
// 顶层
enum Item { FnDecl, StructDecl, EnumDecl, TypeAlias, Import, VarDecl }

// 表达式
enum Expr { Binary, Unary, Call, Member, Index, If, Match, Block,
            Ident, Literal, Array, Object, Arrow, Pipe, OptionalChain,
            NullishCoalesce, Try, Await }

// 语句
enum Stmt { Let, Mut, Const, Expr, Return, If, For, While, Match, TryCatch }

// 类型
enum Type { Named, Array, Map, Nullable, Union, Function, Object }

// 模式（match arm）
enum Pattern { Literal, Ident, Struct, Enum, Wildcard, Range }
```

每个节点携带 `Span` 用于错误报告。

**理由**：自己的 AST 可以精确表达 AgentScript 语义（如 `mut` vs `let`、`match` with guards、pipe operator），而 SWC AST 是 JS 语义。codegen 阶段做翻译。

### 4. Parser 设计 (`ag-parser`)

递归下降，LL(1)。

- 入口：`parse(tokens: &[Token]) -> Result<Module, Vec<Diagnostic>>`
- 表达式解析：Pratt parsing（优先级爬升），每个优先级一个函数
- 错误恢复：基于同步 token（`;`、`}`、关键字），收集多个错误而非首错即停
- 无 parser generator 依赖

**优先级表**（低到高）：

| 优先级 | 运算符 |
|--------|--------|
| 1 | `=` `+=` `-=` `*=` `/=` |
| 2 | `\|>` (pipe) |
| 3 | `??` |
| 4 | `\|\|` |
| 5 | `&&` |
| 6 | `==` `!=` |
| 7 | `<` `>` `<=` `>=` |
| 8 | `+` `-` |
| 9 | `*` `/` `%` |
| 10 | `**` |
| 11 | `!` `-` (unary) |
| 12 | `?.` `.` `::` `()` `[]` `?` |

### 5. Checker 设计 (`ag-checker`)

最小化类型检查器：

1. **符号表构建**：作用域链，每个 block 一层
2. **类型推断**：仅 `let` / `mut` / `const` 右侧推断，函数参数和返回类型必须标注
3. **结构化子类型**：`{a: str, b: int}` 满足 `{a: str}`
4. **联合窄化**：`match` 各 arm 内窄化到对应 variant
5. **函数调用检查**：参数数量 + 类型兼容性
6. **字段访问检查**：struct 字段是否存在

不做：泛型推断、生命周期、高阶类型、trait 约束。

输出：带类型注解的 AST（在节点上附加 `resolved_type` 字段），或 `Vec<Diagnostic>`。

### 6. Codegen 设计 (`ag-codegen`) — 使用 SWC

**核心思路**：ag-ast → swc_ecma_ast → swc_ecma_codegen emit

**依赖的 SWC crate**：
- `swc_common` — Span、SourceMap
- `swc_ecma_ast` — JS AST 节点
- `swc_ecma_codegen` — AST → JS 文本

**翻译规则**：

| AgentScript | JavaScript (SWC AST) |
|-------------|---------------------|
| `let x = 1` | `const x = 1` |
| `mut x = 1` | `let x = 1` |
| `const X = 1` | `const X = 1` |
| `fn foo(a: int) -> int { a + 1 }` | `function foo(a) { return a + 1; }` |
| `(x: int) => x * 2` | `(x) => x * 2` |
| `struct User { name: str }` | 类型擦除，不产出运行时代码 |
| `enum Status { A, B(x: int) }` | tagged union: `{ tag: "A" }` / `{ tag: "B", x: 1 }` |
| `match v { ... }` | if-else chain（pattern → condition） |
| `a \|> b` | `b(a)` |
| `a \|> f(_, x)` | `f(a, x)` |
| `x?.y` | `x?.y`（JS 原生支持） |
| `x ?? y` | `x ?? y`（JS 原生支持） |
| `x?` (error propagation) | early return pattern（待定，可能需要 runtime helper） |
| `import { x } from "y"` | `import { x } from "y"` |
| `pub fn` / `export` | `export function` |

**emit 流程**：

```rust
fn codegen(module: &ag_ast::Module) -> String {
    let swc_module = translate(module);  // ag-ast → swc_ecma_ast::Module
    let cm = Lrc::new(SourceMap::default());
    let mut buf = Vec::new();
    let mut emitter = Emitter {
        cfg: Default::default(),
        comments: None,
        cm: cm.clone(),
        wr: Box::new(JsWriter::new(cm, "\n", &mut buf, None)),
    };
    emitter.emit_module(&swc_module).unwrap();
    String::from_utf8(buf).unwrap()
}
```

**理由**：SWC 的 emitter 经过大量实战验证，输出的 JS 格式正确、可靠。自己拼字符串容易出 bug（括号、分号、转义）。代价是引入 SWC 依赖（编译稍慢），但值得。

### 7. CLI 设计 (`ag-cli`)

最小 CLI，binary name: `asc`。

```
asc build <file.ag>              # 编译单文件，输出 <file>.js
asc build <file.ag> -o out.js    # 指定输出路径
asc check <file.ag>              # 仅类型检查，不产出代码
```

流程：读文件 → lex → parse → check → codegen → 写文件。错误输出到 stderr，带 span 信息。

## Risks / Trade-offs

- **SWC 依赖体积**：SWC 的 crate 较大，会增加编译时间。→ 可接受，codegen 质量比编译速度重要；且仅 `ag-codegen` 依赖 SWC，不污染其他 crate。
- **SWC API 不稳定**：SWC 内部 API 偶有 breaking change。→ 锁定版本，codegen 层做适配隔离。
- **`?` 错误传播翻译**：JS 没有原生 `?` 操作符用于 Result。→ 第一版先生成 if-check early return 模式，不引入 runtime helper。
- **模板字符串嵌套**：`` `a ${`b ${c}`} d` `` 的 lexer 处理较复杂。→ lexer 用栈跟踪嵌套层级。
- **match exhaustiveness**：完整的穷尽检查较复杂。→ 第一版不做穷尽检查，仅做基本的 pattern-to-condition 翻译。
