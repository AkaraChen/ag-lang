## Why

AgentScript 需要一个可工作的编译器基础。在实现 `agent`/`tool`/`skill`/`component`/`prompt`/`http` 等 AI 原语之前，必须先有一个能处理基本编程结构的前端（lexer → parser → type checker → codegen）。这是整个语言的地基。

## What Changes

- 新建 Rust workspace，在 `crates/` 下按职责拆包：
  - `ag-lexer` — 词法分析，将 `.ag` 源码转为 token 流
  - `ag-parser` — 递归下降 LL(1) parser，生成 AST
  - `ag-ast` — AST 节点定义，供 parser/checker/codegen 共享
  - `ag-checker` — 基础类型检查（结构化类型、局部推断）
  - `ag-codegen` — 生成 JavaScript (ESM)
  - `ag-cli` — 命令行入口 `asc`
- 源文件扩展名：`.ag`
- 覆盖 spec 中的纯编程语言子集，**不包含** agent/tool/skill/component/prompt literals/http server/JSX
- 具体语言特性：
  - 变量声明：`let`（不可变）、`mut`（可变）、`const`（编译期常量）
  - 函数：`fn`、箭头函数 `=>`、默认参数、隐式返回
  - 类型系统：`str`/`num`/`int`/`bool`/`nil`/`any`、数组 `[T]`、map `{K: V}`、可空 `T?`、联合 `A | B`、函数类型
  - 结构体 `struct`、枚举 `enum`（含关联数据）、类型别名 `type`
  - 控制流：`if`/`else`、`for..in`、`while`、`match`（模式匹配含 guard）
  - 表达式：管道 `|>`、可选链 `?.`、空值合并 `??`、`?` 错误传播
  - 错误处理：`try`/`catch`、`Result` 风格返回
  - 注释：`//`、`/* */`、`///` doc comment
  - 字符串：单引号、双引号、模板字符串 `` `hello ${name}` ``
  - 模块：`import`/`export`/`from`/`as`/`pub`

## Capabilities

### New Capabilities

- `lexer`: 词法分析 — 将 `.ag` 源码切分为 token（关键字、运算符、字面量、标识符、注释）
- `parser`: 语法分析 — 递归下降 parser，产出 AST；支持上述所有语法结构
- `type-system`: 类型系统 — 原始类型、复合类型、结构化子类型、局部推断、联合窄化
- `codegen-js`: 代码生成 — 从 AST 输出 JavaScript ESM；struct → object、enum → tagged union、match → if/switch

### Modified Capabilities

（无现有 spec）

## Impact

- **项目结构**：从单 crate 改为 workspace + `crates/` 多包
- **Cargo.toml**：根 workspace 配置 + 每个 crate 独立 Cargo.toml
- **入口**：`src/main.rs` 迁移到 `crates/ag-cli/src/main.rs`
- **依赖**：暂无外部依赖，全部手写（lexer/parser 不使用 parser generator）
- **输出**：`asc build foo.ag` 产出 `.js` 文件
