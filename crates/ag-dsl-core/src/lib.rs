use std::any::Any;

// ── Span ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

// ── DslError ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DslError {
    pub message: String,
    pub span: Option<Span>,
}

// ── DslBlock / DslContent / DslPart ───────────────────────

#[derive(Debug)]
pub struct DslBlock {
    pub kind: String,
    pub name: String,
    pub content: DslContent,
    pub span: Span,
}

#[derive(Debug)]
pub enum DslContent {
    Inline { parts: Vec<DslPart> },
    FileRef { path: String, span: Span },
}

pub enum DslPart {
    Text(String, Span),
    Capture(Box<dyn Any>, Span),
}

impl std::fmt::Debug for DslPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DslPart::Text(s, span) => write!(f, "Text({:?}, {:?})", s, span),
            DslPart::Capture(_, span) => write!(f, "Capture(<expr>, {:?})", span),
        }
    }
}

// ── CodegenContext trait ──────────────────────────────────

pub trait CodegenContext {
    fn translate_expr(&mut self, expr: &dyn Any) -> swc_ecma_ast::Expr;
    fn translate_block(&mut self, block: &dyn Any) -> Vec<swc_ecma_ast::Stmt>;
}

// ── DslHandler trait ─────────────────────────────────────

pub trait DslHandler {
    fn handle(
        &self,
        block: &DslBlock,
        ctx: &mut dyn CodegenContext,
    ) -> Result<Vec<swc_ecma_ast::ModuleItem>, DslError>;
}
