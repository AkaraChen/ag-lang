use ag_dsl_core::{CodegenContext, DslBlock, DslContent, DslError, DslHandler, DslPart};
use swc_ecma_ast as swc;

use crate::codegen;
use crate::lexer;
use crate::parser;
use crate::validator;

pub struct PromptDslHandler;

impl DslHandler for PromptDslHandler {
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
                // Warnings are non-fatal, we proceed

                // 4. Collect capture references for codegen
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
            DslContent::FileRef { path, .. } => {
                let items = codegen::generate_file_ref(&block.name, path);
                Ok(items)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::Span;

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
        fn translate_block(&mut self, _block: &dyn std::any::Any) -> Vec<swc_ecma_ast::Stmt> {
            Vec::new()
        }
    }

    #[test]
    fn handler_inline_simple() {
        let block = DslBlock {
            kind: "prompt".to_string(),
            name: "greeting".to_string(),
            content: DslContent::Inline {
                parts: vec![DslPart::Text(
                    "@role system\nYou are a helpful assistant.\n".to_string(),
                    Span::dummy(),
                )],
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = PromptDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_ok());
        let items = result.unwrap();
        let js = codegen::emit_module(&items);
        assert!(js.contains("greeting"));
        assert!(js.contains("PromptTemplate"));
        assert!(js.contains("system"));
    }

    #[test]
    fn handler_file_ref() {
        let block = DslBlock {
            kind: "prompt".to_string(),
            name: "system".to_string(),
            content: DslContent::FileRef {
                path: "./system-prompt.txt".to_string(),
                span: Span::dummy(),
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = PromptDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_ok());
        let items = result.unwrap();
        let js = codegen::emit_module(&items);
        assert!(js.contains("system"));
        assert!(js.contains("readFile"));
    }

    #[test]
    fn handler_with_capture() {
        let block = DslBlock {
            kind: "prompt".to_string(),
            name: "test".to_string(),
            content: DslContent::Inline {
                parts: vec![
                    DslPart::Text("@role system\nHello ".to_string(), Span::dummy()),
                    DslPart::Capture(Box::new(42u32), Span::dummy()),
                    DslPart::Text("!\n".to_string(), Span::dummy()),
                ],
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = PromptDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_ok());
        let items = result.unwrap();
        let js = codegen::emit_module(&items);
        assert!(js.contains("ctx"));
        assert!(js.contains("=>"));
    }

    #[test]
    fn handler_invalid_prompt_error() {
        let block = DslBlock {
            kind: "prompt".to_string(),
            name: "bad".to_string(),
            content: DslContent::Inline {
                parts: vec![DslPart::Text("".to_string(), Span::dummy())],
            },
            span: Span::dummy(),
        };

        let mut ctx = MockCodegenContext;
        let handler = PromptDslHandler;
        let result = handler.handle(&block, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("empty prompt"));
    }
}
