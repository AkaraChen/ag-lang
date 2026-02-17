use std::process::Command;

fn asc_binary() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_asc"));
    cmd
}

fn build_ag(source: &str) -> (String, String, i32) {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.ag");
    let output = dir.path().join("test.js");
    std::fs::write(&input, source).unwrap();

    let result = asc_binary()
        .args(["build", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let js = std::fs::read_to_string(&output).unwrap_or_default();
    let code = result.status.code().unwrap_or(-1);

    if code == 0 {
        (js, stderr, code)
    } else {
        (String::new(), stderr, code)
    }
}

fn check_ag(source: &str) -> (String, i32) {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.ag");
    std::fs::write(&input, source).unwrap();

    let result = asc_binary()
        .args(["check", input.to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let code = result.status.code().unwrap_or(-1);
    (stderr, code)
}

// ── Build command tests ──

#[test]
fn build_variable_declarations() {
    let (js, _, code) = build_ag(r#"
let x: num = 42
mut y: str = "hello"
const PI: num = 3.14
"#);
    assert_eq!(code, 0);
    assert!(js.contains("const x = 42;"));
    assert!(js.contains("let y = \"hello\";"));
    assert!(js.contains("const PI = 3.14;"));
}

#[test]
fn build_function_declaration() {
    let (js, _, code) = build_ag(r#"
fn add(a: num, b: num) -> num {
    a + b
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("function add(a, b)"));
    assert!(js.contains("return a + b;"));
}

#[test]
fn build_pub_export() {
    let (js, _, code) = build_ag(r#"
pub fn greet(name: str) -> str {
    "hi"
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("export function greet(name)"));
}

#[test]
fn build_async_function() {
    let (js, _, code) = build_ag(r#"
async fn fetch_data(url: str) -> str {
    "data"
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("async function fetch_data(url)"));
}

#[test]
fn build_if_expression() {
    let (js, _, code) = build_ag(r#"
fn check(x: num) -> str {
    let result = if x > 0 { "positive" } else { "non-positive" }
    result
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("if") || js.contains("?"));
}

#[test]
fn build_for_loop() {
    let (js, _, code) = build_ag(r#"
fn process(items: [num]) {
    for item in items {
        let x = item
    }
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("for"));
    assert!(js.contains("of"));
}

#[test]
fn build_match_expression() {
    let (js, _, code) = build_ag(r#"
fn describe(x: num) -> str {
    match x {
        0 => "zero",
        1 => "one",
        _ => "other",
    }
}
"#);
    assert_eq!(code, 0);
    // match compiles to if-else chain
    assert!(js.contains("if"));
}

#[test]
fn build_pipe_operator() {
    let (js, _, code) = build_ag(r#"
fn double(x: num) -> num { x * 2 }
fn main() {
    let result = 5 |> double
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("double(5)"));
}

#[test]
fn build_optional_chaining_and_nullish() {
    let (js, _, code) = build_ag(r#"
fn test(x: any) -> any {
    let a = x?.name
    let b = x ?? "default"
    a
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("?."));
    assert!(js.contains("??"));
}

#[test]
fn build_struct_erased() {
    let (js, _, code) = build_ag(r#"
struct Point {
    x: num,
    y: num,
}

fn make_point() -> Point {
    { x: 1, y: 2 }
}
"#);
    assert_eq!(code, 0);
    // struct declaration should be erased
    assert!(!js.contains("struct"));
    assert!(js.contains("function make_point"));
}

#[test]
fn build_arrow_function() {
    let (js, _, code) = build_ag(r#"
fn main() {
    let add = (a: num, b: num) => a + b
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("=>"));
}

// ── Check command tests ──

#[test]
fn check_valid_program() {
    let (stderr, code) = check_ag(r#"
fn add(a: num, b: num) -> num {
    a + b
}
"#);
    assert_eq!(code, 0);
    assert!(stderr.contains("ok"));
}

#[test]
fn check_type_mismatch() {
    let (stderr, code) = check_ag(r#"
let x: num = "not a number"
"#);
    assert_ne!(code, 0);
    assert!(stderr.contains("error"));
    assert!(stderr.contains("type mismatch") || stderr.contains("expected"));
}

#[test]
fn check_undefined_variable() {
    let (stderr, code) = check_ag(r#"
fn main() {
    let x = undefined_var
}
"#);
    assert_ne!(code, 0);
    assert!(stderr.contains("error"));
}

// ── Error output format test ──

#[test]
fn error_shows_file_line_col() {
    let (stderr, code) = check_ag(r#"
let x: num = "bad"
"#);
    assert_ne!(code, 0);
    // Should show file:line:col format
    assert!(stderr.contains("test.ag:"));
    assert!(stderr.contains("error:"));
}

// ── CLI usage tests ──

#[test]
fn no_args_shows_usage() {
    let result = asc_binary().output().unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_ne!(result.status.code().unwrap(), 0);
    assert!(stderr.contains("Usage:") || stderr.contains("usage:"));
}

#[test]
fn unknown_command_errors() {
    let result = asc_binary().args(["unknown"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_ne!(result.status.code().unwrap(), 0);
    assert!(stderr.contains("Unknown command") || stderr.contains("unknown"));
}

#[test]
fn build_nonexistent_file_errors() {
    let result = asc_binary()
        .args(["build", "/tmp/nonexistent_ag_file.ag"])
        .output()
        .unwrap();
    assert_ne!(result.status.code().unwrap(), 0);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("error") || stderr.contains("cannot read"));
}

// ── Default output path test ──

#[test]
fn build_default_output_path() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("hello.ag");
    std::fs::write(&input, "let x: num = 1\n").unwrap();

    let result = asc_binary()
        .args(["build", input.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(result.status.code().unwrap(), 0);
    let expected_output = dir.path().join("hello.js");
    assert!(expected_output.exists(), "Expected hello.js to be created at default path");
}

// ── DSL e2e tests ──

#[test]
fn build_dsl_prompt_inline_with_capture() {
    let (js, _, code) = build_ag(
        "let role: str = \"admin\"\n@prompt system ```\n@role system\nYou are #{role}.\n```\n",
    );
    assert_eq!(code, 0);
    assert!(js.contains("const system"));
    assert!(js.contains("PromptTemplate"));
    assert!(js.contains("ctx.role"));
}

#[test]
fn build_dsl_prompt_from_file() {
    let (js, _, code) = build_ag(
        r#"@prompt system from "./system-prompt.txt""#,
    );
    assert_eq!(code, 0);
    assert!(js.contains("const system"));
    assert!(js.contains("PromptTemplate"));
    assert!(js.contains("readFile"));
}

#[test]
fn check_dsl_capture_undefined_var_error() {
    let (stderr, code) = check_ag("@prompt sys ```\n#{undefined_var}\n```\n");
    assert_ne!(code, 0);
    assert!(stderr.contains("error"));
}

#[test]
fn build_dsl_prompt_full_template() {
    let (js, _, code) = build_ag(r#"
@prompt chat ```
@model claude-sonnet | gpt-4o
@role system
You are a helpful assistant.
@examples {
  user: "hello"
  assistant: "hi there"
}
@constraints {
  temperature: 0.7
  max_tokens: 4096
}
```
"#);
    assert_eq!(code, 0);
    assert!(js.contains("PromptTemplate"));
    assert!(js.contains("claude-sonnet"));
    assert!(js.contains("gpt-4o"));
    assert!(js.contains("system"));
    assert!(js.contains("helpful assistant"));
    assert!(js.contains("examples"));
    assert!(js.contains("hello"));
    assert!(js.contains("constraints"));
    assert!(js.contains("temperature"));
    assert!(js.contains("0.7"));
}

#[test]
fn build_dsl_prompt_with_output_schema() {
    let (js, _, code) = build_ag(r#"
@prompt qa ```
@role system
Answer the question.
@output {
  answer: str
  confidence: num
}
```
"#);
    assert_eq!(code, 0);
    assert!(js.contains("outputSchema"));
    assert!(js.contains("string"));
    assert!(js.contains("number"));
}

// ── Extern / stdlib / JS interop e2e tests ──

#[test]
fn build_extern_fn_with_js_annotation() {
    let (js, _, code) = build_ag(r#"
@js("node:fs/promises")
extern fn readFile(path: str) -> Promise<str>

async fn main() {
    let content = await readFile("test.txt")
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains(r#"import { readFile } from "node:fs/promises""#));
    assert!(js.contains("await readFile"));
}

#[test]
fn build_extern_fn_erased() {
    let (js, _, code) = build_ag(r#"
extern fn console_log(msg: str)
"#);
    assert_eq!(code, 0);
    // extern fn should not generate any JS function definition
    assert!(!js.contains("function console_log"));
}

#[test]
fn build_extern_struct_erased() {
    let (js, _, code) = build_ag(r#"
extern struct MyClass {
    name: str,
    fn hello() -> str
}
"#);
    assert_eq!(code, 0);
    assert!(js.trim().is_empty());
}

#[test]
fn build_extern_type_erased() {
    let (js, _, code) = build_ag(r#"
extern type Headers
"#);
    assert_eq!(code, 0);
    assert!(js.trim().is_empty());
}

#[test]
fn build_js_import_merged() {
    let (js, _, code) = build_ag(r#"
@js("node:fs/promises")
extern fn readFile(path: str) -> Promise<str>

@js("node:fs/promises")
extern fn writeFile(path: str, data: str) -> Promise<nil>

async fn main() {
    let data = await readFile("in.txt")
    await writeFile("out.txt", data)
}
"#);
    assert_eq!(code, 0);
    // Both should be merged into a single import
    assert!(js.contains("readFile"));
    assert!(js.contains("writeFile"));
    assert!(js.contains(r#"from "node:fs/promises""#));
}

#[test]
fn build_js_import_aliased() {
    let (js, _, code) = build_ag(r#"
@js("my-lib", name = "doWork")
extern fn do_work(input: str) -> str

fn main() {
    let result = do_work("test")
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("doWork"));
    assert!(js.contains(r#"from "my-lib""#));
}

#[test]
fn build_unreferenced_js_extern_no_import() {
    let (js, _, code) = build_ag(r#"
@js("some-module")
extern fn unused_fn(x: str) -> str

let x: int = 42
"#);
    assert_eq!(code, 0);
    // unused extern should not generate import
    assert!(!js.contains("some-module"));
}

#[test]
fn build_extern_fn_no_js_no_import() {
    let (js, _, code) = build_ag(r#"
extern fn fetch(url: str) -> Promise<any>

async fn main() {
    let resp = await fetch("https://example.com")
}
"#);
    assert_eq!(code, 0);
    // No @js annotation = global API, no import
    assert!(!js.contains("import"));
    assert!(js.contains("fetch"));
}

#[test]
fn build_promise_type_and_async_await() {
    let (js, _, code) = build_ag(r#"
extern fn fetch(url: str) -> Promise<any>

async fn load_data(url: str) -> any {
    let resp = await fetch(url)
    resp
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("async function load_data"));
    assert!(js.contains("await fetch"));
}

#[test]
fn check_await_outside_async_error() {
    let (stderr, code) = check_ag(r#"
extern fn fetch(url: str) -> Promise<any>

fn main() {
    let resp = await fetch("url")
}
"#);
    assert_ne!(code, 0);
    assert!(stderr.contains("await") && stderr.contains("async"));
}

#[test]
fn build_std_web_fetch_import() {
    let (js, _, code) = build_ag(r#"
import { fetch, Response } from "std:web/fetch"

async fn load(url: str) -> any {
    let resp = await fetch(url)
    resp
}
"#);
    assert_eq!(code, 0);
    // web/fetch is Layer A (global API), no import in JS output
    assert!(!js.contains(r#"from "std:"#));
    assert!(js.contains("fetch"));
}

#[test]
fn build_std_log_import() {
    let (js, _, code) = build_ag(r#"
import { info } from "std:log"

fn main() {
    info("hello")
}
"#);
    assert_eq!(code, 0);
    // Layer B: should generate @agentscript/stdlib import
    assert!(js.contains(r#"@agentscript/stdlib/log"#));
    assert!(js.contains("info"));
}

#[test]
fn check_unknown_std_module_error() {
    let (stderr, code) = check_ag(r#"
import { foo } from "std:nonexistent"
"#);
    assert_ne!(code, 0);
    assert!(stderr.contains("unknown") || stderr.contains("nonexistent"));
}

// ── http-stdlib tests ──

#[test]
fn build_http_server_import() {
    let (js, _, code) = build_ag(r#"
import { createApp, Context } from "std:http/server"
fn main() {
    let app = createApp()
    app.get("/", fn(c: Context) -> Response { c.text("hello") })
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains(r#"import { App as createApp } from "@agentscript/stdlib/http/server""#));
    assert!(js.contains("createApp()"));
}

#[test]
fn build_http_server_full_app() {
    let (js, _, code) = build_ag(r#"
import { createApp, Context } from "std:http/server"

fn handler(c: Context) -> Response {
    c.json({ message: "hello" })
}

fn main() {
    let app = createApp()
    app.get("/", handler)
    app.post("/api", fn(c: Context) -> Response { c.text("ok") })
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains(r#"@agentscript/stdlib/http/server"#));
    assert!(js.contains("createApp()"));
    assert!(js.contains(r#"app.get("/"#));
    assert!(js.contains(r#"app.post("/"#));
}

#[test]
fn build_http_client_import() {
    let (js, _, code) = build_ag(r#"
import { get, post } from "std:http/client"
async fn main() {
    let resp = await get("https://api.example.com/data")
    await post("https://api.example.com/send", { body: { key: "value" } })
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains(r#"@agentscript/stdlib/http/client"#));
    assert!(js.contains("get("));
    assert!(js.contains("post("));
}

#[test]
fn build_http_server_client_mixed() {
    let (js, _, code) = build_ag(r#"
import { createApp, Context } from "std:http/server"
import { get } from "std:http/client"

async fn handler(c: Context) -> Response {
    let resp = await get("https://api.example.com/data")
    let data = await resp.json()
    c.json(data)
}

fn main() {
    let app = createApp()
    app.get("/proxy", handler)
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains(r#"@agentscript/stdlib/http/server"#));
    assert!(js.contains(r#"@agentscript/stdlib/http/client"#));
}

#[test]
fn build_fn_expression() {
    let (js, _, code) = build_ag(r#"
fn apply(f: (int) -> int, x: int) -> int {
    f(x)
}
fn main() {
    apply(fn(x: int) -> int { x + 1 }, 42)
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("(x)=>{"));
}

#[test]
fn build_async_fn_expression() {
    let (js, _, code) = build_ag(r#"
fn run(f: () -> any) -> any {
    f()
}
fn main() {
    run(async fn() -> str { "hello" })
}
"#);
    assert_eq!(code, 0);
    assert!(js.contains("async ()=>{"));
}

#[test]
fn build_example_http_server() {
    let example_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/http-server/app.ag");

    let output = tempfile::NamedTempFile::new().unwrap();
    let result = asc_binary()
        .args([
            "build",
            example_path.to_str().unwrap(),
            "-o",
            output.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let code = result.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(code, 0, "example compilation failed: {}", stderr);

    let js = std::fs::read_to_string(output.path()).unwrap();
    // Check correct imports
    assert!(js.contains(r#"@agentscript/stdlib/http/server"#));
    // Check functions exist
    assert!(js.contains("function add("));
    assert!(js.contains("function subtract("));
    assert!(js.contains("function calculate("));
    assert!(js.contains("export function setup("));
    // Check routes
    assert!(js.contains(r#"app.get("/"#));
    assert!(js.contains(r#"app.post("/echo"#));
    assert!(js.contains(r#"app.post("/calc"#));
    assert!(js.contains(r#"app.get("/greet/:name"#));
    // Check async handlers
    assert!(js.contains("async (c)=>{"));
    assert!(js.contains("await c.req.json()"));
}
