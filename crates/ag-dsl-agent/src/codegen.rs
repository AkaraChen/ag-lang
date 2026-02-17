use std::any::Any;

use ag_dsl_core::CodegenContext;
use ag_dsl_core::swc_helpers::{ident, str_lit, expr_or_spread, make_prop};
use ag_dsl_prompt::ast::{PromptSection, Example};
use ag_dsl_prompt::codegen::{build_content_expr, build_output_schema, build_constraints_expr};
use crate::ast::*;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

/// Generate JS AST for an agent template.
///
/// Produces: `import { AgentRuntime } from "@agentscript/runtime"`
/// and `const <name> = new AgentRuntime({ ... })`
pub fn generate(
    template: &AgentTemplate,
    captures: &[&dyn Any],
    ctx: &mut dyn CodegenContext,
) -> Vec<swc::ModuleItem> {
    let mut items = Vec::new();

    // 1. Import: import { AgentRuntime } from "@agentscript/runtime"
    items.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(
        swc::ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
                span: DUMMY_SP,
                local: ident("AgentRuntime"),
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(swc::Str {
                span: DUMMY_SP,
                value: "@agentscript/runtime".into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        },
    )));

    // 2. Build config object
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

    for section in &template.sections {
        match section {
            PromptSection::Role { role, body } => {
                let content_expr = build_content_expr(body, captures, ctx);
                let msg = swc::Expr::Object(swc::ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        make_prop("role", str_lit(role.as_str())),
                        make_prop("content", content_expr),
                    ],
                });
                messages.push(Some(expr_or_spread(msg)));
            }
            PromptSection::Examples(_) => {
                // handled separately
            }
            PromptSection::Messages { capture_index } => {
                if let Some(cap) = captures.get(*capture_index) {
                    let js_expr = ctx.translate_expr(*cap);
                    // Spread the messages capture into the array
                    messages.push(Some(swc::ExprOrSpread {
                        spread: Some(DUMMY_SP),
                        expr: Box::new(js_expr),
                    }));
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

    // tools property
    if let Some(capture_index) = template.tools_capture {
        if let Some(cap) = captures.get(capture_index) {
            let js_expr = ctx.translate_expr(*cap);
            config_props.push(make_prop("tools", js_expr));
        }
    }

    // skills property
    if let Some(capture_index) = template.skills_capture {
        if let Some(cap) = captures.get(capture_index) {
            let js_expr = ctx.translate_expr(*cap);
            config_props.push(make_prop("skills", js_expr));
        }
    }

    // agents property
    if let Some(capture_index) = template.agents_capture {
        if let Some(cap) = captures.get(capture_index) {
            let js_expr = ctx.translate_expr(*cap);
            config_props.push(make_prop("agents", js_expr));
        }
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

    // hooks property
    if !template.on_hooks.is_empty() {
        let hook_props: Vec<swc::PropOrSpread> = template
            .on_hooks
            .iter()
            .filter_map(|hook| {
                captures.get(hook.capture_index).map(|cap| {
                    let js_expr = ctx.translate_expr(*cap);
                    make_prop(&hook.event, js_expr)
                })
            })
            .collect();

        if !hook_props.is_empty() {
            config_props.push(make_prop(
                "hooks",
                swc::Expr::Object(swc::ObjectLit {
                    span: DUMMY_SP,
                    props: hook_props,
                }),
            ));
        }
    }

    // 3. const <name> = new AgentRuntime({ ... })
    let config_obj = swc::Expr::Object(swc::ObjectLit {
        span: DUMMY_SP,
        props: config_props,
    });

    let new_expr = swc::Expr::New(swc::NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Box::new(swc::Expr::Ident(ident("AgentRuntime"))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::swc_helpers::emit_module;
    use ag_dsl_prompt::ast::{ModelSpec, RoleName, PromptPart, Constraints, ConstraintValue, OutputSpec, OutputKind, OutputField};

    struct MockCtx;
    impl CodegenContext for MockCtx {
        fn translate_expr(&mut self, _expr: &dyn Any) -> swc::Expr {
            swc::Expr::Ident(ident("__mock__"))
        }
        fn translate_block(&mut self, _block: &dyn Any) -> Vec<swc::Stmt> {
            vec![]
        }
    }

    fn codegen_agent(template: &AgentTemplate) -> String {
        let caps: Vec<Box<dyn Any>> = vec![];
        let cap_refs: Vec<&dyn Any> = caps.iter().map(|c| c.as_ref()).collect();
        let mut ctx = MockCtx;
        let items = generate(template, &cap_refs, &mut ctx);
        emit_module(&items)
    }

    fn codegen_agent_with_captures(template: &AgentTemplate, n: usize) -> String {
        let caps: Vec<Box<dyn Any>> = (0..n).map(|i| Box::new(i as u32) as Box<dyn Any>).collect();
        let cap_refs: Vec<&dyn Any> = caps.iter().map(|c| c.as_ref()).collect();
        let mut ctx = MockCtx;
        let items = generate(template, &cap_refs, &mut ctx);
        emit_module(&items)
    }

    #[test]
    fn minimal_agent() {
        let template = AgentTemplate {
            name: "my_agent".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("You are helpful.".into())],
            }],
            model: None,
            output: None,
            constraints: None,
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let js = codegen_agent(&template);
        assert!(js.contains("AgentRuntime"), "should import AgentRuntime");
        assert!(js.contains("@agentscript/runtime"), "should import from runtime");
        assert!(js.contains("my_agent"), "should declare agent");
        assert!(js.contains("system"), "should have system role");
        assert!(js.contains("You are helpful"), "should have body text");
    }

    #[test]
    fn agent_with_model() {
        let template = AgentTemplate {
            name: "smart".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Hi".into())],
            }],
            model: Some(ModelSpec { models: vec!["claude-sonnet".into(), "gpt-4o".into()] }),
            output: None,
            constraints: None,
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let js = codegen_agent(&template);
        assert!(js.contains("model"), "should have model property");
        assert!(js.contains("claude-sonnet"), "should have model name");
        assert!(js.contains("gpt-4o"), "should have second model");
    }

    #[test]
    fn agent_with_tools() {
        let template = AgentTemplate {
            name: "tool_agent".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Use tools.".into())],
            }],
            model: None,
            output: None,
            constraints: None,
            tools_capture: Some(0),
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let js = codegen_agent_with_captures(&template, 1);
        assert!(js.contains("tools"), "should have tools property");
        assert!(js.contains("__mock__"), "should translate capture");
    }

    #[test]
    fn agent_with_constraints() {
        let template = AgentTemplate {
            name: "constrained".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Be brief.".into())],
            }],
            model: None,
            output: None,
            constraints: Some(Constraints {
                fields: vec![
                    ("temperature".into(), ConstraintValue::Number(0.5)),
                    ("max_tokens".into(), ConstraintValue::Number(100.0)),
                ],
            }),
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let js = codegen_agent(&template);
        assert!(js.contains("constraints"), "should have constraints");
        assert!(js.contains("temperature"), "should have temperature");
    }

    #[test]
    fn agent_with_hooks() {
        let template = AgentTemplate {
            name: "hooked".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Hi".into())],
            }],
            model: None,
            output: None,
            constraints: None,
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![
                OnHook { event: "init".into(), capture_index: 0 },
                OnHook { event: "message".into(), capture_index: 1 },
            ],
        };
        let js = codegen_agent_with_captures(&template, 2);
        assert!(js.contains("hooks"), "should have hooks property");
        assert!(js.contains("init"), "should have init hook");
        assert!(js.contains("message"), "should have message hook");
    }

    #[test]
    fn agent_with_output() {
        let template = AgentTemplate {
            name: "structured".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Return JSON.".into())],
            }],
            model: None,
            output: Some(OutputSpec {
                kind: OutputKind::Inline(vec![
                    OutputField { name: "answer".into(), ty: "str".into() },
                    OutputField { name: "confidence".into(), ty: "num".into() },
                ]),
            }),
            constraints: None,
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let js = codegen_agent(&template);
        assert!(js.contains("outputSchema"), "should have outputSchema");
        assert!(js.contains("answer"), "should have answer field");
        assert!(js.contains("confidence"), "should have confidence field");
    }

    #[test]
    fn agent_with_skills_and_agents() {
        let template = AgentTemplate {
            name: "orchestrator".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("Orchestrate.".into())],
            }],
            model: None,
            output: None,
            constraints: None,
            tools_capture: None,
            skills_capture: Some(0),
            agents_capture: Some(1),
            on_hooks: vec![],
        };
        let js = codegen_agent_with_captures(&template, 2);
        assert!(js.contains("skills"), "should have skills property");
        assert!(js.contains("agents"), "should have agents property");
    }

    #[test]
    fn agent_with_examples() {
        let template = AgentTemplate {
            name: "example_agent".into(),
            sections: vec![
                PromptSection::Role {
                    role: RoleName::System,
                    body: vec![PromptPart::Text("Help.".into())],
                },
                PromptSection::Examples(vec![Example {
                    pairs: vec![
                        (RoleName::User, "What is 2+2?".into()),
                        (RoleName::Assistant, "4".into()),
                    ],
                }]),
            ],
            model: None,
            output: None,
            constraints: None,
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let js = codegen_agent(&template);
        assert!(js.contains("examples"), "should have examples property");
        assert!(js.contains("What is 2+2?"), "should have example user msg");
    }

    #[test]
    fn agent_with_capture_in_body() {
        let template = AgentTemplate {
            name: "dynamic".into(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![
                    PromptPart::Text("Hello ".into()),
                    PromptPart::Capture(0),
                    PromptPart::Text("!".into()),
                ],
            }],
            model: None,
            output: None,
            constraints: None,
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let js = codegen_agent_with_captures(&template, 1);
        assert!(js.contains("dynamic"), "should declare agent");
        assert!(js.contains("__mock__"), "should translate capture");
    }
}
