use std::any::Any;
use std::collections::HashMap;

use ag_ast::*;
use swc_common::sync::Lrc;
use swc_common::{SourceMap, SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_codegen::Emitter;

// ── Re-export DslHandler from ag-dsl-core ────────────────────

pub use ag_dsl_core::DslHandler;

#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
    pub span: Span,
}

/// Bridges the host compiler's expression translator to the DSL system.
pub struct AgCodegenContext;

impl ag_dsl_core::CodegenContext for AgCodegenContext {
    fn translate_expr(&mut self, expr: &dyn Any) -> swc::Expr {
        if let Some(ag_expr) = expr.downcast_ref::<ag_ast::Expr>() {
            translate_expr(ag_expr)
        } else {
            swc::Expr::Ident(ident("undefined"))
        }
    }
}

/// Convert an ag-ast DslBlock to an ag-dsl-core DslBlock for handler dispatch.
fn convert_dsl_block(dsl: &ag_ast::DslBlock) -> ag_dsl_core::DslBlock {
    let content = match &dsl.content {
        ag_ast::DslContent::Inline { parts } => {
            let core_parts: Vec<ag_dsl_core::DslPart> = parts
                .iter()
                .map(|p| match p {
                    ag_ast::DslPart::Text(s, span) => ag_dsl_core::DslPart::Text(
                        s.clone(),
                        ag_dsl_core::Span::new(span.start, span.end),
                    ),
                    ag_ast::DslPart::Capture(expr, span) => {
                        // Clone the inner Expr (not the Box) for type erasure
                        let boxed: Box<dyn Any> = Box::new((**expr).clone());
                        ag_dsl_core::DslPart::Capture(
                            boxed,
                            ag_dsl_core::Span::new(span.start, span.end),
                        )
                    }
                })
                .collect();
            ag_dsl_core::DslContent::Inline { parts: core_parts }
        }
        ag_ast::DslContent::FileRef { path, span } => ag_dsl_core::DslContent::FileRef {
            path: path.clone(),
            span: ag_dsl_core::Span::new(span.start, span.end),
        },
    };

    ag_dsl_core::DslBlock {
        kind: dsl.kind.clone(),
        name: dsl.name.name.clone(),
        content,
        span: ag_dsl_core::Span::new(dsl.span.start, dsl.span.end),
    }
}

// ── Translator with handler registry ──────────────────────

pub struct Translator {
    handlers: HashMap<String, Box<dyn ag_dsl_core::DslHandler>>,
}

impl Translator {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register_dsl_handler(&mut self, kind: &str, handler: Box<dyn ag_dsl_core::DslHandler>) {
        self.handlers.insert(kind.to_string(), handler);
    }

    pub fn codegen(&self, module: &Module) -> Result<String, CodegenError> {
        let swc_module = self.translate_module(module)?;
        Ok(emit(&swc_module))
    }

    fn translate_module(&self, module: &Module) -> Result<swc::Module, CodegenError> {
        // First pass: collect @js extern declarations
        let mut js_externs: HashMap<String, JsExternInfo> = HashMap::new();
        for item in &module.items {
            match item {
                Item::ExternFnDecl(ef) => {
                    if let Some(ref ann) = ef.js_annotation {
                        if let Some(ref module_name) = ann.module {
                            js_externs.insert(ef.name.clone(), JsExternInfo {
                                module: module_name.clone(),
                                js_name: ann.js_name.clone(),
                            });
                        }
                    }
                }
                Item::ExternStructDecl(es) => {
                    if let Some(ref ann) = es.js_annotation {
                        if let Some(ref module_name) = ann.module {
                            js_externs.insert(es.name.clone(), JsExternInfo {
                                module: module_name.clone(),
                                js_name: ann.js_name.clone(),
                            });
                        }
                    }
                }
                Item::ExternTypeDecl(et) => {
                    if let Some(ref ann) = et.js_annotation {
                        if let Some(ref module_name) = ann.module {
                            js_externs.insert(et.name.clone(), JsExternInfo {
                                module: module_name.clone(),
                                js_name: ann.js_name.clone(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Collect referenced identifiers
        let mut referenced = std::collections::HashSet::new();
        for item in &module.items {
            collect_referenced_idents(item, &mut referenced);
        }

        // Generate import statements for referenced @js externs, grouped by module
        let mut module_imports: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
        for (ag_name, info) in &js_externs {
            if referenced.contains(ag_name) {
                let entry = module_imports.entry(info.module.clone()).or_default();
                entry.push((ag_name.clone(), info.js_name.clone()));
            }
        }

        let mut body = Vec::new();

        // Emit merged import statements at the top
        let mut sorted_modules: Vec<_> = module_imports.keys().cloned().collect();
        sorted_modules.sort();
        for module_path in sorted_modules {
            let names = &module_imports[&module_path];
            let specifiers: Vec<swc::ImportSpecifier> = names.iter().map(|(ag_name, js_name)| {
                swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: ident(ag_name),
                    imported: js_name.as_ref().map(|jn| {
                        swc::ModuleExportName::Ident(ident(jn))
                    }),
                    is_type_only: false,
                })
            }).collect();
            body.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(
                swc::ImportDecl {
                    span: DUMMY_SP,
                    specifiers,
                    src: Box::new(swc::Str {
                        span: DUMMY_SP,
                        value: module_path.into(),
                        raw: None,
                    }),
                    type_only: false,
                    with: None,
                    phase: Default::default(),
                },
            )));
        }

        // Second pass: translate items
        for item in &module.items {
            match item {
                Item::DslBlock(dsl) => {
                    if let Some(handler) = self.handlers.get(&dsl.kind) {
                        let mut ctx = AgCodegenContext;
                        let core_block = convert_dsl_block(dsl);
                        let items = handler.handle(&core_block, &mut ctx).map_err(|e| {
                            CodegenError {
                                message: e.message,
                                span: dsl.span,
                            }
                        })?;
                        body.extend(items);
                    } else {
                        return Err(CodegenError {
                            message: format!(
                                "no handler registered for DSL kind `{}`",
                                dsl.kind
                            ),
                            span: dsl.span,
                        });
                    }
                }
                other => {
                    translate_item_into(other, &mut body);
                }
            }
        }

        Ok(swc::Module {
            span: DUMMY_SP,
            body,
            shebang: None,
        })
    }
}

struct JsExternInfo {
    module: String,
    js_name: Option<String>,
}

fn collect_referenced_idents(item: &Item, set: &mut std::collections::HashSet<String>) {
    match item {
        Item::FnDecl(f) => collect_idents_block(&f.body, set),
        Item::VarDecl(v) => collect_idents_expr(&v.init, set),
        Item::ExprStmt(e) => collect_idents_expr(&e.expr, set),
        Item::DslBlock(dsl) => {
            if let DslContent::Inline { parts } = &dsl.content {
                for part in parts {
                    if let DslPart::Capture(expr, _) = part {
                        collect_idents_expr(expr, set);
                    }
                }
            }
        }
        _ => {}
    }
}

fn collect_idents_expr(expr: &Expr, set: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::Ident(id) => { set.insert(id.name.clone()); }
        Expr::Binary(b) => { collect_idents_expr(&b.left, set); collect_idents_expr(&b.right, set); }
        Expr::Unary(u) => collect_idents_expr(&u.operand, set),
        Expr::Call(c) => {
            collect_idents_expr(&c.callee, set);
            for a in &c.args { collect_idents_expr(a, set); }
        }
        Expr::Member(m) => collect_idents_expr(&m.object, set),
        Expr::Index(i) => { collect_idents_expr(&i.object, set); collect_idents_expr(&i.index, set); }
        Expr::If(if_expr) => {
            collect_idents_expr(&if_expr.condition, set);
            collect_idents_block(&if_expr.then_block, set);
            if let Some(ref eb) = if_expr.else_branch {
                match eb {
                    ElseBranch::Block(b) => collect_idents_block(b, set),
                    ElseBranch::If(nested) => collect_idents_expr(&Expr::If(nested.clone()), set),
                }
            }
        }
        Expr::Match(m) => {
            collect_idents_expr(&m.subject, set);
            for arm in &m.arms {
                collect_idents_expr(&arm.body, set);
                if let Some(ref g) = arm.guard { collect_idents_expr(g, set); }
            }
        }
        Expr::Block(b) => collect_idents_block(b, set),
        Expr::Array(a) => { for e in &a.elements { collect_idents_expr(e, set); } }
        Expr::Object(o) => { for f in &o.fields { collect_idents_expr(&f.value, set); } }
        Expr::Arrow(ar) => {
            match &ar.body {
                ArrowBody::Expr(e) => collect_idents_expr(e, set),
                ArrowBody::Block(b) => collect_idents_block(b, set),
            }
        }
        Expr::Pipe(p) => { collect_idents_expr(&p.left, set); collect_idents_expr(&p.right, set); }
        Expr::OptionalChain(oc) => collect_idents_expr(&oc.object, set),
        Expr::NullishCoalesce(nc) => { collect_idents_expr(&nc.left, set); collect_idents_expr(&nc.right, set); }
        Expr::Await(a) => collect_idents_expr(&a.expr, set),
        Expr::ErrorPropagate(ep) => collect_idents_expr(&ep.expr, set),
        Expr::Assign(a) => { collect_idents_expr(&a.target, set); collect_idents_expr(&a.value, set); }
        Expr::TemplateString(ts) => {
            for p in &ts.parts {
                if let TemplatePart::Expr(e) = p { collect_idents_expr(e, set); }
            }
        }
        _ => {}
    }
}

fn collect_idents_block(block: &Block, set: &mut std::collections::HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::VarDecl(v) => collect_idents_expr(&v.init, set),
            Stmt::ExprStmt(e) => collect_idents_expr(&e.expr, set),
            Stmt::Return(r) => { if let Some(ref v) = r.value { collect_idents_expr(v, set); } }
            Stmt::If(i) => collect_idents_expr(&Expr::If(Box::new(i.clone())), set),
            Stmt::For(f) => { collect_idents_expr(&f.iter, set); collect_idents_block(&f.body, set); }
            Stmt::While(w) => { collect_idents_expr(&w.condition, set); collect_idents_block(&w.body, set); }
            Stmt::Match(m) => collect_idents_expr(&Expr::Match(Box::new(m.clone())), set),
            Stmt::TryCatch(tc) => { collect_idents_block(&tc.try_block, set); collect_idents_block(&tc.catch_block, set); }
        }
    }
    if let Some(ref tail) = block.tail_expr {
        collect_idents_expr(tail, set);
    }
}

// ── Legacy API (keeps existing code working) ──────────────

pub fn codegen(module: &Module) -> String {
    let mut translator = Translator::new();
    translator.register_dsl_handler(
        "prompt",
        Box::new(ag_dsl_prompt::handler::PromptDslHandler),
    );
    translator.codegen(module).unwrap_or_else(|e| {
        panic!("codegen error: {}", e.message)
    })
}

fn emit(module: &swc::Module) -> String {
    let cm: Lrc<SourceMap> = Lrc::new(SourceMap::default());
    let mut buf = Vec::new();
    {
        let mut emitter = Emitter {
            cfg: swc_ecma_codegen::Config::default(),
            cm: cm.clone(),
            comments: None,
            wr: JsWriter::new(cm, "\n", &mut buf, None),
        };
        emitter.emit_module(module).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

// ── Helpers ────────────────────────────────────────────────

fn ident(name: &str) -> swc::Ident {
    swc::Ident {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        sym: name.into(),
        optional: false,
    }
}

fn binding_ident(name: &str) -> swc::BindingIdent {
    swc::BindingIdent {
        id: ident(name),
        type_ann: None,
    }
}

fn expr_or_spread(expr: swc::Expr) -> swc::ExprOrSpread {
    swc::ExprOrSpread {
        spread: None,
        expr: Box::new(expr),
    }
}

fn stmt_to_module_item(stmt: swc::Stmt) -> swc::ModuleItem {
    swc::ModuleItem::Stmt(stmt)
}

// ── Module translation ─────────────────────────────────────

fn translate_item_into(item: &Item, body: &mut Vec<swc::ModuleItem>) {
    match item {
        Item::FnDecl(f) => {
            if f.is_pub {
                body.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::ExportDecl(
                    swc::ExportDecl {
                        span: DUMMY_SP,
                        decl: swc::Decl::Fn(translate_fn_decl(f)),
                    },
                )));
            } else {
                body.push(stmt_to_module_item(swc::Stmt::Decl(swc::Decl::Fn(
                    translate_fn_decl(f),
                ))));
            }
        }
        Item::VarDecl(v) => {
            body.push(stmt_to_module_item(translate_var_decl_stmt(v)));
        }
        Item::Import(imp) => {
            body.push(swc::ModuleItem::ModuleDecl(translate_import(imp)));
        }
        // Struct, Enum, TypeAlias, Extern declarations are erased
        Item::StructDecl(_) | Item::EnumDecl(_) | Item::TypeAlias(_)
        | Item::ExternFnDecl(_) | Item::ExternStructDecl(_) | Item::ExternTypeDecl(_) => {}
        Item::ExprStmt(e) => {
            body.push(stmt_to_module_item(swc::Stmt::Expr(swc::ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(translate_expr(&e.expr)),
            })));
        }
        Item::DslBlock(_) => {
            // Handled by Translator; legacy codegen() registers prompt handler
        }
    }
}

// ── Variable declarations ──────────────────────────────────

fn translate_var_decl_stmt(v: &VarDecl) -> swc::Stmt {
    let kind = match v.kind {
        VarKind::Let => swc::VarDeclKind::Const,
        VarKind::Mut => swc::VarDeclKind::Let,
        VarKind::Const => swc::VarDeclKind::Const,
    };

    swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind,
        declare: false,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name: swc::Pat::Ident(binding_ident(&v.name)),
            init: Some(Box::new(translate_expr(&v.init))),
            definite: false,
        }],
    })))
}

// ── Function declarations ──────────────────────────────────

fn translate_fn_decl(f: &FnDecl) -> swc::FnDecl {
    let params: Vec<swc::Param> = f
        .params
        .iter()
        .map(|p| {
            let pat = if let Some(ref default) = p.default {
                swc::Pat::Assign(swc::AssignPat {
                    span: DUMMY_SP,
                    left: Box::new(swc::Pat::Ident(binding_ident(&p.name))),
                    right: Box::new(translate_expr(default)),
                })
            } else {
                swc::Pat::Ident(binding_ident(&p.name))
            };
            swc::Param {
                span: DUMMY_SP,
                decorators: Vec::new(),
                pat,
            }
        })
        .collect();

    let body = translate_block_with_implicit_return(&f.body);

    swc::FnDecl {
        ident: ident(&f.name),
        declare: false,
        function: Box::new(swc::Function {
            params,
            decorators: Vec::new(),
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            body: Some(body),
            is_generator: false,
            is_async: f.is_async,
            type_params: None,
            return_type: None,
        }),
    }
}

// ── Block translation ──────────────────────────────────────

fn translate_block(block: &Block) -> swc::BlockStmt {
    let mut stmts = Vec::new();
    for stmt in &block.stmts {
        stmts.push(translate_stmt(stmt));
    }
    if let Some(ref tail) = block.tail_expr {
        stmts.push(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(translate_expr(tail)),
        }));
    }
    swc::BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts,
    }
}

fn translate_block_with_implicit_return(block: &Block) -> swc::BlockStmt {
    let mut stmts = Vec::new();
    for stmt in &block.stmts {
        stmts.push(translate_stmt(stmt));
    }
    if let Some(ref tail) = block.tail_expr {
        stmts.push(swc::Stmt::Return(swc::ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(translate_expr(tail))),
        }));
    }
    swc::BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts,
    }
}

// ── Statement translation ──────────────────────────────────

fn translate_stmt(stmt: &Stmt) -> swc::Stmt {
    match stmt {
        Stmt::VarDecl(v) => translate_var_decl_stmt(v),
        Stmt::ExprStmt(e) => swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(translate_expr(&e.expr)),
        }),
        Stmt::Return(r) => swc::Stmt::Return(swc::ReturnStmt {
            span: DUMMY_SP,
            arg: r.value.as_ref().map(|v| Box::new(translate_expr(v))),
        }),
        Stmt::If(if_expr) => translate_if_stmt(if_expr),
        Stmt::For(f) => swc::Stmt::ForOf(swc::ForOfStmt {
            span: DUMMY_SP,
            is_await: false,
            left: swc::ForHead::VarDecl(Box::new(swc::VarDecl {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                kind: swc::VarDeclKind::Const,
                declare: false,
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: swc::Pat::Ident(binding_ident(&f.binding)),
                    init: None,
                    definite: false,
                }],
            })),
            right: Box::new(translate_expr(&f.iter)),
            body: Box::new(swc::Stmt::Block(translate_block(&f.body))),
        }),
        Stmt::While(w) => swc::Stmt::While(swc::WhileStmt {
            span: DUMMY_SP,
            test: Box::new(translate_expr(&w.condition)),
            body: Box::new(swc::Stmt::Block(translate_block(&w.body))),
        }),
        Stmt::Match(m) => {
            let expr = translate_match(m);
            swc::Stmt::Expr(swc::ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(expr),
            })
        }
        Stmt::TryCatch(tc) => swc::Stmt::Try(Box::new(swc::TryStmt {
            span: DUMMY_SP,
            block: translate_block(&tc.try_block),
            handler: Some(swc::CatchClause {
                span: DUMMY_SP,
                param: Some(swc::Pat::Ident(binding_ident(&tc.catch_binding))),
                body: translate_block(&tc.catch_block),
            }),
            finalizer: None,
        })),
    }
}

fn translate_if_stmt(if_expr: &IfExpr) -> swc::Stmt {
    let alt = if_expr.else_branch.as_ref().map(|eb| {
        Box::new(match eb {
            ElseBranch::Block(b) => swc::Stmt::Block(translate_block(b)),
            ElseBranch::If(nested) => translate_if_stmt(nested),
        })
    });

    swc::Stmt::If(swc::IfStmt {
        span: DUMMY_SP,
        test: Box::new(translate_expr(&if_expr.condition)),
        cons: Box::new(swc::Stmt::Block(translate_block(&if_expr.then_block))),
        alt,
    })
}

// ── Expression translation ─────────────────────────────────

fn translate_expr(expr: &Expr) -> swc::Expr {
    match expr {
        Expr::Literal(lit) => translate_literal(lit),
        Expr::Ident(id) => swc::Expr::Ident(ident(&id.name)),
        Expr::Binary(b) => translate_binary(b),
        Expr::Unary(u) => translate_unary(u),
        Expr::Call(c) => translate_call(c),
        Expr::Member(m) => translate_member(m),
        Expr::Index(i) => swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(translate_expr(&i.object)),
            prop: swc::MemberProp::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(translate_expr(&i.index)),
            }),
        }),
        Expr::If(if_expr) => {
            // Translate as ternary if simple, else IIFE
            if let Some(ref else_branch) = if_expr.else_branch {
                let alt_expr = match else_branch {
                    ElseBranch::Block(b) => block_to_expr(b),
                    ElseBranch::If(nested) => translate_expr(&Expr::If(nested.clone())),
                };
                swc::Expr::Cond(swc::CondExpr {
                    span: DUMMY_SP,
                    test: Box::new(translate_expr(&if_expr.condition)),
                    cons: Box::new(block_to_expr(&if_expr.then_block)),
                    alt: Box::new(alt_expr),
                })
            } else {
                // IIFE for if without else
                let body = translate_block_with_implicit_return(&if_expr.then_block);
                let if_stmt = swc::Stmt::If(swc::IfStmt {
                    span: DUMMY_SP,
                    test: Box::new(translate_expr(&if_expr.condition)),
                    cons: Box::new(swc::Stmt::Block(body)),
                    alt: None,
                });
                make_iife(vec![if_stmt])
            }
        }
        Expr::Match(m) => translate_match(m),
        Expr::Block(b) => block_to_expr(b),
        Expr::Array(arr) => swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems: arr
                .elements
                .iter()
                .map(|e| Some(expr_or_spread(translate_expr(e))))
                .collect(),
        }),
        Expr::Object(obj) => swc::Expr::Object(swc::ObjectLit {
            span: DUMMY_SP,
            props: obj
                .fields
                .iter()
                .map(|f| {
                    swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
                        key: swc::PropName::Ident(swc::IdentName {
                            span: DUMMY_SP,
                            sym: f.key.clone().into(),
                        }),
                        value: Box::new(translate_expr(&f.value)),
                    })))
                })
                .collect(),
        }),
        Expr::Arrow(arrow) => translate_arrow(arrow),
        Expr::Pipe(p) => translate_pipe(p),
        Expr::OptionalChain(oc) => swc::Expr::OptChain(swc::OptChainExpr {
            span: DUMMY_SP,
            optional: true,
            base: Box::new(swc::OptChainBase::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(translate_expr(&oc.object)),
                prop: swc::MemberProp::Ident(swc::IdentName {
                    span: DUMMY_SP,
                    sym: oc.field.clone().into(),
                }),
            })),
        }),
        Expr::NullishCoalesce(nc) => swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::NullishCoalescing,
            left: Box::new(translate_expr(&nc.left)),
            right: Box::new(translate_expr(&nc.right)),
        }),
        Expr::Await(a) => swc::Expr::Await(swc::AwaitExpr {
            span: DUMMY_SP,
            arg: Box::new(translate_expr(&a.expr)),
        }),
        Expr::ErrorPropagate(ep) => translate_error_propagate(ep),
        Expr::Assign(assign) => translate_assign(assign),
        Expr::TemplateString(ts) => translate_template_string(ts),
        Expr::Placeholder(_) => swc::Expr::Ident(ident("undefined")),
    }
}

fn translate_literal(lit: &Literal) -> swc::Expr {
    match lit {
        Literal::Int(val, _) => swc::Expr::Lit(swc::Lit::Num(swc::Number {
            span: DUMMY_SP,
            value: *val as f64,
            raw: None,
        })),
        Literal::Float(val, _) => swc::Expr::Lit(swc::Lit::Num(swc::Number {
            span: DUMMY_SP,
            value: *val,
            raw: None,
        })),
        Literal::String(s, _) => swc::Expr::Lit(swc::Lit::Str(swc::Str {
            span: DUMMY_SP,
            value: s.clone().into(),
            raw: None,
        })),
        Literal::Bool(b, _) => swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
            span: DUMMY_SP,
            value: *b,
        })),
        Literal::Nil(_) => swc::Expr::Lit(swc::Lit::Null(swc::Null { span: DUMMY_SP })),
    }
}

fn translate_binary(b: &BinaryExpr) -> swc::Expr {
    let op = match b.op {
        BinaryOp::Add => swc::BinaryOp::Add,
        BinaryOp::Sub => swc::BinaryOp::Sub,
        BinaryOp::Mul => swc::BinaryOp::Mul,
        BinaryOp::Div => swc::BinaryOp::Div,
        BinaryOp::Mod => swc::BinaryOp::Mod,
        BinaryOp::Pow => swc::BinaryOp::Exp,
        BinaryOp::Eq => swc::BinaryOp::EqEqEq,
        BinaryOp::Ne => swc::BinaryOp::NotEqEq,
        BinaryOp::Lt => swc::BinaryOp::Lt,
        BinaryOp::Gt => swc::BinaryOp::Gt,
        BinaryOp::Le => swc::BinaryOp::LtEq,
        BinaryOp::Ge => swc::BinaryOp::GtEq,
        BinaryOp::And => swc::BinaryOp::LogicalAnd,
        BinaryOp::Or => swc::BinaryOp::LogicalOr,
    };
    swc::Expr::Bin(swc::BinExpr {
        span: DUMMY_SP,
        op,
        left: Box::new(translate_expr(&b.left)),
        right: Box::new(translate_expr(&b.right)),
    })
}

fn translate_unary(u: &UnaryExpr) -> swc::Expr {
    let op = match u.op {
        UnaryOp::Not => swc::UnaryOp::Bang,
        UnaryOp::Neg => swc::UnaryOp::Minus,
    };
    swc::Expr::Unary(swc::UnaryExpr {
        span: DUMMY_SP,
        op,
        arg: Box::new(translate_expr(&u.operand)),
    })
}

fn translate_call(c: &CallExpr) -> swc::Expr {
    swc::Expr::Call(swc::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: swc::Callee::Expr(Box::new(translate_expr(&c.callee))),
        args: c.args.iter().map(|a| expr_or_spread(translate_expr(a))).collect(),
        type_args: None,
    })
}

fn translate_member(m: &MemberExpr) -> swc::Expr {
    // Check if this is an enum variant construction: Enum::Variant or Enum::Variant(...)
    // We detect this pattern: Member { object: Ident(EnumName), field: VariantName }
    // For now, just do regular member access
    swc::Expr::Member(swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(translate_expr(&m.object)),
        prop: swc::MemberProp::Ident(swc::IdentName {
            span: DUMMY_SP,
            sym: m.field.clone().into(),
        }),
    })
}

fn translate_arrow(arrow: &ArrowExpr) -> swc::Expr {
    let params: Vec<swc::Pat> = arrow
        .params
        .iter()
        .map(|p| swc::Pat::Ident(binding_ident(&p.name)))
        .collect();

    let body = match &arrow.body {
        ArrowBody::Expr(e) => {
            swc::BlockStmtOrExpr::Expr(Box::new(translate_expr(e)))
        }
        ArrowBody::Block(b) => {
            swc::BlockStmtOrExpr::BlockStmt(translate_block_with_implicit_return(b))
        }
    };

    swc::Expr::Arrow(swc::ArrowExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        params,
        body: Box::new(body),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
    })
}

fn translate_pipe(p: &PipeExpr) -> swc::Expr {
    let left = translate_expr(&p.left);

    // Check if right side is a call with placeholder
    match &p.right {
        Expr::Call(call) => {
            // Replace Placeholder args with the piped value
            let has_placeholder = call.args.iter().any(|a| matches!(a, Expr::Placeholder(_)));
            if has_placeholder {
                let args: Vec<swc::ExprOrSpread> = call
                    .args
                    .iter()
                    .map(|a| {
                        if matches!(a, Expr::Placeholder(_)) {
                            expr_or_spread(left.clone())
                        } else {
                            expr_or_spread(translate_expr(a))
                        }
                    })
                    .collect();
                swc::Expr::Call(swc::CallExpr {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    callee: swc::Callee::Expr(Box::new(translate_expr(&call.callee))),
                    args,
                    type_args: None,
                })
            } else {
                // a |> f(x) → f(x) with a prepended? No — a |> f means f(a)
                // But here it's a |> f(x) which should be f(a, x)? Actually no.
                // Per spec: a |> f → f(a), a |> f(_, x) → f(a, x)
                // If there's no placeholder, it's not a call form, just wrap
                swc::Expr::Call(swc::CallExpr {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    callee: swc::Callee::Expr(Box::new(translate_expr(&p.right))),
                    args: vec![expr_or_spread(left)],
                    type_args: None,
                })
            }
        }
        Expr::Ident(_) => {
            // a |> f → f(a)
            swc::Expr::Call(swc::CallExpr {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                callee: swc::Callee::Expr(Box::new(translate_expr(&p.right))),
                args: vec![expr_or_spread(left)],
                type_args: None,
            })
        }
        _ => {
            // Fallback: wrap right side as call with left as arg
            swc::Expr::Call(swc::CallExpr {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                callee: swc::Callee::Expr(Box::new(translate_expr(&p.right))),
                args: vec![expr_or_spread(left)],
                type_args: None,
            })
        }
    }
}

fn translate_error_propagate(ep: &ErrorPropagateExpr) -> swc::Expr {
    // expr? → (()=>{ const _tmp = expr; if (_tmp instanceof Error) return _tmp; return _tmp; })()
    let tmp = "_tmp";
    let inner = translate_expr(&ep.expr);

    let body = swc::BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts: vec![
            // const _tmp = expr;
            swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                kind: swc::VarDeclKind::Const,
                declare: false,
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: swc::Pat::Ident(binding_ident(tmp)),
                    init: Some(Box::new(inner)),
                    definite: false,
                }],
            }))),
            // if (_tmp instanceof Error) return _tmp;
            swc::Stmt::If(swc::IfStmt {
                span: DUMMY_SP,
                test: Box::new(swc::Expr::Bin(swc::BinExpr {
                    span: DUMMY_SP,
                    op: swc::BinaryOp::InstanceOf,
                    left: Box::new(swc::Expr::Ident(ident(tmp))),
                    right: Box::new(swc::Expr::Ident(ident("Error"))),
                })),
                cons: Box::new(swc::Stmt::Return(swc::ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(swc::Expr::Ident(ident(tmp)))),
                })),
                alt: None,
            }),
            // return _tmp;
            swc::Stmt::Return(swc::ReturnStmt {
                span: DUMMY_SP,
                arg: Some(Box::new(swc::Expr::Ident(ident(tmp)))),
            }),
        ],
    };

    make_iife(body.stmts)
}

fn translate_assign(assign: &AssignExpr) -> swc::Expr {
    let op = match assign.op {
        AssignOp::Assign => swc::AssignOp::Assign,
        AssignOp::AddAssign => swc::AssignOp::AddAssign,
        AssignOp::SubAssign => swc::AssignOp::SubAssign,
        AssignOp::MulAssign => swc::AssignOp::MulAssign,
        AssignOp::DivAssign => swc::AssignOp::DivAssign,
    };

    swc::Expr::Assign(swc::AssignExpr {
        span: DUMMY_SP,
        op,
        left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Ident(binding_ident(
            match &assign.target {
                Expr::Ident(id) => &id.name,
                _ => "_",
            },
        ))),
        right: Box::new(translate_expr(&assign.value)),
    })
}

fn translate_template_string(ts: &TemplateStringExpr) -> swc::Expr {
    let mut quasis = Vec::new();
    let mut exprs: Vec<Box<swc::Expr>> = Vec::new();

    // Build template literal parts
    let mut i = 0;
    let parts = &ts.parts;
    while i < parts.len() {
        match &parts[i] {
            TemplatePart::String(s) => {
                let is_tail = i + 1 >= parts.len()
                    || (i + 2 >= parts.len() && matches!(&parts[i + 1], TemplatePart::Expr(_)));
                quasis.push(swc::TplElement {
                    span: DUMMY_SP,
                    tail: false, // will be fixed up
                    cooked: Some(s.clone().into()),
                    raw: s.clone().into(),
                });
                i += 1;
            }
            TemplatePart::Expr(e) => {
                // If no string before this expr, add empty quasis
                if quasis.len() == exprs.len() {
                    quasis.push(swc::TplElement {
                        span: DUMMY_SP,
                        tail: false,
                        cooked: Some("".into()),
                        raw: "".into(),
                    });
                }
                exprs.push(Box::new(translate_expr(e)));
                i += 1;
            }
        }
    }

    // Ensure we have trailing quasis
    if quasis.len() == exprs.len() {
        quasis.push(swc::TplElement {
            span: DUMMY_SP,
            tail: true,
            cooked: Some("".into()),
            raw: "".into(),
        });
    }

    // Mark last as tail
    if let Some(last) = quasis.last_mut() {
        last.tail = true;
    }

    swc::Expr::Tpl(swc::Tpl {
        span: DUMMY_SP,
        exprs,
        quasis,
    })
}

fn translate_match(m: &MatchExpr) -> swc::Expr {
    // Translate match to IIFE with if-else chain
    let subject_var = "_match";
    let subject = translate_expr(&m.subject);

    let mut stmts: Vec<swc::Stmt> = vec![swc::Stmt::Decl(swc::Decl::Var(Box::new(
        swc::VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: swc::VarDeclKind::Const,
            declare: false,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: swc::Pat::Ident(binding_ident(subject_var)),
                init: Some(Box::new(subject)),
                definite: false,
            }],
        },
    )))];

    // Build if-else chain from bottom up
    let mut else_stmt: Option<Box<swc::Stmt>> = None;

    for arm in m.arms.iter().rev() {
        let body_expr = translate_expr(&arm.body);
        let return_stmt = swc::Stmt::Return(swc::ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(body_expr)),
        });

        let (condition, bindings) = translate_pattern_to_condition(&arm.pattern, subject_var);

        let mut body_stmts: Vec<swc::Stmt> = Vec::new();
        // Add bindings
        for (name, init_expr) in bindings {
            body_stmts.push(swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                kind: swc::VarDeclKind::Const,
                declare: false,
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: swc::Pat::Ident(binding_ident(&name)),
                    init: Some(Box::new(init_expr)),
                    definite: false,
                }],
            }))));
        }
        body_stmts.push(return_stmt);

        match condition {
            Some(mut cond) => {
                // Add guard to condition
                if let Some(ref guard) = arm.guard {
                    cond = swc::Expr::Bin(swc::BinExpr {
                        span: DUMMY_SP,
                        op: swc::BinaryOp::LogicalAnd,
                        left: Box::new(cond),
                        right: Box::new(translate_expr(guard)),
                    });
                }
                let if_stmt = swc::Stmt::If(swc::IfStmt {
                    span: DUMMY_SP,
                    test: Box::new(cond),
                    cons: Box::new(swc::Stmt::Block(swc::BlockStmt {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        stmts: body_stmts,
                    })),
                    alt: else_stmt,
                });
                else_stmt = Some(Box::new(if_stmt));
            }
            None => {
                // Wildcard or catch-all — just the body
                if let Some(ref guard) = arm.guard {
                    let if_stmt = swc::Stmt::If(swc::IfStmt {
                        span: DUMMY_SP,
                        test: Box::new(translate_expr(guard)),
                        cons: Box::new(swc::Stmt::Block(swc::BlockStmt {
                            span: DUMMY_SP,
                            ctxt: SyntaxContext::empty(),
                            stmts: body_stmts,
                        })),
                        alt: else_stmt,
                    });
                    else_stmt = Some(Box::new(if_stmt));
                } else {
                    else_stmt = Some(Box::new(swc::Stmt::Block(swc::BlockStmt {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        stmts: body_stmts,
                    })));
                }
            }
        }
    }

    if let Some(chain) = else_stmt {
        stmts.push(*chain);
    }

    make_iife(stmts)
}

fn translate_pattern_to_condition(
    pattern: &Pattern,
    subject_var: &str,
) -> (Option<swc::Expr>, Vec<(String, swc::Expr)>) {
    match pattern {
        Pattern::Literal(lit) => {
            let cond = swc::Expr::Bin(swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::EqEqEq,
                left: Box::new(swc::Expr::Ident(ident(subject_var))),
                right: Box::new(translate_literal(lit)),
            });
            (Some(cond), Vec::new())
        }
        Pattern::Ident(name, _) => {
            // Bind the subject to the name
            let binding = (name.clone(), swc::Expr::Ident(ident(subject_var)));
            (None, vec![binding])
        }
        Pattern::Wildcard(_) => (None, Vec::new()),
        Pattern::Enum(ep) => {
            // Check tag field
            let cond = swc::Expr::Bin(swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::EqEqEq,
                left: Box::new(swc::Expr::Member(swc::MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(swc::Expr::Ident(ident(subject_var))),
                    prop: swc::MemberProp::Ident(swc::IdentName {
                        span: DUMMY_SP,
                        sym: "tag".into(),
                    }),
                })),
                right: Box::new(swc::Expr::Lit(swc::Lit::Str(swc::Str {
                    span: DUMMY_SP,
                    value: ep.variant.clone().into(),
                    raw: None,
                }))),
            });
            // Bind variant fields
            let bindings: Vec<(String, swc::Expr)> = ep
                .bindings
                .iter()
                .map(|b| {
                    (
                        b.clone(),
                        swc::Expr::Member(swc::MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(swc::Expr::Ident(ident(subject_var))),
                            prop: swc::MemberProp::Ident(swc::IdentName {
                                span: DUMMY_SP,
                                sym: b.clone().into(),
                            }),
                        }),
                    )
                })
                .collect();
            (Some(cond), bindings)
        }
        Pattern::Struct(sp) => {
            let bindings: Vec<(String, swc::Expr)> = sp
                .fields
                .iter()
                .map(|f| {
                    (
                        f.clone(),
                        swc::Expr::Member(swc::MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(swc::Expr::Ident(ident(subject_var))),
                            prop: swc::MemberProp::Ident(swc::IdentName {
                                span: DUMMY_SP,
                                sym: f.clone().into(),
                            }),
                        }),
                    )
                })
                .collect();
            (None, bindings)
        }
        Pattern::Range(from, to, _) => {
            let cond = swc::Expr::Bin(swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::LogicalAnd,
                left: Box::new(swc::Expr::Bin(swc::BinExpr {
                    span: DUMMY_SP,
                    op: swc::BinaryOp::GtEq,
                    left: Box::new(swc::Expr::Ident(ident(subject_var))),
                    right: Box::new(translate_expr(from)),
                })),
                right: Box::new(swc::Expr::Bin(swc::BinExpr {
                    span: DUMMY_SP,
                    op: swc::BinaryOp::LtEq,
                    left: Box::new(swc::Expr::Ident(ident(subject_var))),
                    right: Box::new(translate_expr(to)),
                })),
            });
            (Some(cond), Vec::new())
        }
    }
}

// ── Import translation ─────────────────────────────────────

fn translate_import(imp: &Import) -> swc::ModuleDecl {
    let src = Box::new(swc::Str {
        span: DUMMY_SP,
        value: imp.path.clone().into(),
        raw: None,
    });

    if let Some(ref alias) = imp.namespace {
        // import * as alias from "path"
        swc::ModuleDecl::Import(swc::ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![swc::ImportSpecifier::Namespace(
                swc::ImportStarAsSpecifier {
                    span: DUMMY_SP,
                    local: ident(alias),
                },
            )],
            src,
            type_only: false,
            with: None,
            phase: Default::default(),
        })
    } else {
        let specifiers: Vec<swc::ImportSpecifier> = imp
            .names
            .iter()
            .map(|n| {
                swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: ident(&n.name),
                    imported: n
                        .alias
                        .as_ref()
                        .map(|a| swc::ModuleExportName::Ident(ident(a))),
                    is_type_only: false,
                })
            })
            .collect();
        swc::ModuleDecl::Import(swc::ImportDecl {
            span: DUMMY_SP,
            specifiers,
            src,
            type_only: false,
            with: None,
            phase: Default::default(),
        })
    }
}

// ── Utility functions ──────────────────────────────────────

fn block_to_expr(block: &Block) -> swc::Expr {
    if block.stmts.is_empty() {
        if let Some(ref tail) = block.tail_expr {
            return translate_expr(tail);
        }
    }
    // Wrap in IIFE
    let body = translate_block_with_implicit_return(block);
    make_iife(body.stmts)
}

fn make_iife(stmts: Vec<swc::Stmt>) -> swc::Expr {
    swc::Expr::Call(swc::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: swc::Callee::Expr(Box::new(swc::Expr::Arrow(swc::ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: Vec::new(),
            body: Box::new(swc::BlockStmtOrExpr::BlockStmt(swc::BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts,
            })),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }))),
        args: Vec::new(),
        type_args: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_parser;

    fn compile(src: &str) -> String {
        let parsed = ag_parser::parse(src);
        assert!(
            parsed.diagnostics.is_empty(),
            "parse errors: {:?}",
            parsed.diagnostics
        );
        codegen(&parsed.module)
    }

    #[test]
    fn let_binding() {
        let js = compile("let x = 42");
        assert!(js.contains("const x = 42"));
    }

    #[test]
    fn mut_binding() {
        let js = compile("mut counter = 0");
        assert!(js.contains("let counter = 0"));
    }

    #[test]
    fn const_binding() {
        let js = compile("const MAX = 100");
        assert!(js.contains("const MAX = 100"));
    }

    #[test]
    fn simple_function() {
        let js = compile("fn add(a: int, b: int) -> int { a + b }");
        assert!(js.contains("function add(a, b)"));
        assert!(js.contains("return a + b"));
    }

    #[test]
    fn pub_function() {
        let js = compile("pub fn greet(name: str) -> str { name }");
        assert!(js.contains("export function greet(name)"));
    }

    #[test]
    fn default_params() {
        let js = compile("fn greet(name: str, loud: bool = false) -> str { name }");
        assert!(js.contains("loud = false"));
    }

    #[test]
    fn arrow_function() {
        let js = compile("let double = (x: int) => x * 2");
        assert!(js.contains("const double = (x)=>x * 2"));
    }

    #[test]
    fn struct_erased() {
        let js = compile("struct User { name: str, age: int }");
        assert!(js.trim().is_empty());
    }

    #[test]
    fn enum_erased() {
        let js = compile("enum Status { Pending, Active(since: str) }");
        assert!(js.trim().is_empty());
    }

    #[test]
    fn type_alias_erased() {
        let js = compile("type ID = str");
        assert!(js.trim().is_empty());
    }

    #[test]
    fn for_of_loop() {
        let js = compile("fn f(items: [int]) { for item in items { process(item) } }");
        assert!(js.contains("for (const item of items)"));
    }

    #[test]
    fn while_loop() {
        let js = compile("fn f() { while x > 0 { x = x - 1 } }");
        assert!(js.contains("while"));
    }

    #[test]
    fn try_catch() {
        let js = compile("fn f() { try { parse(input) } catch e { log(e) } }");
        assert!(js.contains("try"));
        assert!(js.contains("catch"));
    }

    #[test]
    fn named_imports() {
        let js = compile(r#"import { read, write } from "./fs""#);
        assert!(js.contains("import"));
        assert!(js.contains("read"));
        assert!(js.contains("write"));
    }

    #[test]
    fn namespace_import() {
        let js = compile(r#"import * as fs from "./fs""#);
        assert!(js.contains("* as fs"));
    }

    #[test]
    fn pipe_simple() {
        let js = compile("let x = data |> parse");
        assert!(js.contains("parse(data)"));
    }

    #[test]
    fn optional_chaining() {
        let js = compile("fn f(user: any) { let x = user?.name }");
        assert!(js.contains("?."));
    }

    #[test]
    fn nullish_coalescing() {
        let js = compile(r#"fn f(x: any) { let y = x ?? "default" }"#);
        assert!(js.contains("??"));
    }

    #[test]
    fn template_string() {
        let js = compile("let x = `hello ${name}!`");
        assert!(js.contains("`"));
    }

    // ── DSL codegen tests (prompt-dsl handler) ──

    #[test]
    fn dsl_prompt_inline_no_capture() {
        let js = compile("@prompt greeting ```\n@role system\nHello, world!\n```\n");
        assert!(js.contains("const greeting"));
        assert!(js.contains("PromptTemplate"));
        assert!(js.contains("Hello, world!"));
        assert!(js.contains("system"));
    }

    #[test]
    fn dsl_prompt_inline_with_captures() {
        let js = compile("@prompt system ```\n@role system\nYou are #{role}. Answer in #{lang}.\n```\n");
        assert!(js.contains("const system"));
        assert!(js.contains("PromptTemplate"));
        assert!(js.contains("ctx.role"));
        assert!(js.contains("ctx.lang"));
    }

    #[test]
    fn dsl_prompt_file_ref() {
        let js = compile(r#"@prompt system from "./system-prompt.txt""#);
        assert!(js.contains("const system"));
        assert!(js.contains("PromptTemplate"));
        assert!(js.contains("readFile"));
        assert!(js.contains("system-prompt.txt"));
    }

    #[test]
    fn dsl_unregistered_handler_error() {
        let parsed = ag_parser::parse("@graphql GetUsers ```\nquery { users }\n```\n");
        let translator = Translator::new();
        // Don't register any handler
        let result = translator.codegen(&parsed.module);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("no handler registered"));
        assert!(err.message.contains("graphql"));
    }

    #[test]
    fn dsl_handler_uses_block_name() {
        let js = compile("@prompt my_prompt ```\n@role system\nContent here\n```\n");
        assert!(js.contains("const my_prompt"));
        assert!(js.contains("PromptTemplate"));
    }
}
