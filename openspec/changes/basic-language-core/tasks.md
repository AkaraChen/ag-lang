## 1. Workspace 搭建

- [ ] 1.1 将根 `Cargo.toml` 改为 workspace 配置，`members = ["crates/*"]`，移除现有 `[package]` 段
- [ ] 1.2 创建 6 个 crate 目录及其 `Cargo.toml` + `src/lib.rs`（`ag-ast`, `ag-lexer`, `ag-parser`, `ag-checker`, `ag-codegen`）和 `ag-cli`（`src/main.rs`, `bin = "asc"`）
- [ ] 1.3 配置 crate 间依赖：ag-parser → ag-lexer + ag-ast, ag-checker → ag-ast, ag-codegen → ag-ast + swc crates, ag-cli → 全部
- [ ] 1.4 添加 SWC 依赖到 ag-codegen：`swc_common`, `swc_ecma_ast`, `swc_ecma_codegen`
- [ ] 1.5 `cargo build` 确认 workspace 编译通过

## 2. ag-ast：AST 节点定义

- [ ] 2.1 定义 `Span` 类型（`start: u32, end: u32`）
- [ ] 2.2 定义顶层结构 `Module { items: Vec<Item> }` 和 `Item` 枚举（FnDecl, StructDecl, EnumDecl, TypeAlias, Import, VarDecl, ExprStmt）
- [ ] 2.3 定义 `Expr` 枚举：Binary, Unary, Call, Member, Index, If, Match, Block, Ident, Literal, Array, Object, Arrow, Pipe, OptionalChain, NullishCoalesce, Await, ErrorPropagate
- [ ] 2.4 定义 `Stmt` 枚举：VarDecl, ExprStmt, Return, If, For, While, Match, TryCatch
- [ ] 2.5 定义 `TypeExpr` 枚举：Named, Array, Map, Nullable, Union, Function, Object
- [ ] 2.6 定义 `Pattern` 枚举：Literal, Ident, Struct, Enum, Wildcard, Range
- [ ] 2.7 定义辅助结构：FnDecl, StructDecl, EnumDecl, MatchArm, Param, Field, Variant, Block, BinaryOp, UnaryOp

## 3. ag-lexer：词法分析

- [ ] 3.1 定义 `TokenKind` 枚举（所有关键字、运算符、字面量类别、注释、EOF、Error）
- [ ] 3.2 定义 `Token { kind: TokenKind, span: Span, text: String }` 和 `Lexer` 结构体
- [ ] 3.3 实现核心扫描循环：跳过空白 → peek char → dispatch 到对应 lex 函数
- [ ] 3.4 实现标识符和关键字 lexing（先 lex 标识符，再 keyword lookup）
- [ ] 3.5 实现数字字面量 lexing（int, float, exponent notation）
- [ ] 3.6 实现字符串字面量 lexing（单引号、双引号、转义序列、未终止报错）
- [ ] 3.7 实现模板字符串 lexing（TemplateHead/Middle/Tail/NoSub，`${}` 嵌套用栈跟踪）
- [ ] 3.8 实现运算符 lexing（maximal munch：`==`, `!=`, `|>`, `?.`, `=>`, `->`, `::`, `..`, `...`, `**`, `&&`, `||`, `??`, `+=` 等）
- [ ] 3.9 实现注释 lexing（`//`, `/* */` 含嵌套, `///`）
- [ ] 3.10 实现错误恢复：遇到无法识别字符时产出 Error token 并继续
- [ ] 3.11 编写 lexer 单元测试：覆盖所有 spec 中的 scenario

## 4. ag-parser：语法分析

- [ ] 4.1 实现 `Parser` 结构体：token 流、cursor、diagnostics 收集、peek/advance/expect 工具方法
- [ ] 4.2 实现顶层 `parse_module`：循环解析 top-level items 直到 EOF
- [ ] 4.3 实现 `parse_import`：named imports 和 namespace imports
- [ ] 4.4 实现 `parse_var_decl`：let / mut / const + 可选类型标注 + 初始化表达式
- [ ] 4.5 实现 `parse_fn_decl`：pub? async? fn + 参数列表（含默认值）+ 返回类型 + block
- [ ] 4.6 实现 `parse_struct_decl`：struct Name { fields }
- [ ] 4.7 实现 `parse_enum_decl`：enum Name { Variant, Variant(fields), ... }
- [ ] 4.8 实现 `parse_type_alias`：type Name = Type
- [ ] 4.9 实现 `parse_type`：Named, Array `[T]`, Map `{K: V}`, Nullable `T?`, Union `A | B`, Function `(P) -> R`, Object `{ f: T }`
- [ ] 4.10 实现 Pratt expression parser 框架：`parse_expr(min_bp)` 配合优先级表
- [ ] 4.11 实现各优先级的中缀运算符解析（arithmetic, comparison, logical, assignment, pipe, `??`）
- [ ] 4.12 实现前缀运算符（`!`, `-`）和后缀运算符（`.`, `?.`, `::`, `()`, `[]`, `?`）
- [ ] 4.13 实现 primary 表达式：literals, ident, array `[...]`, object `{...}`, grouped `(expr)`, arrow function
- [ ] 4.14 实现 `parse_if`：if expr block (else (if | block))?，作为表达式可赋值
- [ ] 4.15 实现 `parse_for`：for ident in expr block
- [ ] 4.16 实现 `parse_while`：while expr block
- [ ] 4.17 实现 `parse_match`：match expr { arms }，每个 arm 含 pattern + 可选 guard + body
- [ ] 4.18 实现 `parse_pattern`：literal, ident, wildcard `_`, range `a..b`, struct `{ fields }`, enum `Enum::Variant(bindings)`
- [ ] 4.19 实现 `parse_block`：`{ stmts* expr? }`，最后一个无分号的表达式为隐式返回
- [ ] 4.20 实现 `parse_try_catch`：try block catch ident block
- [ ] 4.21 实现 `parse_ret`：ret expr?
- [ ] 4.22 实现错误恢复：同步到 `;`, `}`, 或顶层关键字，收集多个 diagnostic
- [ ] 4.23 编写 parser 单元测试：覆盖所有 spec scenario

## 5. ag-checker：类型检查

- [ ] 5.1 实现 `Scope` 和 `SymbolTable`：嵌套作用域链、符号注册和查找
- [ ] 5.2 实现 `Type` 内部表示：Primitive(str/num/int/bool/nil/any), Array, Map, Nullable, Union, Function, Struct, Enum, Unknown
- [ ] 5.3 实现 `type_compatible(expected, actual) -> bool`：基本兼容判断 + int→num 拓宽 + any 逃逸 + 结构化子类型 + 联合类型赋值
- [ ] 5.4 实现顶层 `check_module`：遍历 items，注册声明到符号表
- [ ] 5.5 实现 `check_fn_decl`：验证参数有类型标注、推断函数体、检查返回类型一致
- [ ] 5.6 实现 `check_var_decl`：从 RHS 推断类型或与标注对比
- [ ] 5.7 实现 `check_expr`：对每种表达式推断类型、检查运算符两侧类型兼容
- [ ] 5.8 实现 `check_call`：参数数量和类型匹配（考虑默认参数）
- [ ] 5.9 实现 `check_member_access`：struct 字段存在性检查
- [ ] 5.10 实现 `check_match`：各 arm pattern 类型窄化、arm body 类型统一
- [ ] 5.11 实现可变性检查：let/const 不可重赋值、mut 可以
- [ ] 5.12 编写 checker 单元测试：覆盖所有 spec scenario

## 6. ag-codegen：JS 代码生成（SWC）

- [ ] 6.1 实现 `Translator` 结构体：持有 `swc_common::SourceMap`，提供 `translate_module(&ag_ast::Module) -> swc_ecma_ast::Module`
- [ ] 6.2 实现变量声明翻译：let→const, mut→let, const→const，擦除类型标注
- [ ] 6.3 实现函数声明翻译：fn→function, pub→export, async→async, 参数擦除类型, 隐式返回包 return, 默认参数保留
- [ ] 6.4 实现箭头函数翻译
- [ ] 6.5 实现 struct/enum/type 声明擦除（不产出代码）
- [ ] 6.6 实现 enum variant 构造翻译：`Enum::Variant(...)` → `{ tag: "Variant", ... }`
- [ ] 6.7 实现表达式翻译：binary ops, unary ops, call, member access, index, array, object, template string
- [ ] 6.8 实现 pipe 翻译：`a |> f` → `f(a)`, `a |> f(_, x)` → `f(a, x)`
- [ ] 6.9 实现 `?.` 和 `??` 翻译（直接映射到 JS）
- [ ] 6.10 实现 `?` 错误传播翻译：生成 if-check early return 模式
- [ ] 6.11 实现 match 翻译：pattern → condition 的 if-else chain，binding → const 声明
- [ ] 6.12 实现控制流翻译：if/else, for-in→for-of, while, try-catch, ret→return
- [ ] 6.13 实现 import/export 翻译
- [ ] 6.14 实现 `emit` 函数：swc_ecma_ast::Module → JS 文本（用 `Emitter` + `JsWriter`）
- [ ] 6.15 编写 codegen 单元测试：覆盖所有 spec scenario

## 7. ag-cli：命令行入口

- [ ] 7.1 实现 `asc build <file.ag>` 命令：读取 .ag 文件 → lex → parse → check → codegen → 写 .js 文件
- [ ] 7.2 实现 `-o <output>` 参数指定输出路径
- [ ] 7.3 实现 `asc check <file.ag>` 命令：仅类型检查，不产出代码
- [ ] 7.4 实现错误输出格式：文件名、行号、列号、错误信息（从 Span 计算行列号）
- [ ] 7.5 编写端到端集成测试：.ag 输入 → .js 输出对比
