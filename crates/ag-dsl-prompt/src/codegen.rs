use std::any::Any;

use ag_dsl_core::CodegenContext;
use crate::ast::*;
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use swc_common::SyntaxContext;

/// Generate JS AST for a prompt template.
///
/// `captures` are the original DslPart::Capture values (type-erased expressions).
/// `ctx` is used to translate capture expressions to JS.
pub fn generate(
    template: &PromptTemplate,
    captures: &[&dyn Any],
    ctx: &mut dyn CodegenContext,
) -> Vec<swc::ModuleItem> {
    let mut items = Vec::new();

    // 1. Import statement: import { PromptTemplate } from "@agentscript/prompt-runtime"
    items.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(
        swc::ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
                span: DUMMY_SP,
                local: ident("PromptTemplate"),
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(swc::Str {
                span: DUMMY_SP,
                value: "@agentscript/prompt-runtime".into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        },
    )));

    // 2. Build the config object properties
    let mut config_props: Vec<swc::PropOrSpread> = Vec::new();

    // model property
    if let Some(ref model) = template.model {
        config_props.push(make_prop(
            "model",
            swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: model
                    .models
                    .iter()
                    .map(|m| Some(expr_or_spread(str_lit(m))))
                    .collect(),
            }),
        ));
    }

    // messages property
    let mut messages: Vec<Option<swc::ExprOrSpread>> = Vec::new();
    let mut messages_placeholder: Option<String> = None;

    for section in &template.sections {
        match section {
            PromptSection::Role { role, body } => {
                let role_str = role.as_str();
                let content_expr = build_content_expr(body, captures, ctx);

                let msg = swc::Expr::Object(swc::ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        make_prop("role", str_lit(role_str)),
                        make_prop("content", content_expr),
                    ],
                });
                messages.push(Some(expr_or_spread(msg)));
            }
            PromptSection::Examples(_) => {
                // Examples go to a separate property, not messages
            }
            PromptSection::Messages { capture_index } => {
                // Extract capture variable name for messagesPlaceholder
                if let Some(cap) = captures.get(*capture_index) {
                    let js_expr = ctx.translate_expr(*cap);
                    if let swc::Expr::Ident(id) = &js_expr {
                        messages_placeholder = Some(id.sym.to_string());
                    } else {
                        messages_placeholder = Some(format!("__capture_{}", capture_index));
                    }
                }
            }
        }
    }

    config_props.push(make_prop(
        "messages",
        swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems: messages,
        }),
    ));

    // examples property
    let example_sections: Vec<&Vec<Example>> = template
        .sections
        .iter()
        .filter_map(|s| {
            if let PromptSection::Examples(ex) = s {
                Some(ex)
            } else {
                None
            }
        })
        .collect();

    if !example_sections.is_empty() {
        let mut example_elems: Vec<Option<swc::ExprOrSpread>> = Vec::new();
        for examples in &example_sections {
            for example in *examples {
                for (role, content) in &example.pairs {
                    let obj = swc::Expr::Object(swc::ObjectLit {
                        span: DUMMY_SP,
                        props: vec![
                            make_prop("role", str_lit(role.as_str())),
                            make_prop("content", str_lit(content)),
                        ],
                    });
                    example_elems.push(Some(expr_or_spread(obj)));
                }
            }
        }
        config_props.push(make_prop(
            "examples",
            swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: example_elems,
            }),
        ));
    }

    // messagesPlaceholder property
    if let Some(ref placeholder) = messages_placeholder {
        config_props.push(make_prop("messagesPlaceholder", str_lit(placeholder)));
    }

    // outputSchema property
    if let Some(ref output) = template.output {
        let schema_expr = build_output_schema(&output.kind, captures, ctx);
        config_props.push(make_prop("outputSchema", schema_expr));
    }

    // constraints property
    if let Some(ref constraints) = template.constraints {
        let constraints_expr = build_constraints_expr(constraints);
        config_props.push(make_prop("constraints", constraints_expr));
    }

    // 3. const <name> = new PromptTemplate({ ... })
    let config_obj = swc::Expr::Object(swc::ObjectLit {
        span: DUMMY_SP,
        props: config_props,
    });

    let new_expr = swc::Expr::New(swc::NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Box::new(swc::Expr::Ident(ident("PromptTemplate"))),
        args: Some(vec![expr_or_spread(config_obj)]),
        type_args: None,
    });

    let decl = swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: swc::VarDeclKind::Const,
        declare: false,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name: swc::Pat::Ident(swc::BindingIdent {
                id: ident(&template.name),
                type_ann: None,
            }),
            init: Some(Box::new(new_expr)),
            definite: false,
        }],
    })));

    items.push(swc::ModuleItem::Stmt(decl));
    items
}

/// Build a FileRef codegen: const <name> = new PromptTemplate({ messages: [{ role: "system", content: await readFile("<path>", "utf-8") }] })
pub fn generate_file_ref(name: &str, path: &str) -> Vec<swc::ModuleItem> {
    let mut items = Vec::new();

    // Import
    items.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(
        swc::ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
                span: DUMMY_SP,
                local: ident("PromptTemplate"),
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(swc::Str {
                span: DUMMY_SP,
                value: "@agentscript/prompt-runtime".into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        },
    )));

    // readFile call
    let read_call = swc::Expr::Call(swc::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::Ident(ident("fs"))),
            prop: swc::MemberProp::Ident(swc::IdentName {
                span: DUMMY_SP,
                sym: "readFile".into(),
            }),
        }))),
        args: vec![
            expr_or_spread(str_lit(path)),
            expr_or_spread(str_lit("utf-8")),
        ],
        type_args: None,
    });

    let await_expr = swc::Expr::Await(swc::AwaitExpr {
        span: DUMMY_SP,
        arg: Box::new(read_call),
    });

    let msg = swc::Expr::Object(swc::ObjectLit {
        span: DUMMY_SP,
        props: vec![
            make_prop("role", str_lit("system")),
            make_prop("content", await_expr),
        ],
    });

    let config = swc::Expr::Object(swc::ObjectLit {
        span: DUMMY_SP,
        props: vec![make_prop(
            "messages",
            swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: vec![Some(expr_or_spread(msg))],
            }),
        )],
    });

    let new_expr = swc::Expr::New(swc::NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Box::new(swc::Expr::Ident(ident("PromptTemplate"))),
        args: Some(vec![expr_or_spread(config)]),
        type_args: None,
    });

    let decl = swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: swc::VarDeclKind::Const,
        declare: false,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name: swc::Pat::Ident(swc::BindingIdent {
                id: ident(name),
                type_ann: None,
            }),
            init: Some(Box::new(new_expr)),
            definite: false,
        }],
    })));

    items.push(swc::ModuleItem::Stmt(decl));
    items
}

// ── Content expression builders ──────────────────────────────

fn build_content_expr(
    body: &[PromptPart],
    captures: &[&dyn Any],
    ctx: &mut dyn CodegenContext,
) -> swc::Expr {
    let has_captures = body.iter().any(|p| matches!(p, PromptPart::Capture(_)));

    if !has_captures {
        // Pure text → string literal
        let text: String = body
            .iter()
            .map(|p| match p {
                PromptPart::Text(s) => s.as_str(),
                _ => "",
            })
            .collect();
        return str_lit(text.trim_end());
    }

    // Has captures → (ctx) => `...${ctx.var}...` template literal
    let mut quasis = Vec::new();
    let mut exprs: Vec<Box<swc::Expr>> = Vec::new();
    let mut current_text = String::new();

    for part in body {
        match part {
            PromptPart::Text(s) => {
                current_text.push_str(s);
            }
            PromptPart::Capture(idx) => {
                quasis.push(swc::TplElement {
                    span: DUMMY_SP,
                    tail: false,
                    cooked: Some(current_text.clone().into()),
                    raw: current_text.clone().into(),
                });
                current_text.clear();

                // Translate the capture expression
                let capture_expr = if let Some(cap) = captures.get(*idx) {
                    let js_expr = ctx.translate_expr(*cap);
                    // Wrap in ctx.name if simple ident, else ctx.__capture_N
                    match &js_expr {
                        swc::Expr::Ident(id) => {
                            // ctx.varname
                            swc::Expr::Member(swc::MemberExpr {
                                span: DUMMY_SP,
                                obj: Box::new(swc::Expr::Ident(ident("ctx"))),
                                prop: swc::MemberProp::Ident(swc::IdentName {
                                    span: DUMMY_SP,
                                    sym: id.sym.clone(),
                                }),
                            })
                        }
                        _ => {
                            // ctx.__capture_N
                            swc::Expr::Member(swc::MemberExpr {
                                span: DUMMY_SP,
                                obj: Box::new(swc::Expr::Ident(ident("ctx"))),
                                prop: swc::MemberProp::Ident(swc::IdentName {
                                    span: DUMMY_SP,
                                    sym: format!("__capture_{}", idx).into(),
                                }),
                            })
                        }
                    }
                } else {
                    // Fallback
                    swc::Expr::Member(swc::MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(swc::Expr::Ident(ident("ctx"))),
                        prop: swc::MemberProp::Ident(swc::IdentName {
                            span: DUMMY_SP,
                            sym: format!("__capture_{}", idx).into(),
                        }),
                    })
                };

                exprs.push(Box::new(capture_expr));
            }
        }
    }

    // Tail quasis
    let trimmed = current_text.trim_end().to_string();
    quasis.push(swc::TplElement {
        span: DUMMY_SP,
        tail: true,
        cooked: Some(trimmed.clone().into()),
        raw: trimmed.into(),
    });

    let tpl = swc::Expr::Tpl(swc::Tpl {
        span: DUMMY_SP,
        exprs,
        quasis,
    });

    // (ctx) => `...`
    swc::Expr::Arrow(swc::ArrowExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        params: vec![swc::Pat::Ident(swc::BindingIdent {
            id: ident("ctx"),
            type_ann: None,
        })],
        body: Box::new(swc::BlockStmtOrExpr::Expr(Box::new(tpl))),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
    })
}

fn build_output_schema(
    kind: &OutputKind,
    captures: &[&dyn Any],
    ctx: &mut dyn CodegenContext,
) -> swc::Expr {
    match kind {
        OutputKind::CaptureRef(idx) => {
            if let Some(cap) = captures.get(*idx) {
                ctx.translate_expr(*cap)
            } else {
                swc::Expr::Ident(ident("undefined"))
            }
        }
        OutputKind::Inline(fields) => {
            // Build JSON Schema-like object
            let mut properties: Vec<swc::PropOrSpread> = Vec::new();
            let mut required: Vec<Option<swc::ExprOrSpread>> = Vec::new();

            for field in fields {
                let type_schema = ag_type_to_json_schema(&field.ty);
                properties.push(make_prop(&field.name, type_schema));
                required.push(Some(expr_or_spread(str_lit(&field.name))));
            }

            swc::Expr::Object(swc::ObjectLit {
                span: DUMMY_SP,
                props: vec![
                    make_prop("type", str_lit("object")),
                    make_prop(
                        "properties",
                        swc::Expr::Object(swc::ObjectLit {
                            span: DUMMY_SP,
                            props: properties,
                        }),
                    ),
                    make_prop(
                        "required",
                        swc::Expr::Array(swc::ArrayLit {
                            span: DUMMY_SP,
                            elems: required,
                        }),
                    ),
                ],
            })
        }
    }
}

fn ag_type_to_json_schema(ty: &str) -> swc::Expr {
    if ty.starts_with('[') && ty.ends_with(']') {
        let inner = &ty[1..ty.len() - 1];
        swc::Expr::Object(swc::ObjectLit {
            span: DUMMY_SP,
            props: vec![
                make_prop("type", str_lit("array")),
                make_prop("items", ag_type_to_json_schema(inner)),
            ],
        })
    } else {
        let json_type = match ty {
            "str" => "string",
            "num" => "number",
            "int" => "integer",
            "bool" => "boolean",
            _ => "string",
        };
        swc::Expr::Object(swc::ObjectLit {
            span: DUMMY_SP,
            props: vec![make_prop("type", str_lit(json_type))],
        })
    }
}

fn build_constraints_expr(constraints: &Constraints) -> swc::Expr {
    let props: Vec<swc::PropOrSpread> = constraints
        .fields
        .iter()
        .map(|(key, value)| make_prop(key, constraint_value_to_expr(value)))
        .collect();

    swc::Expr::Object(swc::ObjectLit {
        span: DUMMY_SP,
        props,
    })
}

fn constraint_value_to_expr(value: &ConstraintValue) -> swc::Expr {
    match value {
        ConstraintValue::Number(n) => swc::Expr::Lit(swc::Lit::Num(swc::Number {
            span: DUMMY_SP,
            value: *n,
            raw: None,
        })),
        ConstraintValue::String(s) => str_lit(s),
        ConstraintValue::Bool(b) => swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
            span: DUMMY_SP,
            value: *b,
        })),
        ConstraintValue::Array(items) => swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems: items
                .iter()
                .map(|v| Some(expr_or_spread(constraint_value_to_expr(v))))
                .collect(),
        }),
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn ident(name: &str) -> swc::Ident {
    swc::Ident {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        sym: name.into(),
        optional: false,
    }
}

fn str_lit(s: &str) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Str(swc::Str {
        span: DUMMY_SP,
        value: s.into(),
        raw: None,
    }))
}

fn expr_or_spread(expr: swc::Expr) -> swc::ExprOrSpread {
    swc::ExprOrSpread {
        spread: None,
        expr: Box::new(expr),
    }
}

fn make_prop(key: &str, value: swc::Expr) -> swc::PropOrSpread {
    swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
        key: swc::PropName::Ident(swc::IdentName {
            span: DUMMY_SP,
            sym: key.into(),
        }),
        value: Box::new(value),
    })))
}

// ── Emit helper for tests ────────────────────────────────────

pub fn emit_module(items: &[swc::ModuleItem]) -> String {
    use swc_common::sync::Lrc;
    use swc_common::SourceMap;
    use swc_ecma_codegen::text_writer::JsWriter;
    use swc_ecma_codegen::Emitter;

    let module = swc::Module {
        span: DUMMY_SP,
        body: items.to_vec(),
        shebang: None,
    };

    let cm: Lrc<SourceMap> = Lrc::new(SourceMap::default());
    let mut buf = Vec::new();
    {
        let mut emitter = Emitter {
            cfg: swc_ecma_codegen::Config::default(),
            cm: cm.clone(),
            comments: None,
            wr: JsWriter::new(cm, "\n", &mut buf, None),
        };
        emitter.emit_module(&module).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockContext;

    impl CodegenContext for MockContext {
        fn translate_expr(&mut self, _expr: &dyn Any) -> swc::Expr {
            // Return a simple ident for testing
            swc::Expr::Ident(ident("mockVar"))
        }
        fn translate_block(&mut self, _block: &dyn Any) -> Vec<swc::Stmt> {
            Vec::new()
        }
    }

    #[test]
    fn codegen_simple_prompt() {
        let tpl = PromptTemplate {
            name: "greeting".to_string(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Hello!".to_string())],
            }],
            model: None,
            output: None,
            constraints: None,
        };

        let items = generate(&tpl, &[], &mut MockContext);
        let js = emit_module(&items);
        assert!(js.contains("PromptTemplate"));
        assert!(js.contains("greeting"));
        assert!(js.contains("system"));
        assert!(js.contains("Hello!"));
    }

    #[test]
    fn codegen_with_model() {
        let tpl = PromptTemplate {
            name: "test".to_string(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Hi".to_string())],
            }],
            model: Some(ModelSpec {
                models: vec!["claude-sonnet".to_string(), "gpt-4o".to_string()],
            }),
            output: None,
            constraints: None,
        };

        let items = generate(&tpl, &[], &mut MockContext);
        let js = emit_module(&items);
        assert!(js.contains("claude-sonnet"));
        assert!(js.contains("gpt-4o"));
    }

    #[test]
    fn codegen_with_captures() {
        let mock_expr: Box<dyn Any> = Box::new(42u32);
        let captures: Vec<&dyn Any> = vec![mock_expr.as_ref()];

        let tpl = PromptTemplate {
            name: "test".to_string(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![
                    PromptPart::Text("You are ".to_string()),
                    PromptPart::Capture(0),
                    PromptPart::Text(".".to_string()),
                ],
            }],
            model: None,
            output: None,
            constraints: None,
        };

        let items = generate(&tpl, &captures, &mut MockContext);
        let js = emit_module(&items);
        // Should produce an arrow function with template literal
        assert!(js.contains("ctx"));
        assert!(js.contains("=>"));
    }

    #[test]
    fn codegen_with_examples() {
        let tpl = PromptTemplate {
            name: "test".to_string(),
            sections: vec![
                PromptSection::Role {
                    role: RoleName::System,
                    body: vec![PromptPart::Text("Hi".to_string())],
                },
                PromptSection::Examples(vec![Example {
                    pairs: vec![
                        (RoleName::User, "hello".to_string()),
                        (RoleName::Assistant, "hi".to_string()),
                    ],
                }]),
            ],
            model: None,
            output: None,
            constraints: None,
        };

        let items = generate(&tpl, &[], &mut MockContext);
        let js = emit_module(&items);
        assert!(js.contains("examples"));
        assert!(js.contains("hello"));
        assert!(js.contains("hi"));
    }

    #[test]
    fn codegen_with_constraints() {
        let tpl = PromptTemplate {
            name: "test".to_string(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Hi".to_string())],
            }],
            model: None,
            output: None,
            constraints: Some(Constraints {
                fields: vec![
                    ("temperature".to_string(), ConstraintValue::Number(0.7)),
                    ("max_tokens".to_string(), ConstraintValue::Number(4096.0)),
                ],
            }),
        };

        let items = generate(&tpl, &[], &mut MockContext);
        let js = emit_module(&items);
        assert!(js.contains("constraints"));
        assert!(js.contains("temperature"));
        assert!(js.contains("0.7"));
        assert!(js.contains("max_tokens"));
    }

    #[test]
    fn codegen_output_inline() {
        let tpl = PromptTemplate {
            name: "test".to_string(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Answer".to_string())],
            }],
            model: None,
            output: Some(OutputSpec {
                kind: OutputKind::Inline(vec![
                    OutputField { name: "answer".to_string(), ty: "str".to_string() },
                    OutputField { name: "confidence".to_string(), ty: "num".to_string() },
                    OutputField { name: "sources".to_string(), ty: "[str]".to_string() },
                ]),
            }),
            constraints: None,
        };

        let items = generate(&tpl, &[], &mut MockContext);
        let js = emit_module(&items);
        assert!(js.contains("outputSchema"));
        assert!(js.contains("string"));
        assert!(js.contains("number"));
        assert!(js.contains("array"));
    }

    #[test]
    fn codegen_import_statement() {
        let tpl = PromptTemplate {
            name: "test".to_string(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Hi".to_string())],
            }],
            model: None,
            output: None,
            constraints: None,
        };

        let items = generate(&tpl, &[], &mut MockContext);
        let js = emit_module(&items);
        assert!(js.contains("import { PromptTemplate } from \"@agentscript/prompt-runtime\""));
    }

    #[test]
    fn codegen_file_ref() {
        let items = generate_file_ref("system", "./system-prompt.txt");
        let js = emit_module(&items);
        assert!(js.contains("PromptTemplate"));
        assert!(js.contains("const system"));
        assert!(js.contains("readFile"));
        assert!(js.contains("system-prompt.txt"));
    }
}
