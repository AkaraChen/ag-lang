# AgentScript

> 本项目正在密集开发中。

**为 AI Agent 而生的编程语言。**

[English Version](./README.md)

---

## 为什么需要 AgentScript？

今天构建 AI Agent 意味着与从未为此设计的 SDK 搏斗。你拼凑提示词模板、工具定义、运行时胶水代码——然后调试它们之间不可避免的错位。

**AgentScript 建立在一个简单想法之上：AI Agent 值得拥有自己的语言。**

不是另一个框架，不是另一个 SDK。一种语言，其中 prompts、tools、agents 和 servers 是一等公民——不是配置文件中的字符串，也不是类层次中的对象。

## 核心理念

### Prompts 即值

`@prompt` 块让 LLM 消息成为真正的代码——版本控制、类型检查、可组合。不再有字符串拼接的噩梦。`#{user.name}` 这样的捕获是表达式，不是模板变量。

### 有保证的 Tools

`@tool` 注解将函数转变为 LLM 可调用的工具，自动生成 JSON Schema。编译器检查参数是否可序列化。当模型用错误的参数调用你的函数时，不会有运行时惊喜。

### Agents 即架构

`@agent` 块声明具有模型选择、工具绑定和生命周期钩子的自主执行者。多 Agent 团队自然组合——不需要编排框架。

### Components 即输出

`@component` 块生成 React 组件。Agent 返回富 UI，不只是文本。语言将展示视为一等关注点。

## 你可以做什么

**定义工具**
```ag
@tool("搜索文档")
fn search_docs(query: str, limit: int = 5) -> [Result] { ... }
```
编译器生成 JSON Schema。你写函数，不写规范。

**编写 Prompts**
```ag
@prompt system <<EOF
@role system
你是一个专注于 #{domain} 的助手。
EOF
```
类型检查的捕获、指令解析、模板组合——全在一个块里。

**构建 Agents**
```ag
@agent Coder <<EOF
@model claude-sonnet | gpt-4o
@tools #{[read_file, write_file, run_tests]}

你是一个专业的软件工程师。
EOF
```
模型回退、工具绑定、系统提示——声明式且版本控制。

**创建 UI**
```ag
@component DiffView <<EOF
@props { original: str, modified: str }
@render <Diff ... />
EOF
```
返回组件的 Agent，不只是文本字符串。

**提供 HTTP 服务**
```ag
@server app <<EOF
@port 3000
@post /chat #{fn(c) { c.json(await agent.send(c.body)) }}
EOF
```
一个块，运行中的服务器。

## 目标用户

AgentScript 适合这样的开发者：

- 厌倦了基于字符串的提示词工程
- 想要工具 Schema 的编译时保证
- 构建多 Agent 系统并需要架构清晰度
- 相信 AI 配得上比数据科学 Python SDK 更好的工具

## 愿景

一个构建 AI Agent 像写程序一样的世界——因为它就是程序。Prompts、tools、agents 是有编辑器支持、类型系统、编译目标的语法结构。"我想让 agent 做什么"和"实现它的代码"之间的差距，用行数衡量，而不是框架。

---

*编译到 Node.js。用 Rust 编写。源于一个信念：AI Agent 是一种新软件，它们值得一种新语言。*