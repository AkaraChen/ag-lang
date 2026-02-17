use ag_dsl_core::{CodegenContext, DslBlock, DslContent, DslError, DslHandler, DslPart};
use swc_ecma_ast as swc;

use crate::codegen;
use crate::lexer;
use crate::parser;
use crate::validator;

pub struct AgentDslHandler;

impl DslHandler for AgentDslHandler {
    fn handle(
        &self,
        block: &DslBlock,
        ctx: &mut dyn CodegenContext,
    ) -> Result<Vec<swc::ModuleItem>, DslError> {
        match &block.content {
            DslContent::Inline { parts } => {
                // 1. Lex
                let tokens = lexer::lex(parts);

                // 2. Parse
                let template = parser::parse(&block.name, &tokens).map_err(|diags| {
                    let messages: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
                    DslError {
                        message: messages.join("; "),
                        span: Some(block.span),
                    }
                })?;

                // 3. Validate
                let _warnings = validator::validate(&template);

                // 4. Collect capture references
                let captures: Vec<&dyn std::any::Any> = parts
                    .iter()
                    .filter_map(|p| match p {
                        DslPart::Capture(expr, _) => Some(expr.as_ref()),
                        _ => None,
                    })
                    .collect();

                // 5. Codegen
                let items = codegen::generate(&template, &captures, ctx);
                Ok(items)
            }
            DslContent::FileRef { .. } => Err(DslError {
                message: "@agent blocks do not support `from` file references".to_string(),
                span: Some(block.span),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::Span;
    use ag_dsl_core::swc_helpers::emit_module;

    struct MockCodegenContext;

    impl CodegenContext for MockCodegenContext {
        fn translate_expr(&mut self, _expr: &dyn std::any::Any) -> swc::Expr {
            swc::Expr::Ident(swc::Ident {
                span: swc_common::DUMMY_SP,
                ctxt: swc_common::SyntaxContext::empty(),
                sym: "mockExpr".into(),
                optional: false,
            })
        }
        fn translate_block(&mut self, _block: &dyn std::any::Any) -> Vec<swc::Stmt> {
            Vec::new()
        }
    }

    #[test]
    fn handler_inline_simple() {
        let block = DslBlock {
            kind: "agent".to_string(),
            name: "my_agent".to_string(),
            content: DslContent::Inline {
                parts: vec![DslPart::Text(
                    "@role system\nYou are a helpful assistant.\n".to_string(),
                    Span::dummy(),
                )],
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = AgentDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_ok());
        let items = result.unwrap();
        let js = emit_module(&items);
        assert!(js.contains("my_agent"));
        assert!(js.contains("AgentRuntime"));
        assert!(js.contains("system"));
    }

    #[test]
    fn handler_file_ref_rejected() {
        let block = DslBlock {
            kind: "agent".to_string(),
            name: "bad".to_string(),
            content: DslContent::FileRef {
                path: "./agent.txt".to_string(),
                span: Span::dummy(),
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = AgentDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("file references"));
    }

    #[test]
    fn handler_parse_error() {
        let block = DslBlock {
            kind: "agent".to_string(),
            name: "empty".to_string(),
            content: DslContent::Inline {
                parts: vec![DslPart::Text("".to_string(), Span::dummy())],
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = AgentDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn handler_with_capture() {
        let block = DslBlock {
            kind: "agent".to_string(),
            name: "tool_agent".to_string(),
            content: DslContent::Inline {
                parts: vec![
                    DslPart::Text("@role system\nUse these tools:\n@tools #{ ".to_string(), Span::dummy()),
                    DslPart::Capture(Box::new(42u32), Span::dummy()),
                    DslPart::Text(" }\n".to_string(), Span::dummy()),
                ],
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = AgentDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_ok());
        let items = result.unwrap();
        let js = emit_module(&items);
        assert!(js.contains("tool_agent"));
        assert!(js.contains("tools"));
    }
}
