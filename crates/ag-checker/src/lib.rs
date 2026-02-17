use ag_ast::*;
use std::collections::HashMap;

// ── Type representation ────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Str,
    Num,
    Int,
    Bool,
    Nil,
    Any,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Nullable(Box<Type>),
    Union(Box<Type>, Box<Type>),
    Function(Vec<Type>, Box<Type>),
    Struct(String, Vec<(String, Type)>),
    Enum(String, Vec<(String, Vec<(String, Type)>)>),
    Promise(Box<Type>),
    VariadicFunction(Vec<Type>, Box<Type>), // fixed params + variadic element type as last
    Unknown,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Str => write!(f, "str"),
            Type::Num => write!(f, "num"),
            Type::Int => write!(f, "int"),
            Type::Bool => write!(f, "bool"),
            Type::Nil => write!(f, "nil"),
            Type::Any => write!(f, "any"),
            Type::Array(t) => write!(f, "[{t}]"),
            Type::Map(k, v) => write!(f, "{{{k}: {v}}}"),
            Type::Nullable(t) => write!(f, "{t}?"),
            Type::Union(a, b) => write!(f, "{a} | {b}"),
            Type::Function(params, ret) => {
                let ps: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({}) -> {ret}", ps.join(", "))
            }
            Type::Struct(name, _) => write!(f, "{name}"),
            Type::Enum(name, _) => write!(f, "{name}"),
            Type::Promise(inner) => write!(f, "Promise<{inner}>"),
            Type::VariadicFunction(params, ret) => {
                let ps: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({}, ...) -> {ret}", ps.join(", "))
            }
            Type::Unknown => write!(f, "unknown"),
        }
    }
}

// ── Symbol table ───────────────────────────────────────────

#[derive(Debug, Clone)]
struct Symbol {
    ty: Type,
    mutable: bool,
}

struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    fn child(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    fn define(&mut self, name: &str, sym: Symbol) -> bool {
        if self.symbols.contains_key(name) {
            return false; // duplicate
        }
        self.symbols.insert(name.to_string(), sym);
        true
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }
}

// ── Checker ────────────────────────────────────────────────

pub struct Checker {
    scope: Scope,
    pub diagnostics: Vec<Diagnostic>,
    type_aliases: HashMap<String, Type>,
    in_async: bool,
}

pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
}

pub fn check(module: &Module) -> CheckResult {
    let mut checker = Checker::new();
    checker.check_module(module);
    CheckResult {
        diagnostics: checker.diagnostics,
    }
}

impl Checker {
    fn new() -> Self {
        Self {
            scope: Scope::new(),
            diagnostics: Vec::new(),
            type_aliases: HashMap::new(),
            in_async: false,
        }
    }

    fn error(&mut self, msg: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic {
            message: msg.into(),
            span,
        });
    }

    // ── Type compatibility ─────────────────────────────────

    fn type_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if expected == actual {
            return true;
        }
        match (expected, actual) {
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (Type::Num, Type::Int) => true, // int widens to num
            (Type::Nullable(inner), _) => {
                self.type_compatible(inner, actual) || matches!(actual, Type::Nil)
            }
            (_, Type::Nil) if matches!(expected, Type::Nullable(_)) => true,
            (Type::Union(a, b), _) => {
                self.type_compatible(a, actual) || self.type_compatible(b, actual)
            }
            (_, Type::Union(a, b)) => {
                self.type_compatible(expected, a) && self.type_compatible(expected, b)
            }
            (Type::Array(e), Type::Array(a)) => self.type_compatible(e, a),
            (Type::Map(ek, ev), Type::Map(ak, av)) => {
                self.type_compatible(ek, ak) && self.type_compatible(ev, av)
            }
            (Type::Function(ep, er), Type::Function(ap, ar)) => {
                ep.len() == ap.len()
                    && ep.iter().zip(ap).all(|(e, a)| self.type_compatible(e, a))
                    && self.type_compatible(er, ar)
            }
            (Type::Promise(e), Type::Promise(a)) => self.type_compatible(e, a),
            // Structural subtyping for structs
            (Type::Struct(_, expected_fields), Type::Struct(_, actual_fields)) => {
                expected_fields.iter().all(|(name, ty)| {
                    actual_fields
                        .iter()
                        .any(|(n, t)| n == name && self.type_compatible(ty, t))
                })
            }
            _ => false,
        }
    }

    // ── Resolve TypeExpr to Type ───────────────────────────

    fn resolve_type(&self, ty: &TypeExpr) -> Type {
        match ty {
            TypeExpr::Named(name, _) => match name.as_str() {
                "str" => Type::Str,
                "num" => Type::Num,
                "int" => Type::Int,
                "bool" => Type::Bool,
                "nil" => Type::Nil,
                "any" => Type::Any,
                _ => {
                    if let Some(alias) = self.type_aliases.get(name) {
                        alias.clone()
                    } else if let Some(sym) = self.scope.lookup(name) {
                        sym.ty.clone()
                    } else {
                        Type::Unknown
                    }
                }
            },
            TypeExpr::Array(inner, _) => Type::Array(Box::new(self.resolve_type(inner))),
            TypeExpr::Map(k, v, _) => {
                Type::Map(Box::new(self.resolve_type(k)), Box::new(self.resolve_type(v)))
            }
            TypeExpr::Nullable(inner, _) => Type::Nullable(Box::new(self.resolve_type(inner))),
            TypeExpr::Union(a, b, _) => Type::Union(
                Box::new(self.resolve_type(a)),
                Box::new(self.resolve_type(b)),
            ),
            TypeExpr::Function(ft) => {
                let params: Vec<Type> = ft.params.iter().map(|p| self.resolve_type(p)).collect();
                let ret = self.resolve_type(&ft.ret);
                Type::Function(params, Box::new(ret))
            }
            TypeExpr::Object(ot) => {
                let fields: Vec<(String, Type)> = ot
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), self.resolve_type(&f.ty)))
                    .collect();
                Type::Struct("anonymous".to_string(), fields)
            }
            TypeExpr::Promise(inner, _) => {
                Type::Promise(Box::new(self.resolve_type(inner)))
            }
        }
    }

    // ── Module check ───────────────────────────────────────

    fn check_module(&mut self, module: &Module) {
        // First pass: register all declarations
        for item in &module.items {
            match item {
                Item::FnDecl(f) => self.register_fn_decl(f),
                Item::StructDecl(s) => self.register_struct_decl(s),
                Item::EnumDecl(e) => self.register_enum_decl(e),
                Item::TypeAlias(t) => self.register_type_alias(t),
                Item::ExternFnDecl(ef) => self.register_extern_fn_decl(ef),
                Item::ExternStructDecl(es) => self.register_extern_struct_decl(es),
                Item::ExternTypeDecl(et) => self.register_extern_type_decl(et),
                _ => {}
            }
        }

        // Second pass: check bodies
        for item in &module.items {
            match item {
                Item::FnDecl(f) => self.check_fn_decl(f),
                Item::VarDecl(v) => self.check_var_decl(v),
                Item::ExprStmt(e) => {
                    self.check_expr(&e.expr);
                }
                Item::DslBlock(dsl) => self.check_dsl_block(dsl),
                _ => {}
            }
        }
    }

    fn check_dsl_block(&mut self, dsl: &DslBlock) {
        if let DslContent::Inline { parts } = &dsl.content {
            for part in parts {
                if let DslPart::Capture(expr, _) = part {
                    self.check_expr(expr);
                }
            }
        }
    }

    fn register_fn_decl(&mut self, f: &FnDecl) {
        let param_types: Vec<Type> = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or(Type::Any)
            })
            .collect();
        let mut ret_type = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Nil);
        // async fn externally returns Promise<T>
        if f.is_async {
            ret_type = Type::Promise(Box::new(ret_type));
        }
        self.scope.define(
            &f.name,
            Symbol {
                ty: Type::Function(param_types, Box::new(ret_type)),
                mutable: false,
            },
        );
    }

    fn register_struct_decl(&mut self, s: &StructDecl) {
        let fields: Vec<(String, Type)> = s
            .fields
            .iter()
            .map(|f| (f.name.clone(), self.resolve_type(&f.ty)))
            .collect();
        let ty = Type::Struct(s.name.clone(), fields);
        self.scope.define(
            &s.name,
            Symbol {
                ty,
                mutable: false,
            },
        );
    }

    fn register_enum_decl(&mut self, e: &EnumDecl) {
        let variants: Vec<(String, Vec<(String, Type)>)> = e
            .variants
            .iter()
            .map(|v| {
                let fields: Vec<(String, Type)> = v
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), self.resolve_type(&f.ty)))
                    .collect();
                (v.name.clone(), fields)
            })
            .collect();
        let ty = Type::Enum(e.name.clone(), variants);
        self.scope.define(
            &e.name,
            Symbol {
                ty,
                mutable: false,
            },
        );
    }

    fn register_type_alias(&mut self, t: &TypeAlias) {
        let ty = self.resolve_type(&t.ty);
        self.type_aliases.insert(t.name.clone(), ty);
    }

    fn register_extern_fn_decl(&mut self, ef: &ExternFnDecl) {
        let param_types: Vec<Type> = ef
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or(Type::Any)
            })
            .collect();
        let ret_type = ef
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Nil);
        let ty = if ef.variadic {
            Type::VariadicFunction(param_types, Box::new(ret_type))
        } else {
            Type::Function(param_types, Box::new(ret_type))
        };
        if !self.scope.define(
            &ef.name,
            Symbol {
                ty,
                mutable: false,
            },
        ) {
            self.error(format!("duplicate declaration `{}`", ef.name), ef.span);
        }
    }

    fn register_extern_struct_decl(&mut self, es: &ExternStructDecl) {
        let fields: Vec<(String, Type)> = es
            .fields
            .iter()
            .map(|f| (f.name.clone(), self.resolve_type(&f.ty)))
            .collect();
        // Also register methods as fields with function types
        let mut all_fields = fields;
        for m in &es.methods {
            let param_types: Vec<Type> = m
                .params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map(|t| self.resolve_type(t))
                        .unwrap_or(Type::Any)
                })
                .collect();
            let ret_type = m
                .return_type
                .as_ref()
                .map(|t| self.resolve_type(t))
                .unwrap_or(Type::Nil);
            all_fields.push((m.name.clone(), Type::Function(param_types, Box::new(ret_type))));
        }
        let ty = Type::Struct(es.name.clone(), all_fields);
        if !self.scope.define(
            &es.name,
            Symbol {
                ty,
                mutable: false,
            },
        ) {
            self.error(format!("duplicate declaration `{}`", es.name), es.span);
        }
    }

    fn register_extern_type_decl(&mut self, et: &ExternTypeDecl) {
        // Opaque type: register as a struct with no fields
        let ty = Type::Struct(et.name.clone(), Vec::new());
        if !self.scope.define(
            &et.name,
            Symbol {
                ty,
                mutable: false,
            },
        ) {
            self.error(format!("duplicate declaration `{}`", et.name), et.span);
        }
    }

    // ── Function check ─────────────────────────────────────

    fn check_fn_decl(&mut self, f: &FnDecl) {
        let parent = std::mem::replace(&mut self.scope, Scope::new());
        self.scope = Scope::child(parent);
        let prev_async = self.in_async;
        self.in_async = f.is_async;

        // Check and register params
        for param in &f.params {
            if param.ty.is_none() && param.default.is_none() {
                self.error(
                    format!("parameter `{}` requires a type annotation", param.name),
                    param.span,
                );
            }
            let ty = param
                .ty
                .as_ref()
                .map(|t| self.resolve_type(t))
                .unwrap_or(Type::Any);
            self.scope.define(
                &param.name,
                Symbol {
                    ty,
                    mutable: false,
                },
            );
        }

        let declared_ret = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t));

        // Check body
        let body_type = self.check_block(&f.body);

        // Check return type matches
        if let Some(ref expected) = declared_ret {
            if !self.type_compatible(expected, &body_type) {
                self.error(
                    format!(
                        "return type mismatch: expected `{}`, found `{}`",
                        expected, body_type
                    ),
                    f.span,
                );
            }
        }

        // Restore scope and async state
        self.in_async = prev_async;
        let child = std::mem::replace(&mut self.scope, Scope::new());
        self.scope = *child.parent.unwrap();
    }

    // ── Variable check ─────────────────────────────────────

    fn check_var_decl(&mut self, v: &VarDecl) {
        let init_type = self.check_expr(&v.init);

        if let Some(ref ty_expr) = v.ty {
            let declared = self.resolve_type(ty_expr);
            if !self.type_compatible(&declared, &init_type) {
                self.error(
                    format!(
                        "type mismatch: expected `{}`, found `{}`",
                        declared, init_type
                    ),
                    v.span,
                );
            }
        }

        let ty = v
            .ty
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or(init_type);

        let mutable = v.kind == VarKind::Mut;
        if !self.scope.define(
            &v.name,
            Symbol {
                ty,
                mutable,
            },
        ) {
            self.error(format!("duplicate binding `{}`", v.name), v.span);
        }
    }

    // ── Expression check ───────────────────────────────────

    fn check_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_, _) => Type::Int,
                Literal::Float(_, _) => Type::Num,
                Literal::String(_, _) => Type::Str,
                Literal::Bool(_, _) => Type::Bool,
                Literal::Nil(_) => Type::Nil,
            },
            Expr::Ident(ident) => {
                if let Some(sym) = self.scope.lookup(&ident.name) {
                    sym.ty.clone()
                } else {
                    self.error(
                        format!("undefined variable `{}`", ident.name),
                        ident.span,
                    );
                    Type::Unknown
                }
            }
            Expr::Binary(b) => {
                let left_ty = self.check_expr(&b.left);
                let right_ty = self.check_expr(&b.right);
                match b.op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                    | BinaryOp::Mod | BinaryOp::Pow => {
                        if matches!((&left_ty, &right_ty), (Type::Int, Type::Int)) {
                            Type::Int
                        } else if matches!(
                            (&left_ty, &right_ty),
                            (Type::Num | Type::Int, Type::Num | Type::Int)
                        ) {
                            Type::Num
                        } else if b.op == BinaryOp::Add
                            && matches!((&left_ty, &right_ty), (Type::Str, Type::Str))
                        {
                            Type::Str
                        } else {
                            Type::Any
                        }
                    }
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Gt
                    | BinaryOp::Le | BinaryOp::Ge => Type::Bool,
                    BinaryOp::And | BinaryOp::Or => Type::Bool,
                }
            }
            Expr::Unary(u) => {
                let inner = self.check_expr(&u.operand);
                match u.op {
                    UnaryOp::Not => Type::Bool,
                    UnaryOp::Neg => inner,
                }
            }
            Expr::Call(call) => self.check_call(call),
            Expr::Member(m) => self.check_member_access(m),
            Expr::Index(i) => {
                let obj = self.check_expr(&i.object);
                self.check_expr(&i.index);
                match obj {
                    Type::Array(inner) => *inner,
                    Type::Map(_, v) => *v,
                    _ => Type::Any,
                }
            }
            Expr::If(if_expr) => {
                self.check_expr(&if_expr.condition);
                let then_ty = self.check_block(&if_expr.then_block);
                if let Some(ref else_branch) = if_expr.else_branch {
                    let else_ty = match else_branch {
                        ElseBranch::Block(b) => self.check_block(b),
                        ElseBranch::If(nested) => {
                            self.check_expr(&Expr::If(nested.clone()))
                        }
                    };
                    if self.type_compatible(&then_ty, &else_ty) {
                        then_ty
                    } else {
                        Type::Union(Box::new(then_ty), Box::new(else_ty))
                    }
                } else {
                    then_ty
                }
            }
            Expr::Match(m) => self.check_match(m),
            Expr::Block(b) => self.check_block(b),
            Expr::Array(arr) => {
                if arr.elements.is_empty() {
                    Type::Array(Box::new(Type::Any))
                } else {
                    let first = self.check_expr(&arr.elements[0]);
                    for elem in &arr.elements[1..] {
                        self.check_expr(elem);
                    }
                    Type::Array(Box::new(first))
                }
            }
            Expr::Object(obj) => {
                let fields: Vec<(String, Type)> = obj
                    .fields
                    .iter()
                    .map(|f| {
                        let ty = self.check_expr(&f.value);
                        (f.key.clone(), ty)
                    })
                    .collect();
                Type::Struct("anonymous".to_string(), fields)
            }
            Expr::Arrow(arrow) => {
                let parent = std::mem::replace(&mut self.scope, Scope::new());
                self.scope = Scope::child(parent);
                let param_types: Vec<Type> = arrow
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .ty
                            .as_ref()
                            .map(|t| self.resolve_type(t))
                            .unwrap_or(Type::Any);
                        self.scope.define(
                            &p.name,
                            Symbol {
                                ty: ty.clone(),
                                mutable: false,
                            },
                        );
                        ty
                    })
                    .collect();
                let ret = match &arrow.body {
                    ArrowBody::Expr(e) => self.check_expr(e),
                    ArrowBody::Block(b) => self.check_block(b),
                };
                let child = std::mem::replace(&mut self.scope, Scope::new());
                self.scope = *child.parent.unwrap();
                Type::Function(param_types, Box::new(ret))
            }
            Expr::Pipe(p) => {
                let left_ty = self.check_expr(&p.left);
                let _right_ty = self.check_expr(&p.right);
                // Pipe result depends on the right side function
                Type::Any // simplified
            }
            Expr::OptionalChain(oc) => {
                let obj_ty = self.check_expr(&oc.object);
                Type::Any // simplified
            }
            Expr::NullishCoalesce(nc) => {
                let left = self.check_expr(&nc.left);
                let right = self.check_expr(&nc.right);
                right // simplified: result is the non-null type
            }
            Expr::Await(a) => {
                if !self.in_async {
                    self.error("await can only be used inside async functions", a.span);
                }
                let inner_ty = self.check_expr(&a.expr);
                match inner_ty {
                    Type::Promise(inner) => *inner,
                    Type::Any | Type::Unknown => inner_ty,
                    _ => {
                        self.error(
                            format!("await requires a Promise, found `{}`", inner_ty),
                            a.span,
                        );
                        Type::Unknown
                    }
                }
            }
            Expr::ErrorPropagate(ep) => self.check_expr(&ep.expr),
            Expr::Assign(assign) => {
                let value_ty = self.check_expr(&assign.value);
                // Check mutability
                if let Expr::Ident(ident) = &assign.target {
                    if let Some(sym) = self.scope.lookup(&ident.name) {
                        if !sym.mutable {
                            self.error(
                                format!("cannot assign to immutable binding `{}`", ident.name),
                                assign.span,
                            );
                        }
                    }
                }
                value_ty
            }
            Expr::TemplateString(_) => Type::Str,
            Expr::Placeholder(_) => Type::Any,
        }
    }

    fn check_call(&mut self, call: &CallExpr) -> Type {
        let callee_ty = self.check_expr(&call.callee);
        for arg in &call.args {
            self.check_expr(arg);
        }

        match &callee_ty {
            Type::Function(param_types, ret) => {
                if call.args.len() > param_types.len() {
                    self.error(
                        format!(
                            "expected {} arguments, found {}",
                            param_types.len(),
                            call.args.len()
                        ),
                        call.span,
                    );
                }
                for (i, (arg, param_ty)) in call.args.iter().zip(param_types).enumerate() {
                    let arg_ty = self.check_expr(arg);
                    if !self.type_compatible(param_ty, &arg_ty) {
                        self.error(
                            format!(
                                "argument {}: expected `{}`, found `{}`",
                                i + 1, param_ty, arg_ty
                            ),
                            call.span,
                        );
                    }
                }
                *ret.clone()
            }
            Type::VariadicFunction(param_types, ret) => {
                // Fixed params come first; last param_type is the variadic element type
                let (fixed, variadic_ty) = if param_types.is_empty() {
                    (param_types.as_slice(), &Type::Any)
                } else {
                    let (fixed, rest) = param_types.split_at(param_types.len() - 1);
                    (fixed, &rest[0])
                };

                // Check minimum arity (fixed params)
                if call.args.len() < fixed.len() {
                    self.error(
                        format!(
                            "expected at least {} arguments, found {}",
                            fixed.len(),
                            call.args.len()
                        ),
                        call.span,
                    );
                }

                for (i, arg) in call.args.iter().enumerate() {
                    let arg_ty = self.check_expr(arg);
                    if i < fixed.len() {
                        if !self.type_compatible(&fixed[i], &arg_ty) {
                            self.error(
                                format!(
                                    "argument {}: expected `{}`, found `{}`",
                                    i + 1, fixed[i], arg_ty
                                ),
                                call.span,
                            );
                        }
                    } else {
                        // Variadic args
                        if !self.type_compatible(variadic_ty, &arg_ty) {
                            self.error(
                                format!(
                                    "argument {}: expected `{}`, found `{}`",
                                    i + 1, variadic_ty, arg_ty
                                ),
                                call.span,
                            );
                        }
                    }
                }
                *ret.clone()
            }
            _ => Type::Any,
        }
    }

    fn check_member_access(&mut self, m: &MemberExpr) -> Type {
        let obj_ty = self.check_expr(&m.object);
        match &obj_ty {
            Type::Struct(name, fields) => {
                if let Some((_, ty)) = fields.iter().find(|(n, _)| n == &m.field) {
                    ty.clone()
                } else {
                    self.error(
                        format!("field `{}` does not exist on type `{}`", m.field, name),
                        m.span,
                    );
                    Type::Unknown
                }
            }
            _ => Type::Any,
        }
    }

    fn check_match(&mut self, m: &MatchExpr) -> Type {
        let subject_ty = self.check_expr(&m.subject);
        let mut result_ty: Option<Type> = None;

        for arm in &m.arms {
            // Enter new scope for pattern bindings
            let parent = std::mem::replace(&mut self.scope, Scope::new());
            self.scope = Scope::child(parent);

            self.bind_pattern(&arm.pattern, &subject_ty);

            if let Some(ref guard) = arm.guard {
                self.check_expr(guard);
            }

            let arm_ty = self.check_expr(&arm.body);

            // Restore scope
            let child = std::mem::replace(&mut self.scope, Scope::new());
            self.scope = *child.parent.unwrap();

            if let Some(ref existing) = result_ty {
                if !self.type_compatible(existing, &arm_ty) {
                    result_ty = Some(Type::Union(
                        Box::new(existing.clone()),
                        Box::new(arm_ty),
                    ));
                }
            } else {
                result_ty = Some(arm_ty);
            }
        }

        result_ty.unwrap_or(Type::Nil)
    }

    fn bind_pattern(&mut self, pattern: &Pattern, subject_ty: &Type) {
        match pattern {
            Pattern::Ident(name, _) => {
                self.scope.define(
                    name,
                    Symbol {
                        ty: subject_ty.clone(),
                        mutable: false,
                    },
                );
            }
            Pattern::Enum(ep) => {
                // Bind enum variant fields
                if let Type::Enum(_, variants) = subject_ty {
                    if let Some((_, fields)) = variants.iter().find(|(n, _)| n == &ep.variant) {
                        for (binding, (_, ty)) in ep.bindings.iter().zip(fields) {
                            self.scope.define(
                                binding,
                                Symbol {
                                    ty: ty.clone(),
                                    mutable: false,
                                },
                            );
                        }
                    }
                }
            }
            Pattern::Struct(sp) => {
                if let Type::Struct(_, fields) = subject_ty {
                    for field_name in &sp.fields {
                        if let Some((_, ty)) = fields.iter().find(|(n, _)| n == field_name) {
                            self.scope.define(
                                field_name,
                                Symbol {
                                    ty: ty.clone(),
                                    mutable: false,
                                },
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── Block check ────────────────────────────────────────

    fn check_block(&mut self, block: &Block) -> Type {
        let parent = std::mem::replace(&mut self.scope, Scope::new());
        self.scope = Scope::child(parent);

        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }

        let ty = if let Some(ref tail) = block.tail_expr {
            self.check_expr(tail)
        } else {
            Type::Nil
        };

        let child = std::mem::replace(&mut self.scope, Scope::new());
        self.scope = *child.parent.unwrap();

        ty
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(v) => self.check_var_decl(v),
            Stmt::ExprStmt(e) => {
                self.check_expr(&e.expr);
            }
            Stmt::Return(r) => {
                if let Some(ref val) = r.value {
                    self.check_expr(val);
                }
            }
            Stmt::If(if_expr) => {
                self.check_expr(&Expr::If(Box::new(if_expr.clone())));
            }
            Stmt::For(f) => {
                let iter_ty = self.check_expr(&f.iter);
                let elem_ty = match iter_ty {
                    Type::Array(inner) => *inner,
                    _ => Type::Any,
                };
                let parent = std::mem::replace(&mut self.scope, Scope::new());
                self.scope = Scope::child(parent);
                self.scope.define(
                    &f.binding,
                    Symbol {
                        ty: elem_ty,
                        mutable: false,
                    },
                );
                self.check_block(&f.body);
                let child = std::mem::replace(&mut self.scope, Scope::new());
                self.scope = *child.parent.unwrap();
            }
            Stmt::While(w) => {
                self.check_expr(&w.condition);
                self.check_block(&w.body);
            }
            Stmt::Match(m) => {
                self.check_match(m);
            }
            Stmt::TryCatch(tc) => {
                self.check_block(&tc.try_block);
                let parent = std::mem::replace(&mut self.scope, Scope::new());
                self.scope = Scope::child(parent);
                self.scope.define(
                    &tc.catch_binding,
                    Symbol {
                        ty: Type::Any,
                        mutable: false,
                    },
                );
                self.check_block(&tc.catch_block);
                let child = std::mem::replace(&mut self.scope, Scope::new());
                self.scope = *child.parent.unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_parser;

    fn check_src(src: &str) -> Vec<Diagnostic> {
        let parsed = ag_parser::parse(src);
        assert!(
            parsed.diagnostics.is_empty(),
            "parse errors: {:?}",
            parsed.diagnostics
        );
        let result = check(&parsed.module);
        result.diagnostics
    }

    fn assert_no_errors(src: &str) {
        let diags = check_src(src);
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
    }

    fn assert_has_error(src: &str, msg_contains: &str) {
        let diags = check_src(src);
        assert!(
            diags.iter().any(|d| d.message.contains(msg_contains)),
            "expected error containing '{}', got: {:?}",
            msg_contains,
            diags
        );
    }

    #[test]
    fn type_mismatch() {
        assert_has_error(r#"let x: int = "hello""#, "type mismatch");
    }

    #[test]
    fn int_to_num_widening() {
        assert_no_errors("let x: num = 42");
    }

    #[test]
    fn any_escapes_checking() {
        // any should be compatible with everything
        assert_no_errors("let x: any = 42");
    }

    #[test]
    fn infer_let_type() {
        assert_no_errors("let x = 42");
    }

    #[test]
    fn undefined_variable() {
        assert_has_error("fn f() -> int { y }", "undefined variable `y`");
    }

    #[test]
    fn duplicate_binding() {
        assert_has_error("let x = 1\nlet x = 2", "duplicate binding `x`");
    }

    #[test]
    fn reassign_immutable() {
        assert_has_error("fn f() { let x = 1; x = 2 }", "cannot assign to immutable binding `x`");
    }

    #[test]
    fn reassign_mutable() {
        assert_no_errors("fn f() { mut x = 1; x = 2 }");
    }

    #[test]
    fn nullable_assignment() {
        assert_no_errors("let x: str? = nil");
    }

    #[test]
    fn return_type_mismatch() {
        assert_has_error(
            r#"fn foo() -> int { "hello" }"#,
            "return type mismatch",
        );
    }

    #[test]
    fn valid_function_return() {
        assert_no_errors("fn add(a: int, b: int) -> int { a + b }");
    }

    // ── DSL capture tests ──

    #[test]
    fn dsl_valid_capture() {
        assert_no_errors("let role: str = \"admin\"\n@prompt sys ```\nYou are #{role}.\n```\n");
    }

    #[test]
    fn dsl_capture_undefined_var() {
        assert_has_error(
            "@prompt sys ```\n#{undefined_var}\n```\n",
            "undefined variable",
        );
    }

    #[test]
    fn dsl_capture_type_not_constrained() {
        // Any type should be accepted in a capture — no type constraint error
        assert_no_errors("let count: int = 42\n@prompt sys ```\n#{count}\n```\n");
    }
}
