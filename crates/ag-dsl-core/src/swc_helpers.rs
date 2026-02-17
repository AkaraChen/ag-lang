use swc_common::DUMMY_SP;
use swc_common::SyntaxContext;
use swc_ecma_ast as swc;

pub fn ident(name: &str) -> swc::Ident {
    swc::Ident {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        sym: name.into(),
        optional: false,
    }
}

pub fn binding_ident(name: &str) -> swc::BindingIdent {
    swc::BindingIdent {
        id: ident(name),
        type_ann: None,
    }
}

pub fn str_lit(s: &str) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Str(swc::Str {
        span: DUMMY_SP,
        value: s.into(),
        raw: None,
    }))
}

pub fn num_lit(n: f64) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Num(swc::Number {
        span: DUMMY_SP,
        value: n,
        raw: None,
    }))
}

pub fn bool_lit(b: bool) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
        span: DUMMY_SP,
        value: b,
    }))
}

pub fn expr_or_spread(expr: swc::Expr) -> swc::ExprOrSpread {
    swc::ExprOrSpread {
        spread: None,
        expr: Box::new(expr),
    }
}

pub fn make_prop(key: &str, value: swc::Expr) -> swc::PropOrSpread {
    swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
        key: swc::PropName::Ident(swc::IdentName {
            span: DUMMY_SP,
            sym: key.into(),
        }),
        value: Box::new(value),
    })))
}

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
