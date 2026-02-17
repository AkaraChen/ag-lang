use ag_dsl_core::{DslError, DslPart};
use swc_common::{
    comments::{Comments, SingleThreadedComments},
    sync::Lrc,
    FileName, SourceMap,
};
use swc_ecma_ast as swc;
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};

// ── Public types ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ComponentMeta {
    pub name: String,
    pub description: Option<String>,
    pub props: Vec<ComponentProp>,
}

#[derive(Debug, Clone)]
pub struct ComponentProp {
    pub name: String,
    pub ty: String,
    pub description: Option<String>,
    pub has_default: bool,
}

// ── Main entry point ─────────────────────────────────────

/// Parse a `@component` DSL block into [`ComponentMeta`].
///
/// The `parts` slice should come from `DslContent::Inline`. Captures are
/// not supported and will produce an error.
pub fn parse_component(name: &str, parts: &[DslPart]) -> Result<ComponentMeta, DslError> {
    // 1. Concatenate text parts; reject captures.
    let mut source = String::new();
    for part in parts {
        match part {
            DslPart::Text(text, _span) => source.push_str(text),
            DslPart::Capture(_, span) => {
                return Err(DslError {
                    message: "captures are not supported in @component blocks".to_string(),
                    span: Some(*span),
                });
            }
        }
    }

    // 2. Parse with SWC.
    let cm: Lrc<SourceMap> = Default::default();
    let comments = SingleThreadedComments::default();
    let fm = cm.new_source_file(
        Lrc::new(FileName::Custom("component.jsx".into())),
        source,
    );
    let lexer = Lexer::new(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        swc_ecma_ast::EsVersion::Es2022,
        StringInput::from(&*fm),
        Some(&comments),
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().map_err(|e| {
        let msg = format!("{:?}", e);
        DslError {
            message: msg,
            span: None,
        }
    })?;

    // 3. Find export default declaration or expression.
    for item in &module.body {
        match item {
            swc::ModuleItem::ModuleDecl(swc::ModuleDecl::ExportDefaultDecl(export_decl)) => {
                if let swc::DefaultDecl::Fn(fn_expr) = &export_decl.decl {
                    let func_name = fn_expr
                        .ident
                        .as_ref()
                        .map(|id| id.sym.to_string())
                        .unwrap_or_else(|| name.to_string());

                    // Extract JSDoc from leading comments.
                    let (description, mut props) =
                        extract_jsdoc(&comments, export_decl.span.lo);

                    // Extract default values from params.
                    let defaults = extract_defaults_from_params(&fn_expr.function);
                    for prop in &mut props {
                        if defaults.contains(&prop.name) {
                            prop.has_default = true;
                        }
                    }

                    return Ok(ComponentMeta {
                        name: func_name,
                        description,
                        props,
                    });
                }
            }
            swc::ModuleItem::ModuleDecl(swc::ModuleDecl::ExportDefaultExpr(export_expr)) => {
                // Handle `export default () => ...` or `export default function(...)`
                match &*export_expr.expr {
                    swc::Expr::Arrow(arrow) => {
                        let defaults = extract_defaults_from_arrow(arrow);
                        let (description, mut props) =
                            extract_jsdoc(&comments, export_expr.span.lo);
                        for prop in &mut props {
                            if defaults.contains(&prop.name) {
                                prop.has_default = true;
                            }
                        }
                        return Ok(ComponentMeta {
                            name: name.to_string(),
                            description,
                            props,
                        });
                    }
                    swc::Expr::Fn(fn_expr) => {
                        let func_name = fn_expr
                            .ident
                            .as_ref()
                            .map(|id| id.sym.to_string())
                            .unwrap_or_else(|| name.to_string());
                        let (description, mut props) =
                            extract_jsdoc(&comments, export_expr.span.lo);
                        let defaults = extract_defaults_from_params(&fn_expr.function);
                        for prop in &mut props {
                            if defaults.contains(&prop.name) {
                                prop.has_default = true;
                            }
                        }
                        return Ok(ComponentMeta {
                            name: func_name,
                            description,
                            props,
                        });
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Err(DslError {
        message: "no `export default` function found in @component block".to_string(),
        span: None,
    })
}

// ── JSDoc extraction ─────────────────────────────────────

/// Pull the leading block comment for a given position and parse its JSDoc.
fn extract_jsdoc(
    comments: &SingleThreadedComments,
    pos: swc_common::BytePos,
) -> (Option<String>, Vec<ComponentProp>) {
    if let Some(leading) = comments.get_leading(pos) {
        for comment in leading.iter().rev() {
            if comment.kind == swc_common::comments::CommentKind::Block {
                return parse_jsdoc(&comment.text);
            }
        }
    }
    (None, Vec::new())
}

/// Parse the *inner text* of a `/** ... */` block comment into a description
/// and a list of `@param` entries.
fn parse_jsdoc(comment_text: &str) -> (Option<String>, Vec<ComponentProp>) {
    let mut description_lines: Vec<String> = Vec::new();
    let mut props: Vec<ComponentProp> = Vec::new();
    let mut seen_param = false;

    for raw_line in comment_text.lines() {
        // Strip the leading ` * ` decoration common in JSDoc.
        let line = raw_line
            .trim()
            .trim_start_matches('*')
            .trim();

        if line.starts_with("@param") {
            seen_param = true;
            if let Some(prop) = parse_param_line(line) {
                props.push(prop);
            }
        } else if !seen_param && !line.is_empty() {
            description_lines.push(line.to_string());
        }
    }

    let description = if description_lines.is_empty() {
        None
    } else {
        Some(description_lines.join(" "))
    };

    (description, props)
}

/// Parse a single `@param` line.
///
/// Supported formats:
/// - `@param {type} name - description`
/// - `@param {type} name`
/// - `@param name` (type defaults to "any")
fn parse_param_line(line: &str) -> Option<ComponentProp> {
    // Remove the `@param` prefix.
    let rest = line.strip_prefix("@param")?.trim();

    let (ty, after_type) = if rest.starts_with('{') {
        // Extract `{type}`.
        let close = rest.find('}')?;
        let raw_type = &rest[1..close];
        (map_jsdoc_type(raw_type.trim()), rest[close + 1..].trim())

    } else {
        ("any".to_string(), rest)
    };

    // Next token is the param name.
    let (name, desc_part) = match after_type.split_once(char::is_whitespace) {
        Some((n, d)) => (n.to_string(), d.trim().to_string()),
        None => (after_type.to_string(), String::new()),
    };

    if name.is_empty() {
        return None;
    }

    let description = if desc_part.is_empty() {
        None
    } else {
        // Strip optional leading `- `.
        let d = desc_part.strip_prefix("- ").unwrap_or(&desc_part);
        if d.is_empty() { None } else { Some(d.to_string()) }
    };

    Some(ComponentProp {
        name,
        ty,
        description,
        has_default: false,
    })
}

// ── Type mapping ─────────────────────────────────────────

fn map_jsdoc_type(jsdoc_type: &str) -> String {
    match jsdoc_type {
        "string" => "str".to_string(),
        "number" => "num".to_string(),
        "boolean" => "bool".to_string(),
        "object" => "any".to_string(),
        "*" => "any".to_string(),
        t if t.ends_with("[]") => {
            let inner = &t[..t.len() - 2];
            format!("[{}]", map_jsdoc_type(inner))
        }
        t if t.starts_with("Array<") && t.ends_with('>') => {
            let inner = &t[6..t.len() - 1];
            format!("[{}]", map_jsdoc_type(inner))
        }
        _ => "any".to_string(),
    }
}

// ── Default extraction ───────────────────────────────────

/// Walk function params looking for destructured props with default values.
fn extract_defaults_from_params(func: &swc::Function) -> Vec<String> {
    let mut defaults = Vec::new();
    for param in &func.params {
        extract_defaults_from_pat(&param.pat, &mut defaults);
    }
    defaults
}

/// Same as above but for arrow function params (which use `Pat` directly).
fn extract_defaults_from_arrow(arrow: &swc::ArrowExpr) -> Vec<String> {
    let mut defaults = Vec::new();
    for pat in &arrow.params {
        extract_defaults_from_pat(pat, &mut defaults);
    }
    defaults
}

fn extract_defaults_from_pat(pat: &swc::Pat, defaults: &mut Vec<String>) {
    if let swc::Pat::Object(obj_pat) = pat {
        for prop in &obj_pat.props {
            match prop {
                swc::ObjectPatProp::Assign(assign) => {
                    if assign.value.is_some() {
                        defaults.push(assign.key.sym.to_string());
                    }
                }
                _ => {}
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::Span;

    fn text(s: &str) -> DslPart {
        DslPart::Text(s.to_string(), Span::dummy())
    }

    fn capture() -> DslPart {
        DslPart::Capture(Box::new(0u32), Span::dummy())
    }

    #[test]
    fn simple_component_with_jsdoc() {
        let source = r#"
/**
 * A counter component.
 * @param {number} initial - Starting count
 * @param {string} label - Display label
 */
export default function Counter({ initial, label }) {
  return <div>{label}: {initial}</div>
}
"#;
        let parts = vec![text(source)];
        let meta = parse_component("Counter", &parts).expect("should parse");

        assert_eq!(meta.name, "Counter");
        assert_eq!(meta.description.as_deref(), Some("A counter component."));
        assert_eq!(meta.props.len(), 2);

        assert_eq!(meta.props[0].name, "initial");
        assert_eq!(meta.props[0].ty, "num");
        assert_eq!(
            meta.props[0].description.as_deref(),
            Some("Starting count")
        );
        assert!(!meta.props[0].has_default);

        assert_eq!(meta.props[1].name, "label");
        assert_eq!(meta.props[1].ty, "str");
        assert_eq!(
            meta.props[1].description.as_deref(),
            Some("Display label")
        );
        assert!(!meta.props[1].has_default);
    }

    #[test]
    fn component_no_jsdoc() {
        let source = r#"
export default function Foo() {
  return <div/>
}
"#;
        let parts = vec![text(source)];
        let meta = parse_component("Foo", &parts).expect("should parse");

        assert_eq!(meta.name, "Foo");
        assert!(meta.description.is_none());
        assert!(meta.props.is_empty());
    }

    #[test]
    fn component_with_defaults() {
        let source = r#"
export default function Foo({ x = 0, y = "hi" }) {
  return <div/>
}
"#;
        let parts = vec![text(source)];
        let meta = parse_component("Foo", &parts).expect("should parse");

        assert_eq!(meta.name, "Foo");
        assert_eq!(meta.props.len(), 0); // No JSDoc, so no props from JSDoc
        // Defaults are only marked on props that were declared via @param.
    }

    #[test]
    fn component_with_defaults_and_jsdoc() {
        let source = r#"
/**
 * @param {number} x - The x value
 * @param {string} y - The y value
 */
export default function Foo({ x = 0, y = "hi" }) {
  return <div/>
}
"#;
        let parts = vec![text(source)];
        let meta = parse_component("Foo", &parts).expect("should parse");

        assert_eq!(meta.name, "Foo");
        assert_eq!(meta.props.len(), 2);
        assert_eq!(meta.props[0].name, "x");
        assert!(meta.props[0].has_default);
        assert_eq!(meta.props[1].name, "y");
        assert!(meta.props[1].has_default);
    }

    #[test]
    fn missing_export_default() {
        let source = r#"
function Foo() {
  return <div/>
}
"#;
        let parts = vec![text(source)];
        let err = parse_component("Foo", &parts).unwrap_err();
        assert!(
            err.message.contains("export default"),
            "error should mention export default, got: {}",
            err.message
        );
    }

    #[test]
    fn capture_produces_error() {
        let parts = vec![text("export default function F() {}"), capture()];
        let err = parse_component("F", &parts).unwrap_err();
        assert!(
            err.message.contains("captures are not supported"),
            "unexpected error: {}",
            err.message
        );
    }

    #[test]
    fn invalid_jsx_syntax() {
        let source = "export default function { <<<<";
        let parts = vec![text(source)];
        let err = parse_component("Bad", &parts).unwrap_err();
        // Should be a parse error from SWC.
        assert!(
            !err.message.is_empty(),
            "expected a non-empty error message"
        );
    }

    #[test]
    fn type_mapping() {
        assert_eq!(map_jsdoc_type("string"), "str");
        assert_eq!(map_jsdoc_type("number"), "num");
        assert_eq!(map_jsdoc_type("boolean"), "bool");
        assert_eq!(map_jsdoc_type("object"), "any");
        assert_eq!(map_jsdoc_type("*"), "any");
        assert_eq!(map_jsdoc_type("string[]"), "[str]");
        assert_eq!(map_jsdoc_type("number[]"), "[num]");
        assert_eq!(map_jsdoc_type("Array<boolean>"), "[bool]");
        assert_eq!(map_jsdoc_type("SomeCustomType"), "any");
    }

    #[test]
    fn multi_text_parts() {
        let parts = vec![
            text("/**\n * Hello\n * @param {number} n - count\n */\n"),
            text("export default function Multi({ n }) {\n"),
            text("  return <span>{n}</span>\n"),
            text("}\n"),
        ];
        let meta = parse_component("Multi", &parts).expect("should parse");
        assert_eq!(meta.name, "Multi");
        assert_eq!(meta.props.len(), 1);
        assert_eq!(meta.props[0].name, "n");
        assert_eq!(meta.props[0].ty, "num");
    }
}
