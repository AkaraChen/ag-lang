use ag_ast::*;
use ag_lexer::{Lexer, Token, TokenKind};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    source: &'a str,
}

pub struct ParseResult {
    pub module: Module,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(source: &str) -> ParseResult {
    let tokens: Vec<Token> = Lexer::tokenize(source)
        .into_iter()
        .filter(|t| {
            !matches!(
                t.kind,
                TokenKind::LineComment(_) | TokenKind::BlockComment(_) | TokenKind::DocComment(_)
            )
        })
        .collect();
    let mut parser = Parser::new(tokens, source);
    let module = parser.parse_module();
    ParseResult {
        module,
        diagnostics: parser.diagnostics,
    }
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token>, source: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            source,
        }
    }

    // ── Utility methods ────────────────────────────────────

    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn peek_token(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Option<Token> {
        if self.at(expected) {
            Some(self.advance().clone())
        } else {
            let span = self.peek_token().span;
            self.diagnostics.push(Diagnostic {
                message: format!("expected {:?}, found {:?}", expected, self.peek()),
                span,
            });
            None
        }
    }

    fn expect_ident(&mut self) -> Option<String> {
        if let TokenKind::Ident(_) = self.peek() {
            let tok = self.advance().clone();
            if let TokenKind::Ident(name) = tok.kind {
                Some(name)
            } else {
                None
            }
        } else {
            let span = self.peek_token().span;
            self.diagnostics.push(Diagnostic {
                message: format!("expected identifier, found {:?}", self.peek()),
                span,
            });
            None
        }
    }

    fn current_span(&self) -> Span {
        self.peek_token().span
    }

    fn error(&mut self, msg: impl Into<String>) {
        let span = self.current_span();
        self.diagnostics.push(Diagnostic {
            message: msg.into(),
            span,
        });
    }

    fn synchronize(&mut self) {
        loop {
            match self.peek() {
                TokenKind::Eof => break,
                TokenKind::Semi => {
                    self.advance();
                    break;
                }
                TokenKind::RBrace => break,
                TokenKind::Fn
                | TokenKind::Let
                | TokenKind::Mut
                | TokenKind::Const
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Type
                | TokenKind::Import
                | TokenKind::Pub
                | TokenKind::For
                | TokenKind::While
                | TokenKind::Try
                | TokenKind::If
                | TokenKind::Match
                | TokenKind::Ret
                | TokenKind::At
                | TokenKind::Extern => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ── Module parsing ─────────────────────────────────────

    fn parse_module(&mut self) -> Module {
        let mut items = Vec::new();
        while !matches!(self.peek(), TokenKind::Eof) {
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.synchronize(),
            }
        }
        Module { items }
    }

    fn parse_item(&mut self) -> Option<Item> {
        match self.peek() {
            TokenKind::Import => self.parse_import().map(Item::Import),
            TokenKind::Let | TokenKind::Mut | TokenKind::Const => {
                self.parse_var_decl().map(Item::VarDecl)
            }
            TokenKind::Fn | TokenKind::Async => self.parse_fn_decl(false).map(Item::FnDecl),
            TokenKind::Pub => {
                let start = self.current_span();
                self.advance(); // consume 'pub'
                match self.peek() {
                    TokenKind::Fn | TokenKind::Async => self.parse_fn_decl(true).map(Item::FnDecl),
                    _ => {
                        self.error("expected `fn` after `pub`");
                        None
                    }
                }
            }
            TokenKind::Struct => self.parse_struct_decl().map(Item::StructDecl),
            TokenKind::Enum => self.parse_enum_decl().map(Item::EnumDecl),
            TokenKind::Type => self.parse_type_alias().map(Item::TypeAlias),
            TokenKind::Extern => self.parse_extern_item(None),
            TokenKind::At => {
                // Check if this is @js annotation (followed by "js" ident)
                if self.pos + 1 < self.tokens.len() {
                    if let TokenKind::Ident(ref name) = self.tokens[self.pos + 1].kind {
                        if name == "js" {
                            return self.parse_js_annotated_extern();
                        }
                    }
                }
                self.parse_dsl_block().map(Item::DslBlock)
            }
            // Control flow statements at top level — wrap as ExprStmt containing block-level constructs
            TokenKind::For | TokenKind::While | TokenKind::Try | TokenKind::Ret => {
                let span = self.current_span();
                let stmt = match self.peek() {
                    TokenKind::For => self.parse_for().map(Stmt::For)?,
                    TokenKind::While => self.parse_while().map(Stmt::While)?,
                    TokenKind::Try => self.parse_try_catch().map(Stmt::TryCatch)?,
                    TokenKind::Ret => {
                        let r = self.parse_ret()?;
                        if matches!(self.peek(), TokenKind::Semi) {
                            self.advance();
                        }
                        Stmt::Return(r)
                    }
                    _ => unreachable!(),
                };
                // Wrap statement in a block expression as an ExprStmt item
                Some(Item::ExprStmt(ExprStmt {
                    expr: Expr::Block(Box::new(Block {
                        stmts: vec![stmt],
                        tail_expr: None,
                        span,
                    })),
                    span,
                }))
            }
            _ => {
                let expr = self.parse_expr(0)?;
                let span = self.current_span();
                if matches!(self.peek(), TokenKind::Semi) {
                    self.advance();
                }
                Some(Item::ExprStmt(ExprStmt { expr, span }))
            }
        }
    }

    // ── Import ─────────────────────────────────────────────

    fn parse_import(&mut self) -> Option<Import> {
        let start = self.current_span();
        self.advance(); // consume 'import'

        // Check for namespace import: import * as name from "path"
        if matches!(self.peek(), TokenKind::Star) {
            self.advance(); // consume '*'
            self.expect(&TokenKind::As)?;
            let alias = self.expect_ident()?;
            self.expect(&TokenKind::From)?;
            let path = self.parse_string_literal()?;
            let end = self.current_span();
            return Some(Import {
                names: Vec::new(),
                path,
                namespace: Some(alias),
                span: Span::new(start.start, end.end),
            });
        }

        // Named import: import { a, b } from "path"
        self.expect(&TokenKind::LBrace)?;
        let mut names = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let name_span = self.current_span();
            let name = self.expect_ident()?;
            let alias = if matches!(self.peek(), TokenKind::As) {
                self.advance();
                Some(self.expect_ident()?)
            } else {
                None
            };
            names.push(ImportName {
                name,
                alias,
                span: name_span,
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect(&TokenKind::From)?;
        let path = self.parse_string_literal()?;
        let end = self.current_span();
        Some(Import {
            names,
            path,
            namespace: None,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_string_literal(&mut self) -> Option<String> {
        if let TokenKind::StringLiteral(_) = self.peek() {
            let tok = self.advance().clone();
            if let TokenKind::StringLiteral(s) = tok.kind {
                return Some(s);
            }
        }
        self.error("expected string literal");
        None
    }

    // ── Variable declarations ──────────────────────────────

    fn parse_var_decl(&mut self) -> Option<VarDecl> {
        let start = self.current_span();
        let kind = match self.peek() {
            TokenKind::Let => VarKind::Let,
            TokenKind::Mut => VarKind::Mut,
            TokenKind::Const => VarKind::Const,
            _ => return None,
        };
        self.advance();

        let name = self.expect_ident()?;

        let ty = if matches!(self.peek(), TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&TokenKind::Eq)?;
        let init = self.parse_expr(0)?;

        if matches!(self.peek(), TokenKind::Semi) {
            self.advance();
        }

        let end = self.current_span();
        Some(VarDecl {
            kind,
            name,
            ty,
            init,
            span: Span::new(start.start, end.end),
        })
    }

    // ── Function declarations ──────────────────────────────

    fn parse_fn_decl(&mut self, is_pub: bool) -> Option<FnDecl> {
        let start = self.current_span();

        let is_async = if matches!(self.peek(), TokenKind::Async) {
            self.advance();
            true
        } else {
            false
        };

        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;

        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if matches!(self.peek(), TokenKind::ThinArrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let end = body.span;

        Some(FnDecl {
            name,
            params,
            return_type,
            body,
            is_pub,
            is_async,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
            let start = self.current_span();
            let name = self.expect_ident()?;

            let ty = if matches!(self.peek(), TokenKind::Colon) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            let default = if matches!(self.peek(), TokenKind::Eq) {
                self.advance();
                Some(self.parse_expr(0)?)
            } else {
                None
            };

            let end = self.current_span();
            params.push(Param {
                name,
                ty,
                default,
                is_variadic: false,
                span: Span::new(start.start, end.end),
            });

            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        Some(params)
    }

    // ── Struct declarations ────────────────────────────────

    fn parse_struct_decl(&mut self) -> Option<StructDecl> {
        let start = self.current_span();
        self.advance(); // consume 'struct'
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let fstart = self.current_span();
            let fname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let mut ftype = self.parse_type()?;

            // Check for optional field: T?
            if matches!(self.peek(), TokenKind::Question) {
                let qspan = self.current_span();
                self.advance();
                ftype = TypeExpr::Nullable(
                    Box::new(ftype),
                    Span::new(fstart.start, qspan.end),
                );
            }

            let default = if matches!(self.peek(), TokenKind::Eq) {
                self.advance();
                Some(self.parse_expr(0)?)
            } else {
                None
            };

            let fend = self.current_span();
            fields.push(Field {
                name: fname,
                ty: ftype,
                default,
                span: Span::new(fstart.start, fend.end),
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(&TokenKind::RBrace)?;
        let end = self.current_span();
        Some(StructDecl {
            name,
            fields,
            span: Span::new(start.start, end.end),
        })
    }

    // ── Enum declarations ──────────────────────────────────

    fn parse_enum_decl(&mut self) -> Option<EnumDecl> {
        let start = self.current_span();
        self.advance(); // consume 'enum'
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let vstart = self.current_span();
            let vname = self.expect_ident()?;
            let fields = if matches!(self.peek(), TokenKind::LParen) {
                self.advance();
                let mut vfields = Vec::new();
                while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                    let fstart = self.current_span();
                    let fname = self.expect_ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let ftype = self.parse_type()?;
                    let fend = self.current_span();
                    vfields.push(Field {
                        name: fname,
                        ty: ftype,
                        default: None,
                        span: Span::new(fstart.start, fend.end),
                    });
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RParen)?;
                vfields
            } else {
                Vec::new()
            };
            let vend = self.current_span();
            variants.push(Variant {
                name: vname,
                fields,
                span: Span::new(vstart.start, vend.end),
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(&TokenKind::RBrace)?;
        let end = self.current_span();
        Some(EnumDecl {
            name,
            variants,
            span: Span::new(start.start, end.end),
        })
    }

    // ── Type alias ─────────────────────────────────────────

    fn parse_type_alias(&mut self) -> Option<TypeAlias> {
        let start = self.current_span();
        self.advance(); // consume 'type'
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type()?;
        let end = self.current_span();
        Some(TypeAlias {
            name,
            ty,
            span: Span::new(start.start, end.end),
        })
    }

    // ── DSL block parsing ─────────────────────────────────

    fn parse_dsl_block(&mut self) -> Option<DslBlock> {
        let start = self.current_span();
        self.advance(); // consume '@'

        // Expect DSL kind identifier
        let kind = match self.peek() {
            TokenKind::Ident(_) => {
                if let TokenKind::Ident(name) = self.advance().kind.clone() {
                    name
                } else {
                    unreachable!()
                }
            }
            _ => {
                self.error("expected identifier after `@`");
                return None;
            }
        };

        // Expect DSL block name identifier
        let name_span = self.current_span();
        let name = match self.peek() {
            TokenKind::Ident(_) => {
                if let TokenKind::Ident(n) = self.advance().kind.clone() {
                    n
                } else {
                    unreachable!()
                }
            }
            _ => {
                self.error(format!("expected DSL block name after `@{}`", kind));
                return None;
            }
        };
        let name_ident = Ident {
            name: name.clone(),
            span: name_span,
        };

        // Check for `from` (file reference) or ``` (inline block)
        match self.peek() {
            TokenKind::From => {
                self.advance(); // consume 'from'
                let path_span = self.current_span();
                match self.peek() {
                    TokenKind::StringLiteral(_) => {
                        let tok = self.advance().clone();
                        if let TokenKind::StringLiteral(path) = tok.kind {
                            let end = tok.span;
                            Some(DslBlock {
                                kind,
                                name: name_ident,
                                content: DslContent::FileRef {
                                    path,
                                    span: path_span,
                                },
                                span: Span::new(start.start, end.end),
                            })
                        } else {
                            unreachable!()
                        }
                    }
                    _ => {
                        self.error("expected string literal after `from`");
                        None
                    }
                }
            }
            _ => {
                // Inline block: use lexer to scan raw DSL content
                // We need to find the byte offset after the name token to create a sub-lexer
                let byte_offset = self.peek_token().span.start as usize;
                let remaining = &self.source[byte_offset..];
                let mut sub_lexer = Lexer::new(remaining);
                let start_tok = sub_lexer.enter_dsl_raw_mode();

                if matches!(start_tok.kind, TokenKind::Error(_)) {
                    self.error(format!(
                        "expected ``` or `from` after `@{} {}`",
                        kind, name
                    ));
                    return None;
                }

                // Collect DSL tokens from sub-lexer
                let mut dsl_tokens = Vec::new();
                loop {
                    let tok = sub_lexer.next_token();
                    let is_end = matches!(
                        tok.kind,
                        TokenKind::DslBlockEnd | TokenKind::Eof | TokenKind::Error(_)
                    );
                    let is_error = matches!(tok.kind, TokenKind::Error(_));
                    dsl_tokens.push(tok);
                    if is_end {
                        break;
                    }
                    if is_error {
                        break;
                    }
                }

                // Parse the DSL tokens into DslParts
                let mut parts = Vec::new();
                let mut dsl_pos = 0;
                while dsl_pos < dsl_tokens.len() {
                    let tok = &dsl_tokens[dsl_pos];
                    match &tok.kind {
                        TokenKind::DslText(text) => {
                            let span = Span::new(
                                byte_offset as u32 + tok.span.start,
                                byte_offset as u32 + tok.span.end,
                            );
                            parts.push(DslPart::Text(text.clone(), span));
                            dsl_pos += 1;
                        }
                        TokenKind::DslCaptureStart => {
                            let cap_start_span = Span::new(
                                byte_offset as u32 + tok.span.start,
                                byte_offset as u32 + tok.span.end,
                            );
                            dsl_pos += 1;
                            // Collect tokens until DslCaptureEnd
                            let mut capture_tokens = Vec::new();
                            while dsl_pos < dsl_tokens.len() {
                                let ct = &dsl_tokens[dsl_pos];
                                if matches!(ct.kind, TokenKind::DslCaptureEnd) {
                                    dsl_pos += 1;
                                    break;
                                }
                                // Adjust span
                                let mut adjusted = ct.clone();
                                adjusted.span = Span::new(
                                    byte_offset as u32 + ct.span.start,
                                    byte_offset as u32 + ct.span.end,
                                );
                                capture_tokens.push(adjusted);
                                dsl_pos += 1;
                            }
                            // Add EOF token for sub-parser
                            let eof_span = capture_tokens
                                .last()
                                .map(|t| t.span)
                                .unwrap_or(cap_start_span);
                            capture_tokens.push(Token {
                                kind: TokenKind::Eof,
                                span: eof_span,
                                text: String::new(),
                            });
                            // Parse the capture expression
                            let mut sub_parser = Parser::new(capture_tokens, self.source);
                            if let Some(expr) = sub_parser.parse_expr(0) {
                                let expr_span = cap_start_span;
                                parts.push(DslPart::Capture(Box::new(expr), expr_span));
                            } else {
                                self.diagnostics.extend(sub_parser.diagnostics);
                            }
                        }
                        TokenKind::DslBlockEnd => {
                            dsl_pos += 1;
                            break;
                        }
                        TokenKind::Error(msg) => {
                            let span = Span::new(
                                byte_offset as u32 + tok.span.start,
                                byte_offset as u32 + tok.span.end,
                            );
                            self.diagnostics.push(Diagnostic {
                                message: msg.clone(),
                                span,
                            });
                            dsl_pos += 1;
                            break;
                        }
                        _ => {
                            dsl_pos += 1;
                        }
                    }
                }

                // Advance the main parser past the DSL block
                // Find the byte position after the closing ```
                let last_tok = dsl_tokens.last().unwrap();
                let end_byte = byte_offset + last_tok.span.end as usize;
                // Skip main tokens until we're past end_byte
                while self.pos < self.tokens.len() {
                    if self.tokens[self.pos].span.start as usize >= end_byte {
                        break;
                    }
                    self.pos += 1;
                }

                let end_span = Span::new(start.start, end_byte as u32);
                Some(DslBlock {
                    kind,
                    name: name_ident,
                    content: DslContent::Inline { parts },
                    span: end_span,
                })
            }
        }
    }

    // ── Extern declarations ──────────────────────────────

    fn parse_js_annotated_extern(&mut self) -> Option<Item> {
        let annotation = self.parse_js_annotation()?;
        if !matches!(self.peek(), TokenKind::Extern) {
            self.error("@js annotation can only be applied to extern declarations");
            return None;
        }
        self.parse_extern_item(Some(annotation))
    }

    fn parse_js_annotation(&mut self) -> Option<JsAnnotation> {
        let start = self.current_span();
        self.advance(); // consume '@'
        // Expect 'js' identifier
        let name = self.expect_ident()?;
        if name != "js" {
            self.error("expected `js` after `@`");
            return None;
        }
        self.expect(&TokenKind::LParen)?;
        let module = self.parse_string_literal()?;
        let mut js_name = None;
        if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            // Expect name = "jsName"
            let key = self.expect_ident()?;
            if key != "name" {
                self.error("expected `name` in @js annotation");
                return None;
            }
            self.expect(&TokenKind::Eq)?;
            js_name = Some(self.parse_string_literal()?);
        }
        self.expect(&TokenKind::RParen)?;
        let end = self.current_span();
        Some(JsAnnotation {
            module: Some(module),
            js_name,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_extern_item(&mut self, js_annotation: Option<JsAnnotation>) -> Option<Item> {
        let start = self.current_span();
        self.advance(); // consume 'extern'
        match self.peek() {
            TokenKind::Fn => self.parse_extern_fn_decl(start, js_annotation).map(Item::ExternFnDecl),
            TokenKind::Struct => self.parse_extern_struct_decl(start, js_annotation).map(Item::ExternStructDecl),
            TokenKind::Type => self.parse_extern_type_decl(start, js_annotation).map(Item::ExternTypeDecl),
            _ => {
                self.error("expected `fn`, `struct`, or `type` after `extern`");
                None
            }
        }
    }

    fn parse_extern_fn_decl(&mut self, start: Span, js_annotation: Option<JsAnnotation>) -> Option<ExternFnDecl> {
        self.advance(); // consume 'fn'
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let (params, variadic) = self.parse_extern_params()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if matches!(self.peek(), TokenKind::ThinArrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Reject function body
        if matches!(self.peek(), TokenKind::LBrace) {
            self.error("extern functions must not have a body");
            return None;
        }

        let end = self.current_span();
        Some(ExternFnDecl {
            name,
            params,
            return_type,
            js_annotation,
            variadic,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_extern_params(&mut self) -> Option<(Vec<Param>, bool)> {
        let mut params = Vec::new();
        let mut variadic = false;
        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
            let start = self.current_span();
            let is_variadic = if matches!(self.peek(), TokenKind::DotDotDot) {
                self.advance();
                true
            } else {
                false
            };
            let name = self.expect_ident()?;

            let ty = if matches!(self.peek(), TokenKind::Colon) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            let default = if matches!(self.peek(), TokenKind::Eq) {
                self.advance();
                Some(self.parse_expr(0)?)
            } else {
                None
            };

            let end = self.current_span();
            params.push(Param {
                name,
                ty,
                default,
                is_variadic,
                span: Span::new(start.start, end.end),
            });

            if is_variadic {
                variadic = true;
                // Variadic must be last
                if matches!(self.peek(), TokenKind::Comma) {
                    self.advance();
                    if !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                        self.error("variadic parameter must be the last parameter");
                    }
                }
                break;
            }

            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        Some((params, variadic))
    }

    fn parse_extern_struct_decl(&mut self, start: Span, js_annotation: Option<JsAnnotation>) -> Option<ExternStructDecl> {
        self.advance(); // consume 'struct'
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Fn) {
                // Method signature
                let mstart = self.current_span();
                self.advance(); // consume 'fn'
                let mname = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let mparams = self.parse_params()?;
                self.expect(&TokenKind::RParen)?;
                let mret = if matches!(self.peek(), TokenKind::ThinArrow) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                // Reject method body
                if matches!(self.peek(), TokenKind::LBrace) {
                    self.error("extern struct methods must not have a body");
                    return None;
                }
                let mend = self.current_span();
                methods.push(MethodSignature {
                    name: mname,
                    params: mparams,
                    return_type: mret,
                    span: Span::new(mstart.start, mend.end),
                });
            } else {
                // Field
                let fstart = self.current_span();
                let fname = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ftype = self.parse_type()?;
                let fend = self.current_span();
                fields.push(Field {
                    name: fname,
                    ty: ftype,
                    default: None,
                    span: Span::new(fstart.start, fend.end),
                });
            }
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(&TokenKind::RBrace)?;
        let end = self.current_span();
        Some(ExternStructDecl {
            name,
            fields,
            methods,
            js_annotation,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_extern_type_decl(&mut self, start: Span, js_annotation: Option<JsAnnotation>) -> Option<ExternTypeDecl> {
        self.advance(); // consume 'type'
        let name = self.expect_ident()?;
        let end = self.current_span();
        Some(ExternTypeDecl {
            name,
            js_annotation,
            span: Span::new(start.start, end.end),
        })
    }

    // ── Type parsing ───────────────────────────────────────

    fn parse_type(&mut self) -> Option<TypeExpr> {
        let mut ty = self.parse_type_primary()?;

        // Handle nullable T? (postfix)
        if matches!(self.peek(), TokenKind::Question) {
            let span = self.current_span();
            self.advance();
            ty = TypeExpr::Nullable(Box::new(ty), span);
        }

        // Handle union T | U
        if matches!(self.peek(), TokenKind::Pipe) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_type()?;
            ty = TypeExpr::Union(Box::new(ty), Box::new(right), span);
        }

        Some(ty)
    }

    fn parse_type_primary(&mut self) -> Option<TypeExpr> {
        let start = self.current_span();
        match self.peek().clone() {
            TokenKind::LBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                let end = self.current_span();
                Some(TypeExpr::Array(
                    Box::new(inner),
                    Span::new(start.start, end.end),
                ))
            }
            TokenKind::LBrace => {
                self.advance();
                // Could be Map {K: V} or Object { field: T }
                // Peek ahead: if <ident> <colon>, check if the value after colon
                // starts a type or could be a map key type
                // Simple heuristic: if the first key is a type keyword, treat as map
                let first_name = if let TokenKind::Ident(name) = self.peek().clone() {
                    Some(name)
                } else {
                    None
                };

                // For now, parse as Object type { field: Type, ... }
                // Map types use TypeExpr like {str: int}
                if let Some(name) = first_name {
                    // Check if this is a named type used as map key
                    let is_type_name = matches!(
                        name.as_str(),
                        "str" | "int" | "num" | "bool" | "nil" | "any"
                    );

                    // Save position to backtrack
                    let saved_pos = self.pos;

                    // Try to read as: key_type : value_type }
                    let key_type = self.parse_type();
                    if let Some(kt) = key_type {
                        if matches!(self.peek(), TokenKind::Colon) {
                            self.advance();
                            // Check what follows — if it's a type followed by }, it's a map
                            let vt = self.parse_type();
                            if let Some(val_type) = vt {
                                if matches!(self.peek(), TokenKind::RBrace) {
                                    // It's a map type if the key is a primitive type name
                                    if is_type_name {
                                        self.advance();
                                        let end = self.current_span();
                                        return Some(TypeExpr::Map(
                                            Box::new(kt),
                                            Box::new(val_type),
                                            Span::new(start.start, end.end),
                                        ));
                                    }
                                    // Otherwise it's a single-field object: { name: Type }
                                    self.advance();
                                    let end = self.current_span();
                                    return Some(TypeExpr::Object(ObjectType {
                                        fields: vec![TypeField {
                                            name,
                                            ty: val_type,
                                            span: Span::new(start.start, end.end),
                                        }],
                                        span: Span::new(start.start, end.end),
                                    }));
                                }
                            }
                        }
                    }

                    // Backtrack and parse as object type
                    self.pos = saved_pos;
                }

                // Parse as object type { field: Type, ... }
                let mut fields = Vec::new();
                while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
                    let fstart = self.current_span();
                    let fname = self.expect_ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let ftype = self.parse_type()?;
                    let fend = self.current_span();
                    fields.push(TypeField {
                        name: fname,
                        ty: ftype,
                        span: Span::new(fstart.start, fend.end),
                    });
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                let end = self.current_span();
                Some(TypeExpr::Object(ObjectType {
                    fields,
                    span: Span::new(start.start, end.end),
                }))
            }
            TokenKind::LParen => {
                // Function type: (params) -> Return
                self.advance();
                let mut params = Vec::new();
                while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                    params.push(self.parse_type()?);
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::ThinArrow)?;
                let ret = self.parse_type()?;
                let end = self.current_span();
                Some(TypeExpr::Function(FunctionType {
                    params,
                    ret: Box::new(ret),
                    span: Span::new(start.start, end.end),
                }))
            }
            TokenKind::Ident(_) => {
                let tok = self.advance().clone();
                if let TokenKind::Ident(name) = tok.kind {
                    // Check for Promise<T> generic syntax
                    if name == "Promise" && matches!(self.peek(), TokenKind::Lt) {
                        self.advance(); // consume '<'
                        let inner = self.parse_type()?;
                        self.expect(&TokenKind::Gt)?;
                        let end = self.current_span();
                        Some(TypeExpr::Promise(
                            Box::new(inner),
                            Span::new(tok.span.start, end.end),
                        ))
                    } else {
                        Some(TypeExpr::Named(name, tok.span))
                    }
                } else {
                    None
                }
            }
            TokenKind::Nil => {
                let tok = self.advance().clone();
                Some(TypeExpr::Named("nil".to_string(), tok.span))
            }
            _ => {
                self.error("expected type");
                None
            }
        }
    }

    // ── Block parsing ──────────────────────────────────────

    fn parse_block(&mut self) -> Option<Block> {
        let start = self.current_span();
        self.expect(&TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            // Try to parse a statement
            match self.peek() {
                TokenKind::Let | TokenKind::Mut | TokenKind::Const => {
                    if let Some(decl) = self.parse_var_decl() {
                        stmts.push(Stmt::VarDecl(decl));
                    } else {
                        self.synchronize();
                    }
                }
                TokenKind::Ret => {
                    if let Some(ret) = self.parse_ret() {
                        stmts.push(Stmt::Return(ret));
                        if matches!(self.peek(), TokenKind::Semi) {
                            self.advance();
                        }
                    }
                }
                TokenKind::For => {
                    if let Some(f) = self.parse_for() {
                        stmts.push(Stmt::For(f));
                    }
                }
                TokenKind::While => {
                    if let Some(w) = self.parse_while() {
                        stmts.push(Stmt::While(w));
                    }
                }
                TokenKind::Try => {
                    if let Some(tc) = self.parse_try_catch() {
                        stmts.push(Stmt::TryCatch(tc));
                    }
                }
                _ => {
                    // Parse expression — could be tail or statement
                    if let Some(expr) = self.parse_expr(0) {
                        if matches!(self.peek(), TokenKind::Semi) {
                            self.advance();
                            let span = self.current_span();
                            stmts.push(Stmt::ExprStmt(ExprStmt { expr, span }));
                        } else if matches!(self.peek(), TokenKind::RBrace) {
                            // This is the tail expression (implicit return)
                            tail_expr = Some(Box::new(expr));
                        } else {
                            let span = self.current_span();
                            stmts.push(Stmt::ExprStmt(ExprStmt { expr, span }));
                        }
                    } else {
                        self.synchronize();
                    }
                }
            }
        }

        self.expect(&TokenKind::RBrace)?;
        let end = self.current_span();

        Some(Block {
            stmts,
            tail_expr,
            span: Span::new(start.start, end.end),
        })
    }

    // ── Statement parsing ──────────────────────────────────

    fn parse_ret(&mut self) -> Option<ReturnStmt> {
        let start = self.current_span();
        self.advance(); // consume 'ret'

        let value = if matches!(
            self.peek(),
            TokenKind::Semi | TokenKind::RBrace | TokenKind::Eof
        ) {
            None
        } else {
            Some(self.parse_expr(0)?)
        };

        let end = self.current_span();
        Some(ReturnStmt {
            value,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_for(&mut self) -> Option<ForStmt> {
        let start = self.current_span();
        self.advance(); // consume 'for'
        let binding = self.expect_ident()?;
        self.expect(&TokenKind::In)?;
        let iter = self.parse_expr(0)?;
        let body = self.parse_block()?;
        let end = body.span;
        Some(ForStmt {
            binding,
            iter,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_while(&mut self) -> Option<WhileStmt> {
        let start = self.current_span();
        self.advance(); // consume 'while'
        let condition = self.parse_expr(0)?;
        let body = self.parse_block()?;
        let end = body.span;
        Some(WhileStmt {
            condition,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_try_catch(&mut self) -> Option<TryCatchStmt> {
        let start = self.current_span();
        self.advance(); // consume 'try'
        let try_block = self.parse_block()?;
        self.expect(&TokenKind::Catch)?;
        let catch_binding = self.expect_ident()?;
        let catch_block = self.parse_block()?;
        let end = catch_block.span;
        Some(TryCatchStmt {
            try_block,
            catch_binding,
            catch_block,
            span: Span::new(start.start, end.end),
        })
    }

    // ── Expression parsing (Pratt) ─────────────────────────

    fn parse_expr(&mut self, min_bp: u8) -> Option<Expr> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // Check for postfix operators first
            match self.peek() {
                TokenKind::Dot => {
                    let span = self.current_span();
                    self.advance();
                    let field = self.expect_ident()?;
                    lhs = Expr::Member(MemberExpr {
                        object: Box::new(lhs),
                        field,
                        span,
                    });
                    continue;
                }
                TokenKind::QuestionDot => {
                    let span = self.current_span();
                    self.advance();
                    let field = self.expect_ident()?;
                    lhs = Expr::OptionalChain(Box::new(OptionalChainExpr {
                        object: lhs,
                        field,
                        span,
                    }));
                    continue;
                }
                TokenKind::ColonColon => {
                    // Enum::Variant or Enum::Variant(args)
                    let span = self.current_span();
                    self.advance();
                    let field = self.expect_ident()?;
                    lhs = Expr::Member(MemberExpr {
                        object: Box::new(lhs),
                        field,
                        span,
                    });
                    continue;
                }
                TokenKind::LParen => {
                    let span = self.current_span();
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                        args.push(self.parse_expr(0)?);
                        if matches!(self.peek(), TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    let end = self.current_span();
                    lhs = Expr::Call(CallExpr {
                        callee: Box::new(lhs),
                        args,
                        span: Span::new(span.start, end.end),
                    });
                    continue;
                }
                TokenKind::LBracket => {
                    let span = self.current_span();
                    self.advance();
                    let index = self.parse_expr(0)?;
                    self.expect(&TokenKind::RBracket)?;
                    let end = self.current_span();
                    lhs = Expr::Index(IndexExpr {
                        object: Box::new(lhs),
                        index: Box::new(index),
                        span: Span::new(span.start, end.end),
                    });
                    continue;
                }
                TokenKind::Question => {
                    // Error propagation postfix
                    // But only if not followed by something that makes it a ternary (which AG doesn't have)
                    // Check binding power
                    if 24 < min_bp {
                        break;
                    }
                    let span = self.current_span();
                    self.advance();
                    lhs = Expr::ErrorPropagate(Box::new(ErrorPropagateExpr {
                        expr: lhs,
                        span,
                    }));
                    continue;
                }
                _ => {}
            }

            // Infix operators with binding power
            let (op_bp, assoc) = match self.peek() {
                TokenKind::Eq => (2, Assoc::Right),
                TokenKind::PlusEq | TokenKind::MinusEq | TokenKind::StarEq | TokenKind::SlashEq => {
                    (2, Assoc::Right)
                }
                TokenKind::PipeGt => (4, Assoc::Left),
                TokenKind::QuestionQuestion => (6, Assoc::Left),
                TokenKind::PipePipe => (8, Assoc::Left),
                TokenKind::AmpAmp => (10, Assoc::Left),
                TokenKind::EqEq | TokenKind::BangEq => (12, Assoc::Left),
                TokenKind::Lt | TokenKind::Gt | TokenKind::LtEq | TokenKind::GtEq => {
                    (14, Assoc::Left)
                }
                TokenKind::Plus | TokenKind::Minus => (16, Assoc::Left),
                TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (18, Assoc::Left),
                TokenKind::StarStar => (20, Assoc::Right),
                _ => break,
            };

            if op_bp < min_bp {
                break;
            }

            let next_bp = match assoc {
                Assoc::Left => op_bp + 1,
                Assoc::Right => op_bp,
            };

            let op_span = self.current_span();
            let op_tok = self.advance().clone();

            // Handle assignment operators
            match &op_tok.kind {
                TokenKind::Eq => {
                    let rhs = self.parse_expr(next_bp)?;
                    lhs = Expr::Assign(Box::new(AssignExpr {
                        target: lhs,
                        value: rhs,
                        op: AssignOp::Assign,
                        span: op_span,
                    }));
                    continue;
                }
                TokenKind::PlusEq => {
                    let rhs = self.parse_expr(next_bp)?;
                    lhs = Expr::Assign(Box::new(AssignExpr {
                        target: lhs,
                        value: rhs,
                        op: AssignOp::AddAssign,
                        span: op_span,
                    }));
                    continue;
                }
                TokenKind::MinusEq => {
                    let rhs = self.parse_expr(next_bp)?;
                    lhs = Expr::Assign(Box::new(AssignExpr {
                        target: lhs,
                        value: rhs,
                        op: AssignOp::SubAssign,
                        span: op_span,
                    }));
                    continue;
                }
                TokenKind::StarEq => {
                    let rhs = self.parse_expr(next_bp)?;
                    lhs = Expr::Assign(Box::new(AssignExpr {
                        target: lhs,
                        value: rhs,
                        op: AssignOp::MulAssign,
                        span: op_span,
                    }));
                    continue;
                }
                TokenKind::SlashEq => {
                    let rhs = self.parse_expr(next_bp)?;
                    lhs = Expr::Assign(Box::new(AssignExpr {
                        target: lhs,
                        value: rhs,
                        op: AssignOp::DivAssign,
                        span: op_span,
                    }));
                    continue;
                }
                _ => {}
            }

            let rhs = self.parse_expr(next_bp)?;

            // Handle pipe operator
            if op_tok.kind == TokenKind::PipeGt {
                lhs = Expr::Pipe(Box::new(PipeExpr {
                    left: lhs,
                    right: rhs,
                    span: op_span,
                }));
                continue;
            }

            // Handle nullish coalescing
            if op_tok.kind == TokenKind::QuestionQuestion {
                lhs = Expr::NullishCoalesce(Box::new(NullishCoalesceExpr {
                    left: lhs,
                    right: rhs,
                    span: op_span,
                }));
                continue;
            }

            let op = match op_tok.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                TokenKind::StarStar => BinaryOp::Pow,
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::BangEq => BinaryOp::Ne,
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::LtEq => BinaryOp::Le,
                TokenKind::GtEq => BinaryOp::Ge,
                TokenKind::AmpAmp => BinaryOp::And,
                TokenKind::PipePipe => BinaryOp::Or,
                _ => unreachable!(),
            };

            lhs = Expr::Binary(BinaryExpr {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span: op_span,
            });
        }

        Some(lhs)
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        match self.peek() {
            TokenKind::Bang => {
                let span = self.current_span();
                self.advance();
                let operand = self.parse_expr(22)?; // Unary bp
                Some(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                }))
            }
            TokenKind::Minus => {
                let span = self.current_span();
                self.advance();
                let operand = self.parse_expr(22)?;
                Some(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                }))
            }
            TokenKind::Await => {
                let span = self.current_span();
                self.advance();
                let expr = self.parse_expr(22)?;
                Some(Expr::Await(Box::new(AwaitExpr { expr, span })))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let start = self.current_span();
        match self.peek().clone() {
            TokenKind::IntLiteral(s) => {
                self.advance();
                let val: i64 = s.parse().unwrap_or(0);
                Some(Expr::Literal(Literal::Int(val, start)))
            }
            TokenKind::FloatLiteral(s) => {
                self.advance();
                let val: f64 = s.parse().unwrap_or(0.0);
                Some(Expr::Literal(Literal::Float(val, start)))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Some(Expr::Literal(Literal::String(s, start)))
            }
            TokenKind::True => {
                self.advance();
                Some(Expr::Literal(Literal::Bool(true, start)))
            }
            TokenKind::False => {
                self.advance();
                Some(Expr::Literal(Literal::Bool(false, start)))
            }
            TokenKind::Nil => {
                self.advance();
                Some(Expr::Literal(Literal::Nil(start)))
            }
            TokenKind::Underscore => {
                self.advance();
                Some(Expr::Placeholder(start))
            }
            TokenKind::Ident(_) => {
                let tok = self.advance().clone();
                if let TokenKind::Ident(name) = tok.kind {
                    Some(Expr::Ident(Ident {
                        name,
                        span: tok.span,
                    }))
                } else {
                    None
                }
            }
            TokenKind::LParen => {
                // Could be grouped expression or arrow function
                // Heuristic: if we see (ident: or (ident, or (), it's likely arrow params
                self.advance(); // consume '('

                // Empty parens: () => ... is an arrow function
                if matches!(self.peek(), TokenKind::RParen) {
                    let saved = self.pos;
                    self.advance(); // consume ')'
                    if matches!(self.peek(), TokenKind::FatArrow) {
                        self.advance(); // consume '=>'
                        return self.parse_arrow_body(Vec::new(), start);
                    }
                    // Not an arrow — backtrack (rare case of empty parens as expr)
                    self.pos = saved;
                    self.advance(); // consume ')' again
                    // Return nil for empty grouping
                    return Some(Expr::Literal(Literal::Nil(start)));
                }

                // Try to detect arrow function: (ident: type, ...) =>
                let saved_pos = self.pos;
                if let Some(params) = self.try_parse_arrow_params() {
                    if matches!(self.peek(), TokenKind::FatArrow) {
                        self.advance(); // consume '=>'
                        return self.parse_arrow_body(params, start);
                    }
                }
                // Backtrack — it's a grouped expression
                self.pos = saved_pos;
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Some(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !matches!(self.peek(), TokenKind::RBracket | TokenKind::Eof) {
                    elements.push(self.parse_expr(0)?);
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                let end = self.current_span();
                Some(Expr::Array(ArrayExpr {
                    elements,
                    span: Span::new(start.start, end.end),
                }))
            }
            TokenKind::LBrace => {
                // Object literal { key: value, ... }
                // But also could be a block expr
                // Heuristic: if { <ident> : then it's an object
                let saved = self.pos;
                self.advance(); // consume '{'

                // Check for empty block
                if matches!(self.peek(), TokenKind::RBrace) {
                    self.advance();
                    let end = self.current_span();
                    return Some(Expr::Block(Box::new(Block {
                        stmts: Vec::new(),
                        tail_expr: None,
                        span: Span::new(start.start, end.end),
                    })));
                }

                // Try to detect object literal: { ident: expr }
                if let TokenKind::Ident(_) = self.peek() {
                    let saved2 = self.pos;
                    self.advance(); // consume ident
                    if matches!(self.peek(), TokenKind::Colon) {
                        // It's an object literal
                        self.pos = saved + 1; // back to after '{'
                        let mut fields = Vec::new();
                        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
                            let fstart = self.current_span();
                            let key = self.expect_ident()?;
                            self.expect(&TokenKind::Colon)?;
                            let value = self.parse_expr(0)?;
                            let fend = self.current_span();
                            fields.push(ObjectField {
                                key,
                                value,
                                span: Span::new(fstart.start, fend.end),
                            });
                            if matches!(self.peek(), TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(&TokenKind::RBrace)?;
                        let end = self.current_span();
                        return Some(Expr::Object(ObjectExpr {
                            fields,
                            span: Span::new(start.start, end.end),
                        }));
                    }
                    self.pos = saved2; // backtrack from ident peek
                }

                // It's a block
                self.pos = saved;
                let block = self.parse_block()?;
                Some(Expr::Block(Box::new(block)))
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::TemplateNoSub(s) => {
                let s = s.clone();
                self.advance();
                Some(Expr::TemplateString(TemplateStringExpr {
                    parts: vec![TemplatePart::String(s)],
                    span: start,
                }))
            }
            TokenKind::TemplateHead(s) => {
                let s = s.clone();
                self.advance();
                self.parse_template_string(s, start)
            }
            _ => {
                self.error(format!("unexpected token {:?}", self.peek()));
                None
            }
        }
    }

    fn try_parse_arrow_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
            let start = self.current_span();
            let name = if let TokenKind::Ident(_) = self.peek() {
                let tok = self.advance().clone();
                if let TokenKind::Ident(n) = tok.kind {
                    n
                } else {
                    return None;
                }
            } else {
                return None;
            };

            let ty = if matches!(self.peek(), TokenKind::Colon) {
                self.advance();
                self.parse_type()
            } else {
                None
            };

            let default = if matches!(self.peek(), TokenKind::Eq) {
                self.advance();
                self.parse_expr(0)
            } else {
                None
            };

            let end = self.current_span();
            params.push(Param {
                name,
                ty,
                default,
                is_variadic: false,
                span: Span::new(start.start, end.end),
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        if matches!(self.peek(), TokenKind::RParen) {
            self.advance();
            Some(params)
        } else {
            None
        }
    }

    fn parse_arrow_body(&mut self, params: Vec<Param>, start: Span) -> Option<Expr> {
        let body = if matches!(self.peek(), TokenKind::LBrace) {
            ArrowBody::Block(self.parse_block()?)
        } else {
            ArrowBody::Expr(self.parse_expr(0)?)
        };
        let end = self.current_span();
        Some(Expr::Arrow(Box::new(ArrowExpr {
            params,
            body,
            span: Span::new(start.start, end.end),
        })))
    }

    fn parse_if_expr(&mut self) -> Option<Expr> {
        let start = self.current_span();
        self.advance(); // consume 'if'
        let condition = self.parse_expr(0)?;
        let then_block = self.parse_block()?;
        let else_branch = if matches!(self.peek(), TokenKind::Else) {
            self.advance();
            if matches!(self.peek(), TokenKind::If) {
                let if_expr = self.parse_if_expr()?;
                if let Expr::If(if_box) = if_expr {
                    Some(ElseBranch::If(if_box))
                } else {
                    None
                }
            } else {
                Some(ElseBranch::Block(self.parse_block()?))
            }
        } else {
            None
        };
        let end = self.current_span();
        Some(Expr::If(Box::new(IfExpr {
            condition,
            then_block,
            else_branch,
            span: Span::new(start.start, end.end),
        })))
    }

    fn parse_match_expr(&mut self) -> Option<Expr> {
        let start = self.current_span();
        self.advance(); // consume 'match'
        let subject = self.parse_expr(0)?;
        self.expect(&TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let arm_start = self.current_span();
            let pattern = self.parse_pattern()?;
            let guard = if matches!(self.peek(), TokenKind::If) {
                self.advance();
                Some(self.parse_expr(0)?)
            } else {
                None
            };
            self.expect(&TokenKind::FatArrow)?;
            let body = self.parse_expr(0)?;
            let arm_end = self.current_span();
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: Span::new(arm_start.start, arm_end.end),
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(&TokenKind::RBrace)?;
        let end = self.current_span();
        Some(Expr::Match(Box::new(MatchExpr {
            subject,
            arms,
            span: Span::new(start.start, end.end),
        })))
    }

    fn parse_pattern(&mut self) -> Option<Pattern> {
        let start = self.current_span();
        match self.peek().clone() {
            TokenKind::IntLiteral(s) => {
                self.advance();
                let val: i64 = s.parse().unwrap_or(0);
                let mut pat = Pattern::Literal(Literal::Int(val, start));
                // Check for range pattern
                if matches!(self.peek(), TokenKind::DotDot) {
                    self.advance();
                    let end_expr = self.parse_expr(0)?;
                    let end_span = self.current_span();
                    pat = Pattern::Range(
                        Box::new(Expr::Literal(Literal::Int(val, start))),
                        Box::new(end_expr),
                        Span::new(start.start, end_span.end),
                    );
                }
                Some(pat)
            }
            TokenKind::FloatLiteral(s) => {
                self.advance();
                let val: f64 = s.parse().unwrap_or(0.0);
                Some(Pattern::Literal(Literal::Float(val, start)))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Some(Pattern::Literal(Literal::String(s, start)))
            }
            TokenKind::True => {
                self.advance();
                Some(Pattern::Literal(Literal::Bool(true, start)))
            }
            TokenKind::False => {
                self.advance();
                Some(Pattern::Literal(Literal::Bool(false, start)))
            }
            TokenKind::Nil => {
                self.advance();
                Some(Pattern::Literal(Literal::Nil(start)))
            }
            TokenKind::Underscore => {
                self.advance();
                Some(Pattern::Wildcard(start))
            }
            TokenKind::LBrace => {
                // Struct pattern { field, field2, ... }
                self.advance();
                let mut fields = Vec::new();
                while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
                    let f = self.expect_ident()?;
                    fields.push(f);
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                let end = self.current_span();
                Some(Pattern::Struct(StructPattern {
                    fields,
                    span: Span::new(start.start, end.end),
                }))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check for Enum::Variant pattern
                if matches!(self.peek(), TokenKind::ColonColon) {
                    self.advance();
                    let variant = self.expect_ident()?;
                    let bindings = if matches!(self.peek(), TokenKind::LParen) {
                        self.advance();
                        let mut binds = Vec::new();
                        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                            binds.push(self.expect_ident()?);
                            if matches!(self.peek(), TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(&TokenKind::RParen)?;
                        binds
                    } else {
                        Vec::new()
                    };
                    let end = self.current_span();
                    Some(Pattern::Enum(EnumPattern {
                        enum_name: name,
                        variant,
                        bindings,
                        span: Span::new(start.start, end.end),
                    }))
                } else {
                    // Check for range pattern
                    if matches!(self.peek(), TokenKind::DotDot) {
                        self.advance();
                        let end_expr = self.parse_expr(0)?;
                        let end_span = self.current_span();
                        Some(Pattern::Range(
                            Box::new(Expr::Ident(Ident {
                                name,
                                span: start,
                            })),
                            Box::new(end_expr),
                            Span::new(start.start, end_span.end),
                        ))
                    } else {
                        Some(Pattern::Ident(name, start))
                    }
                }
            }
            _ => {
                self.error("expected pattern");
                None
            }
        }
    }

    fn parse_template_string(&mut self, head: String, start: Span) -> Option<Expr> {
        let mut parts = Vec::new();
        if !head.is_empty() {
            parts.push(TemplatePart::String(head));
        }

        loop {
            // Parse the expression inside ${}
            let expr = self.parse_expr(0)?;
            parts.push(TemplatePart::Expr(expr));

            // After the expression, we should see TemplateTail or TemplateMiddle
            match self.peek().clone() {
                TokenKind::TemplateTail(s) => {
                    if !s.is_empty() {
                        parts.push(TemplatePart::String(s));
                    }
                    self.advance();
                    break;
                }
                TokenKind::TemplateMiddle(s) => {
                    if !s.is_empty() {
                        parts.push(TemplatePart::String(s));
                    }
                    self.advance();
                    continue;
                }
                _ => {
                    self.error("expected template continuation");
                    break;
                }
            }
        }

        let end = self.current_span();
        Some(Expr::TemplateString(TemplateStringExpr {
            parts,
            span: Span::new(start.start, end.end),
        }))
    }
}

enum Assoc {
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Module {
        let result = parse(src);
        assert!(
            result.diagnostics.is_empty(),
            "unexpected errors: {:?}",
            result.diagnostics
        );
        result.module
    }

    #[test]
    fn empty_module() {
        let m = parse_ok("");
        assert!(m.items.is_empty());
    }

    #[test]
    fn let_with_type() {
        let m = parse_ok(r#"let name: str = "Alice""#);
        assert!(matches!(m.items[0], Item::VarDecl(_)));
    }

    #[test]
    fn mut_without_type() {
        let m = parse_ok("mut counter = 0");
        if let Item::VarDecl(v) = &m.items[0] {
            assert_eq!(v.kind, VarKind::Mut);
            assert!(v.ty.is_none());
        }
    }

    #[test]
    fn const_decl() {
        let m = parse_ok("const MAX = 100");
        if let Item::VarDecl(v) = &m.items[0] {
            assert_eq!(v.kind, VarKind::Const);
        }
    }

    #[test]
    fn simple_function() {
        let m = parse_ok("fn add(a: int, b: int) -> int { a + b }");
        if let Item::FnDecl(f) = &m.items[0] {
            assert_eq!(f.name, "add");
            assert_eq!(f.params.len(), 2);
            assert!(f.return_type.is_some());
        }
    }

    #[test]
    fn pub_async_function() {
        let m = parse_ok("pub async fn fetch(url: str) -> str { url }");
        if let Item::FnDecl(f) = &m.items[0] {
            assert!(f.is_pub);
            assert!(f.is_async);
        }
    }

    #[test]
    fn function_with_default() {
        let m = parse_ok("fn greet(name: str, loud: bool = false) -> str { name }");
        if let Item::FnDecl(f) = &m.items[0] {
            assert!(f.params[1].default.is_some());
        }
    }

    #[test]
    fn arrow_function() {
        let m = parse_ok("let double = (x: int) => x * 2");
        if let Item::VarDecl(v) = &m.items[0] {
            assert!(matches!(v.init, Expr::Arrow(_)));
        }
    }

    #[test]
    fn struct_decl() {
        let m = parse_ok("struct User { name: str, age: int }");
        if let Item::StructDecl(s) = &m.items[0] {
            assert_eq!(s.name, "User");
            assert_eq!(s.fields.len(), 2);
        }
    }

    #[test]
    fn enum_decl() {
        let m = parse_ok("enum Status { Pending, Active(since: str), Error(code: int, msg: str) }");
        if let Item::EnumDecl(e) = &m.items[0] {
            assert_eq!(e.name, "Status");
            assert_eq!(e.variants.len(), 3);
            assert!(e.variants[0].fields.is_empty());
            assert_eq!(e.variants[1].fields.len(), 1);
            assert_eq!(e.variants[2].fields.len(), 2);
        }
    }

    #[test]
    fn type_alias() {
        let m = parse_ok("type ID = str");
        assert!(matches!(m.items[0], Item::TypeAlias(_)));
    }

    #[test]
    fn union_type_alias() {
        let m = parse_ok("type Result = str | Error");
        if let Item::TypeAlias(t) = &m.items[0] {
            assert!(matches!(t.ty, TypeExpr::Union(_, _, _)));
        }
    }

    #[test]
    fn arithmetic_precedence() {
        let m = parse_ok("let x = 1 + 2 * 3");
        if let Item::VarDecl(v) = &m.items[0] {
            if let Expr::Binary(b) = &v.init {
                assert_eq!(b.op, BinaryOp::Add);
                assert!(matches!(b.right.as_ref(), Expr::Binary(inner) if inner.op == BinaryOp::Mul));
            }
        }
    }

    #[test]
    fn pipe_operator() {
        let m = parse_ok("let x = data |> parse |> validate");
        if let Item::VarDecl(v) = &m.items[0] {
            assert!(matches!(v.init, Expr::Pipe(_)));
        }
    }

    #[test]
    fn if_else_expression() {
        let m = parse_ok("let x = if a > b { a } else { b }");
        if let Item::VarDecl(v) = &m.items[0] {
            assert!(matches!(v.init, Expr::If(_)));
        }
    }

    #[test]
    fn for_in_loop() {
        let result = parse("for item in items { process(item) }");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn while_loop() {
        let result = parse("fn f() { while x > 0 { x = x - 1 } }");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn match_with_guard() {
        let m = parse_ok(r#"let x = match n { 0 => "zero", n if n > 100 => "big", _ => "other" }"#);
        if let Item::VarDecl(v) = &m.items[0] {
            if let Expr::Match(m) = &v.init {
                assert_eq!(m.arms.len(), 3);
                assert!(m.arms[1].guard.is_some());
            }
        }
    }

    #[test]
    fn try_catch() {
        let result = parse("fn f() { try { parse(input) } catch e { log(e) } }");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn named_imports() {
        let m = parse_ok(r#"import { read, write } from "./fs""#);
        if let Item::Import(i) = &m.items[0] {
            assert_eq!(i.names.len(), 2);
            assert!(i.namespace.is_none());
        }
    }

    #[test]
    fn namespace_import() {
        let m = parse_ok(r#"import * as fs from "./fs""#);
        if let Item::Import(i) = &m.items[0] {
            assert_eq!(i.namespace.as_deref(), Some("fs"));
        }
    }

    #[test]
    fn implicit_return() {
        let m = parse_ok("fn foo() -> int { let x = 1; x + 1 }");
        if let Item::FnDecl(f) = &m.items[0] {
            assert!(f.body.tail_expr.is_some());
        }
    }

    #[test]
    fn explicit_semi_suppresses_return() {
        let m = parse_ok("fn foo() { do_something(); }");
        if let Item::FnDecl(f) = &m.items[0] {
            assert!(f.body.tail_expr.is_none());
        }
    }

    #[test]
    fn ret_with_value() {
        let m = parse_ok("fn foo() -> int { ret x + 1 }");
        if let Item::FnDecl(f) = &m.items[0] {
            if let Some(Stmt::Return(r)) = f.body.stmts.first() {
                assert!(r.value.is_some());
            }
        }
    }

    #[test]
    fn ret_without_value() {
        let m = parse_ok("fn foo() { ret }");
        if let Item::FnDecl(f) = &m.items[0] {
            if let Some(Stmt::Return(r)) = f.body.stmts.first() {
                assert!(r.value.is_none());
            }
        }
    }

    #[test]
    fn error_recovery_multiple() {
        let result = parse("fn foo() { !!! } fn bar() { ??? }");
        // Should produce some diagnostics but still parse both functions
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn mixed_top_level() {
        let m = parse_ok(
            r#"import { x } from "y"
let a = 1
fn foo() -> int { 42 }"#,
        );
        assert_eq!(m.items.len(), 3);
        assert!(matches!(m.items[0], Item::Import(_)));
        assert!(matches!(m.items[1], Item::VarDecl(_)));
        assert!(matches!(m.items[2], Item::FnDecl(_)));
    }

    #[test]
    fn template_string_parsing() {
        let m = parse_ok("let x = `hello ${name}!`");
        if let Item::VarDecl(v) = &m.items[0] {
            assert!(matches!(v.init, Expr::TemplateString(_)));
        }
    }

    // ── DSL block tests ──

    #[test]
    fn dsl_inline_block() {
        let m = parse_ok("@prompt system ```\nYou are helpful.\n```\n");
        assert_eq!(m.items.len(), 1);
        if let Item::DslBlock(dsl) = &m.items[0] {
            assert_eq!(dsl.kind, "prompt");
            assert_eq!(dsl.name.name, "system");
            if let DslContent::Inline { parts } = &dsl.content {
                assert_eq!(parts.len(), 1);
                assert!(matches!(&parts[0], DslPart::Text(t, _) if t == "You are helpful.\n"));
            } else {
                panic!("expected inline content");
            }
        } else {
            panic!("expected DslBlock");
        }
    }

    #[test]
    fn dsl_inline_with_capture() {
        let m = parse_ok("@prompt sys ```\nHello #{name}, you have #{count} messages.\n```\n");
        if let Item::DslBlock(dsl) = &m.items[0] {
            if let DslContent::Inline { parts } = &dsl.content {
                assert_eq!(parts.len(), 5);
                assert!(matches!(&parts[0], DslPart::Text(t, _) if t == "Hello "));
                assert!(matches!(&parts[1], DslPart::Capture(_, _)));
                assert!(matches!(&parts[2], DslPart::Text(t, _) if t == ", you have "));
                assert!(matches!(&parts[3], DslPart::Capture(_, _)));
                assert!(matches!(&parts[4], DslPart::Text(t, _) if t == " messages.\n"));
            } else {
                panic!("expected inline content");
            }
        } else {
            panic!("expected DslBlock");
        }
    }

    #[test]
    fn dsl_file_reference() {
        let m = parse_ok(r#"@component Button from "./button.tsx""#);
        if let Item::DslBlock(dsl) = &m.items[0] {
            assert_eq!(dsl.kind, "component");
            assert_eq!(dsl.name.name, "Button");
            if let DslContent::FileRef { path, .. } = &dsl.content {
                assert_eq!(path, "./button.tsx");
            } else {
                panic!("expected file ref content");
            }
        } else {
            panic!("expected DslBlock");
        }
    }

    #[test]
    fn dsl_unknown_kind_accepted() {
        let m = parse_ok("@graphql GetUsers ```\nquery { users { id } }\n```\n");
        if let Item::DslBlock(dsl) = &m.items[0] {
            assert_eq!(dsl.kind, "graphql");
            assert_eq!(dsl.name.name, "GetUsers");
        } else {
            panic!("expected DslBlock");
        }
    }

    #[test]
    fn dsl_mixed_with_other_items() {
        let m = parse_ok(
            r#"import { x } from "y"
@prompt sys ```
hello
```
fn foo() -> int { 1 }"#,
        );
        assert_eq!(m.items.len(), 3);
        assert!(matches!(m.items[0], Item::Import(_)));
        assert!(matches!(m.items[1], Item::DslBlock(_)));
        assert!(matches!(m.items[2], Item::FnDecl(_)));
    }

    #[test]
    fn dsl_missing_kind() {
        let result = parse("@42");
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn dsl_missing_name() {
        let result = parse("@prompt\nfn foo() {}");
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn dsl_missing_body() {
        let result = parse("@prompt system\nfn foo() {}");
        assert!(!result.diagnostics.is_empty());
    }

    // ── Extern declaration tests ──

    #[test]
    fn extern_fn_simple() {
        let m = parse_ok("extern fn fetch(url: str) -> Promise<any>");
        assert_eq!(m.items.len(), 1);
        if let Item::ExternFnDecl(ef) = &m.items[0] {
            assert_eq!(ef.name, "fetch");
            assert_eq!(ef.params.len(), 1);
            assert_eq!(ef.params[0].name, "url");
            assert!(!ef.variadic);
            assert!(ef.js_annotation.is_none());
        } else {
            panic!("expected ExternFnDecl");
        }
    }

    #[test]
    fn extern_fn_no_return_type() {
        let m = parse_ok("extern fn log(msg: str)");
        if let Item::ExternFnDecl(ef) = &m.items[0] {
            assert_eq!(ef.name, "log");
            assert!(ef.return_type.is_none());
        } else {
            panic!("expected ExternFnDecl");
        }
    }

    #[test]
    fn extern_fn_variadic() {
        let m = parse_ok("extern fn info(...args: any)");
        if let Item::ExternFnDecl(ef) = &m.items[0] {
            assert_eq!(ef.name, "info");
            assert!(ef.variadic);
            assert_eq!(ef.params.len(), 1);
            assert_eq!(ef.params[0].name, "args");
        } else {
            panic!("expected ExternFnDecl");
        }
    }

    #[test]
    fn extern_struct() {
        let m = parse_ok("extern struct Response {\n    status: num,\n    fn json() -> any\n}");
        if let Item::ExternStructDecl(es) = &m.items[0] {
            assert_eq!(es.name, "Response");
            assert_eq!(es.fields.len(), 1);
            assert_eq!(es.fields[0].name, "status");
            assert_eq!(es.methods.len(), 1);
            assert_eq!(es.methods[0].name, "json");
        } else {
            panic!("expected ExternStructDecl");
        }
    }

    #[test]
    fn extern_type_simple() {
        let m = parse_ok("extern type Headers");
        if let Item::ExternTypeDecl(et) = &m.items[0] {
            assert_eq!(et.name, "Headers");
            assert!(et.js_annotation.is_none());
        } else {
            panic!("expected ExternTypeDecl");
        }
    }

    #[test]
    fn js_annotation_module_only() {
        let m = parse_ok("@js(\"node:fs\")\nextern fn readFile(path: str) -> Promise<str>");
        if let Item::ExternFnDecl(ef) = &m.items[0] {
            assert_eq!(ef.name, "readFile");
            let ann = ef.js_annotation.as_ref().unwrap();
            assert_eq!(ann.module, Some("node:fs".to_string()));
            assert!(ann.js_name.is_none());
        } else {
            panic!("expected ExternFnDecl");
        }
    }

    #[test]
    fn js_annotation_with_name() {
        let m = parse_ok("@js(\"my-lib\", name = \"doWork\")\nextern fn do_work(input: str) -> str");
        if let Item::ExternFnDecl(ef) = &m.items[0] {
            let ann = ef.js_annotation.as_ref().unwrap();
            assert_eq!(ann.module, Some("my-lib".to_string()));
            assert_eq!(ann.js_name, Some("doWork".to_string()));
        } else {
            panic!("expected ExternFnDecl");
        }
    }

    #[test]
    fn promise_type_parsing() {
        let m = parse_ok("extern fn load(url: str) -> Promise<str>");
        if let Item::ExternFnDecl(ef) = &m.items[0] {
            assert!(matches!(ef.return_type.as_ref().unwrap(), TypeExpr::Promise(..)));
        } else {
            panic!("expected ExternFnDecl");
        }
    }

    #[test]
    fn promise_nested() {
        let m = parse_ok("extern fn load() -> Promise<Promise<str>>");
        if let Item::ExternFnDecl(ef) = &m.items[0] {
            if let TypeExpr::Promise(inner, _) = ef.return_type.as_ref().unwrap() {
                assert!(matches!(inner.as_ref(), TypeExpr::Promise(..)));
            } else {
                panic!("expected Promise type");
            }
        } else {
            panic!("expected ExternFnDecl");
        }
    }

    #[test]
    fn extern_mixed_with_regular() {
        let m = parse_ok("extern fn fetch(url: str) -> Promise<any>\nfn main() {\n    let x = 1\n}");
        assert_eq!(m.items.len(), 2);
        assert!(matches!(m.items[0], Item::ExternFnDecl(_)));
        assert!(matches!(m.items[1], Item::FnDecl(_)));
    }
}
