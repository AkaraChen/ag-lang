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
