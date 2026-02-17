/// Byte offset span in source code.
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

// ── Top-level ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    FnDecl(FnDecl),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    TypeAlias(TypeAlias),
    Import(Import),
    VarDecl(VarDecl),
    ExprStmt(ExprStmt),
    DslBlock(DslBlock),
    ExternFnDecl(ExternFnDecl),
    ExternStructDecl(ExternStructDecl),
    ExternTypeDecl(ExternTypeDecl),
}

// ── DSL Block ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DslBlock {
    pub kind: String,
    pub name: Ident,
    pub content: DslContent,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum DslContent {
    Inline { parts: Vec<DslPart> },
    FileRef { path: String, span: Span },
}

#[derive(Debug, Clone)]
pub enum DslPart {
    Text(String, Span),
    Capture(Box<Expr>, Span),
}

// ── Expressions ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    Member(MemberExpr),
    Index(IndexExpr),
    If(Box<IfExpr>),
    Match(Box<MatchExpr>),
    Block(Box<Block>),
    Ident(Ident),
    Literal(Literal),
    Array(ArrayExpr),
    Object(ObjectExpr),
    Arrow(Box<ArrowExpr>),
    Pipe(Box<PipeExpr>),
    OptionalChain(Box<OptionalChainExpr>),
    NullishCoalesce(Box<NullishCoalesceExpr>),
    Await(Box<AwaitExpr>),
    ErrorPropagate(Box<ErrorPropagateExpr>),
    Assign(Box<AssignExpr>),
    TemplateString(TemplateStringExpr),
    Placeholder(Span),
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MemberExpr {
    pub object: Box<Expr>,
    pub field: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Expr,
    pub then_block: Block,
    pub else_branch: Option<ElseBranch>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfExpr>),
}

#[derive(Debug, Clone)]
pub struct MatchExpr {
    pub subject: Expr,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrayExpr {
    pub elements: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ObjectExpr {
    pub fields: Vec<ObjectField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ObjectField {
    pub key: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrowExpr {
    pub params: Vec<Param>,
    pub body: ArrowBody,
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ArrowBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone)]
pub struct PipeExpr {
    pub left: Expr,
    pub right: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct OptionalChainExpr {
    pub object: Expr,
    pub field: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct NullishCoalesceExpr {
    pub left: Expr,
    pub right: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AwaitExpr {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ErrorPropagateExpr {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignExpr {
    pub target: Expr,
    pub value: Expr,
    pub op: AssignOp,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TemplateStringExpr {
    pub parts: Vec<TemplatePart>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TemplatePart {
    String(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Bool(bool, Span),
    Nil(Span),
}

impl Literal {
    pub fn span(&self) -> Span {
        match self {
            Literal::Int(_, s)
            | Literal::Float(_, s)
            | Literal::String(_, s)
            | Literal::Bool(_, s)
            | Literal::Nil(s) => *s,
        }
    }
}

// ── Statements ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl(VarDecl),
    ExprStmt(ExprStmt),
    Return(ReturnStmt),
    If(IfExpr),
    For(ForStmt),
    While(WhileStmt),
    Match(MatchExpr),
    TryCatch(TryCatchStmt),
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub binding: String,
    pub iter: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TryCatchStmt {
    pub try_block: Block,
    pub catch_binding: String,
    pub catch_block: Block,
    pub span: Span,
}

// ── Types ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String, Span),
    Array(Box<TypeExpr>, Span),
    Map(Box<TypeExpr>, Box<TypeExpr>, Span),
    Nullable(Box<TypeExpr>, Span),
    Union(Box<TypeExpr>, Box<TypeExpr>, Span),
    Function(FunctionType),
    Object(ObjectType),
    Promise(Box<TypeExpr>, Span),
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub params: Vec<TypeExpr>,
    pub ret: Box<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ObjectType {
    pub fields: Vec<TypeField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeField {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

// ── Patterns ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Literal),
    Ident(String, Span),
    Struct(StructPattern),
    Enum(EnumPattern),
    Wildcard(Span),
    Range(Box<Expr>, Box<Expr>, Span),
}

#[derive(Debug, Clone)]
pub struct StructPattern {
    pub fields: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumPattern {
    pub enum_name: String,
    pub variant: String,
    pub bindings: Vec<String>,
    pub span: Span,
}

// ── Extern Declarations ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JsAnnotation {
    pub module: Option<String>,
    pub js_name: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExternFnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub js_annotation: Option<JsAnnotation>,
    pub variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExternStructDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub methods: Vec<MethodSignature>,
    pub js_annotation: Option<JsAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExternTypeDecl {
    pub name: String,
    pub js_annotation: Option<JsAnnotation>,
    pub span: Span,
}

// ── Declarations ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub kind: VarKind,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub init: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Let,
    Mut,
    Const,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub is_pub: bool,
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub default: Option<Expr>,
    pub is_variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub names: Vec<ImportName>,
    pub path: String,
    pub namespace: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
    pub span: Span,
}

// ── Block ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail_expr: Option<Box<Expr>>,
    pub span: Span,
}

// ── Match arm ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

// ── Operators ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

// ── Diagnostic ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}
