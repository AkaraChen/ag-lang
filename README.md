# AgentScript

> This project is under heavy construction.

**A programming language designed for AI agents.**

[中文版本](./README.CN.md)

---

## Why AgentScript?

Building AI agents today means wrestling with SDKs that were never meant for the job. You piece together prompt templates, tool definitions, and runtime glue — then debug the inevitable mismatches between them.

**AgentScript is built on a simple idea: AI agents deserve their own language.**

Not another framework. Not another SDK. A language where prompts, tools, agents, and servers are first-class citizens — not strings in a config file or objects in a class hierarchy.

## The Philosophy

### Prompts as Values

`@prompt` blocks make LLM messages real code — version-controlled, type-checked, composable. No more string interpolation nightmares. Captures like `#{user.name}` are expressions, not template variables.

### Tools with Guarantees

The `@tool` annotation turns functions into LLM-callable tools with auto-generated JSON schemas. The compiler checks that your parameters are serializable. No runtime surprises when the model calls your function with the wrong shape.

### Agents as Architecture

`@agent` blocks declare autonomous actors with model selection, tool bindings, and lifecycle hooks. Multi-agent teams compose naturally — no orchestration framework needed.

### Components as Output

`@component` blocks emit React components. Agents return rich UI, not just text. The language treats presentation as a first-class concern.

## What You Can Do

**Define Tools**
```ag
@tool("Search documentation")
fn search_docs(query: str, limit: int = 5) -> [Result] { ... }
```
The compiler generates the JSON schema. You write functions, not specifications.

**Author Prompts**
```ag
@prompt system <<EOF
@role system
You are a helpful assistant specializing in #{domain}.
EOF
```
Type-checked captures, directive parsing, template composition — all in one block.

**Build Agents**
```ag
@agent Coder <<EOF
@model claude-sonnet | gpt-4o
@tools #{[read_file, write_file, run_tests]}

You are an expert software engineer.
EOF
```
Model fallbacks, tool bindings, system prompts — declarative and version-controlled.

**Create UI**
```ag
@component DiffView <<EOF
@props { original: str, modified: str }
@render <Diff ... />
EOF
```
Agents that return components, not just text strings.

**Serve HTTP**
```ag
@server app <<EOF
@port 3000
@post /chat #{fn(c) { c.json(await agent.send(c.body)) }}
EOF
```
One block, running server.

## Who It's For

AgentScript is for developers building AI agents who:

- Are tired of string-based prompt engineering
- Want compile-time guarantees for tool schemas
- Build multi-agent systems and need architectural clarity
- Believe AI deserves better than Python SDKs designed for data science

## The Vision

A world where building an AI agent feels like writing a program — because it is one. Where prompts, tools, and agents are syntactic constructs with editor support, type systems, and compilation targets. Where the gap between "what I want the agent to do" and "code that makes it happen" is measured in lines, not frameworks.

---

*Compiles to Node.js. Written in Rust. Inspired by the belief that AI agents are a new kind of software, and they deserve a new kind of language.*