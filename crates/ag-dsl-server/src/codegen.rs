use std::any::Any;

use ag_dsl_core::CodegenContext;
use ag_dsl_core::swc_helpers::{ident, str_lit, num_lit, expr_or_spread, make_prop};
use crate::ast::*;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

/// Serialize path segments to a Hono-compatible path string.
fn path_to_string(segments: &[PathSegment]) -> String {
    if segments.is_empty() {
        return "/".to_string();
    }
    let mut path = String::new();
    for seg in segments {
        path.push('/');
        match seg {
            PathSegment::Literal(s) => path.push_str(s),
            PathSegment::Param(name) => {
                path.push(':');
                path.push_str(name);
            }
            PathSegment::Wildcard => path.push('*'),
        }
    }
    path
}

fn method_to_str(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Delete => "delete",
        HttpMethod::Patch => "patch",
    }
}

/// Generate JS AST for a server template.
///
/// Produces:
/// ```js
/// import { Hono } from "hono";
/// import { serve } from "@agentscript/serve";
/// const <name> = new Hono();
/// <name>.use(<middleware>);
/// <name>.<method>("<path>", <handler>);
/// serve(<name>, { port: N, host: "..." });
/// ```
pub fn generate(
    template: &ServerTemplate,
    captures: &[&dyn Any],
    ctx: &mut dyn CodegenContext,
) -> Vec<swc::ModuleItem> {
    let mut items = Vec::new();

    // 1. import { Hono } from "hono"
    items.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(
        swc::ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
                span: DUMMY_SP,
                local: ident("Hono"),
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(swc::Str { span: DUMMY_SP, value: "hono".into(), raw: None }),
            type_only: false,
            with: None,
            phase: Default::default(),
        },
    )));

    // 2. import { serve } from "@agentscript/serve"
    items.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(
        swc::ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
                span: DUMMY_SP,
                local: ident("serve"),
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(swc::Str { span: DUMMY_SP, value: "@agentscript/serve".into(), raw: None }),
            type_only: false,
            with: None,
            phase: Default::default(),
        },
    )));

    // 3. const <name> = new Hono()
    let new_hono = swc::Expr::New(swc::NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Box::new(swc::Expr::Ident(ident("Hono"))),
        args: Some(vec![]),
        type_args: None,
    });

    items.push(swc::ModuleItem::Stmt(swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: swc::VarDeclKind::Const,
        declare: false,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name: swc::Pat::Ident(swc::BindingIdent { id: ident(&template.name), type_ann: None }),
            init: Some(Box::new(new_hono)),
            definite: false,
        }],
    })))));

    // 4. Middleware: <name>.use(<capture_expr>)
    for &mw_capture in &template.middlewares {
        if let Some(cap) = captures.get(mw_capture) {
            let handler_expr = ctx.translate_expr(*cap);
            let call = method_call(&template.name, "use", vec![expr_or_spread(handler_expr)]);
            items.push(expr_stmt(call));
        }
    }

    // 5. Routes: <name>.<method>("<path>", <handler_expr>)
    for route in &template.routes {
        if let Some(cap) = captures.get(route.handler_capture) {
            let handler_expr = ctx.translate_expr(*cap);
            let path_str = path_to_string(&route.path);
            let method = method_to_str(route.method);
            let call = method_call(
                &template.name,
                method,
                vec![
                    expr_or_spread(str_lit(&path_str)),
                    expr_or_spread(handler_expr),
                ],
            );
            items.push(expr_stmt(call));
        }
    }

    // 6. serve(<name>, { port?, host? })
    let mut serve_props = Vec::new();
    if let Some(port) = template.port {
        serve_props.push(make_prop("port", num_lit(port as f64)));
    }
    if let Some(ref host) = template.host {
        serve_props.push(make_prop("host", str_lit(host)));
    }

    let serve_call = swc::Expr::Call(swc::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: swc::Callee::Expr(Box::new(swc::Expr::Ident(ident("serve")))),
        args: vec![
            expr_or_spread(swc::Expr::Ident(ident(&template.name))),
            expr_or_spread(swc::Expr::Object(swc::ObjectLit {
                span: DUMMY_SP,
                props: serve_props,
            })),
        ],
        type_args: None,
    });
    items.push(expr_stmt(serve_call));

    items
}

fn method_call(obj_name: &str, method: &str, args: Vec<swc::ExprOrSpread>) -> swc::Expr {
    swc::Expr::Call(swc::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::Ident(ident(obj_name))),
            prop: swc::MemberProp::Ident(swc::IdentName {
                span: DUMMY_SP,
                sym: method.into(),
            }),
        }))),
        args,
        type_args: None,
    })
}

fn expr_stmt(expr: swc::Expr) -> swc::ModuleItem {
    swc::ModuleItem::Stmt(swc::Stmt::Expr(swc::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(expr),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::swc_helpers::emit_module;

    struct MockCtx;
    impl CodegenContext for MockCtx {
        fn translate_expr(&mut self, _expr: &dyn Any) -> swc::Expr {
            swc::Expr::Ident(ident("__handler__"))
        }
        fn translate_block(&mut self, _block: &dyn Any) -> Vec<swc::Stmt> { vec![] }
    }

    fn codegen_server(template: &ServerTemplate, n_captures: usize) -> String {
        let caps: Vec<Box<dyn Any>> = (0..n_captures).map(|i| Box::new(i as u32) as Box<dyn Any>).collect();
        let cap_refs: Vec<&dyn Any> = caps.iter().map(|c| c.as_ref()).collect();
        let mut ctx = MockCtx;
        let items = generate(template, &cap_refs, &mut ctx);
        emit_module(&items)
    }

    #[test]
    fn minimal_server() {
        let template = ServerTemplate {
            name: "app".into(),
            port: None,
            host: None,
            middlewares: vec![],
            routes: vec![Route {
                method: HttpMethod::Get,
                path: vec![],
                handler_capture: 0,
            }],
        };
        let js = codegen_server(&template, 1);
        assert!(js.contains("Hono"), "should import Hono");
        assert!(js.contains("serve"), "should import serve");
        assert!(js.contains("const app"), "should declare app");
        assert!(js.contains("app.get"), "should have GET route");
    }

    #[test]
    fn server_with_port_and_host() {
        let template = ServerTemplate {
            name: "api".into(),
            port: Some(3000),
            host: Some("0.0.0.0".into()),
            middlewares: vec![],
            routes: vec![],
        };
        let js = codegen_server(&template, 0);
        assert!(js.contains("3000"), "should have port");
        assert!(js.contains("0.0.0.0"), "should have host");
    }

    #[test]
    fn server_with_middleware() {
        let template = ServerTemplate {
            name: "app".into(),
            port: None,
            host: None,
            middlewares: vec![0],
            routes: vec![],
        };
        let js = codegen_server(&template, 1);
        assert!(js.contains("app.use"), "should have middleware");
    }

    #[test]
    fn all_http_methods() {
        let methods = [HttpMethod::Get, HttpMethod::Post, HttpMethod::Put, HttpMethod::Delete, HttpMethod::Patch];
        for (i, method) in methods.iter().enumerate() {
            let template = ServerTemplate {
                name: "app".into(),
                port: None,
                host: None,
                middlewares: vec![],
                routes: vec![Route {
                    method: *method,
                    path: vec![PathSegment::Literal("test".into())],
                    handler_capture: i,
                }],
            };
            let js = codegen_server(&template, i + 1);
            let method_str = method_to_str(*method);
            assert!(js.contains(&format!("app.{method_str}")), "should have {method_str} route");
        }
    }

    #[test]
    fn parameterized_path() {
        let template = ServerTemplate {
            name: "app".into(),
            port: None,
            host: None,
            middlewares: vec![],
            routes: vec![Route {
                method: HttpMethod::Get,
                path: vec![
                    PathSegment::Literal("users".into()),
                    PathSegment::Param("id".into()),
                ],
                handler_capture: 0,
            }],
        };
        let js = codegen_server(&template, 1);
        assert!(js.contains("/users/:id"), "should have parameterized path");
    }

    #[test]
    fn wildcard_path() {
        let template = ServerTemplate {
            name: "app".into(),
            port: None,
            host: None,
            middlewares: vec![],
            routes: vec![Route {
                method: HttpMethod::Get,
                path: vec![
                    PathSegment::Literal("files".into()),
                    PathSegment::Wildcard,
                ],
                handler_capture: 0,
            }],
        };
        let js = codegen_server(&template, 1);
        assert!(js.contains("/files/*"), "should have wildcard path");
    }
}
