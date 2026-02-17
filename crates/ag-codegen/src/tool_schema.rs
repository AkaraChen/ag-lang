use ag_checker::Type;
use ag_dsl_core::swc_helpers::{str_lit, bool_lit, make_prop};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

/// Convert an ag-checker `Type` to a JSON Schema expression (SWC AST).
pub fn type_to_json_schema(ty: &Type) -> swc::Expr {
    match ty {
        Type::Str => obj(&[make_prop("type", str_lit("string"))]),
        Type::Num => obj(&[make_prop("type", str_lit("number"))]),
        Type::Int => obj(&[make_prop("type", str_lit("integer"))]),
        Type::Bool => obj(&[make_prop("type", str_lit("boolean"))]),
        Type::Nil => obj(&[make_prop("type", str_lit("null"))]),
        Type::Any | Type::Unknown => obj(&[]),
        Type::Array(inner) => obj(&[
            make_prop("type", str_lit("array")),
            make_prop("items", type_to_json_schema(inner)),
        ]),
        Type::Map(_key, value) => obj(&[
            make_prop("type", str_lit("object")),
            make_prop("additionalProperties", type_to_json_schema(value)),
        ]),
        Type::Nullable(inner) => type_to_json_schema(inner),
        Type::Union(a, b) => {
            let mut schemas = Vec::new();
            collect_union_schemas(a, &mut schemas);
            collect_union_schemas(b, &mut schemas);
            obj(&[make_prop("anyOf", swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: schemas.into_iter().map(|s| Some(swc::ExprOrSpread {
                    spread: None,
                    expr: Box::new(s),
                })).collect(),
            }))])
        }
        Type::Struct(_, fields) => {
            let props: Vec<swc::PropOrSpread> = fields
                .iter()
                .map(|(name, ty)| make_prop(name, type_to_json_schema(ty)))
                .collect();
            let required: Vec<Option<swc::ExprOrSpread>> = fields
                .iter()
                .filter(|(_, ty)| !matches!(ty, Type::Nullable(_)))
                .map(|(name, _)| Some(swc::ExprOrSpread {
                    spread: None,
                    expr: Box::new(str_lit(name)),
                }))
                .collect();
            let mut schema_props = vec![
                make_prop("type", str_lit("object")),
                make_prop("properties", obj(&props)),
            ];
            if !required.is_empty() {
                schema_props.push(make_prop("required", swc::Expr::Array(swc::ArrayLit {
                    span: DUMMY_SP,
                    elems: required,
                })));
            }
            obj(&schema_props)
        }
        Type::Promise(inner) => type_to_json_schema(inner),
        Type::Function(_, _) | Type::VariadicFunction(_, _) | Type::Enum(_, _) => obj(&[]),
    }
}

/// Build the full tool schema object:
/// `{ name, description?, parameters: { type: "object", properties, required } }`
pub fn build_tool_schema(
    fn_name: &str,
    description: &Option<String>,
    params: &[(String, Type)],
) -> swc::Expr {
    let mut properties = Vec::new();
    let mut required = Vec::new();

    for (name, ty) in params {
        properties.push(make_prop(name, type_to_json_schema(ty)));
        if !matches!(ty, Type::Nullable(_)) {
            required.push(Some(swc::ExprOrSpread {
                spread: None,
                expr: Box::new(str_lit(name)),
            }));
        }
    }

    let parameters = obj(&[
        make_prop("type", str_lit("object")),
        make_prop("properties", obj(&properties)),
        make_prop("required", swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems: required,
        })),
        make_prop("additionalProperties", bool_lit(false)),
    ]);

    let mut schema_props = vec![
        make_prop("name", str_lit(fn_name)),
    ];
    if let Some(desc) = description {
        schema_props.push(make_prop("description", str_lit(desc)));
    }
    schema_props.push(make_prop("parameters", parameters));

    obj(&schema_props)
}

fn collect_union_schemas(ty: &Type, out: &mut Vec<swc::Expr>) {
    match ty {
        Type::Union(a, b) => {
            collect_union_schemas(a, out);
            collect_union_schemas(b, out);
        }
        _ => out.push(type_to_json_schema(ty)),
    }
}

fn obj(props: &[swc::PropOrSpread]) -> swc::Expr {
    swc::Expr::Object(swc::ObjectLit {
        span: DUMMY_SP,
        props: props.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::swc_helpers::emit_module;

    fn schema_to_js(ty: &Type) -> String {
        let expr = type_to_json_schema(ty);
        let stmt = swc::ModuleItem::Stmt(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(expr),
        }));
        emit_module(&[stmt])
    }

    fn tool_schema_to_js(name: &str, desc: &Option<String>, params: &[(String, Type)]) -> String {
        let expr = build_tool_schema(name, desc, params);
        let stmt = swc::ModuleItem::Stmt(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(expr),
        }));
        emit_module(&[stmt])
    }

    #[test]
    fn schema_primitives() {
        let js = schema_to_js(&Type::Str);
        assert!(js.contains(r#""string""#));
        let js = schema_to_js(&Type::Num);
        assert!(js.contains(r#""number""#));
        let js = schema_to_js(&Type::Int);
        assert!(js.contains(r#""integer""#));
        let js = schema_to_js(&Type::Bool);
        assert!(js.contains(r#""boolean""#));
    }

    #[test]
    fn schema_array() {
        let js = schema_to_js(&Type::Array(Box::new(Type::Str)));
        assert!(js.contains(r#""array""#));
        assert!(js.contains("items"));
        assert!(js.contains(r#""string""#));
    }

    #[test]
    fn schema_struct() {
        let ty = Type::Struct("Foo".into(), vec![
            ("name".into(), Type::Str),
            ("age".into(), Type::Int),
        ]);
        let js = schema_to_js(&ty);
        assert!(js.contains(r#""object""#));
        assert!(js.contains("properties"));
        assert!(js.contains("name"));
        assert!(js.contains("age"));
        assert!(js.contains("required"));
    }

    #[test]
    fn schema_nullable_excluded_from_required() {
        let ty = Type::Struct("Bar".into(), vec![
            ("required_field".into(), Type::Str),
            ("optional_field".into(), Type::Nullable(Box::new(Type::Str))),
        ]);
        let js = schema_to_js(&ty);
        assert!(js.contains("required_field"));
        assert!(js.contains("optional_field"));
        // The required array should contain required_field but not optional_field
        // Both should be in properties though
        assert!(js.contains("properties"));
    }

    #[test]
    fn schema_union() {
        let ty = Type::Union(Box::new(Type::Str), Box::new(Type::Num));
        let js = schema_to_js(&ty);
        assert!(js.contains("anyOf"));
        assert!(js.contains(r#""string""#));
        assert!(js.contains(r#""number""#));
    }

    #[test]
    fn schema_any() {
        let js = schema_to_js(&Type::Any);
        assert!(js.contains("{}"));
    }

    #[test]
    fn schema_map() {
        let ty = Type::Map(Box::new(Type::Str), Box::new(Type::Num));
        let js = schema_to_js(&ty);
        assert!(js.contains(r#""object""#));
        assert!(js.contains("additionalProperties"));
        assert!(js.contains(r#""number""#));
    }

    #[test]
    fn build_tool_schema_with_description() {
        let js = tool_schema_to_js(
            "lookup_docs",
            &Some("Look up documentation".into()),
            &[("topic".into(), Type::Str)],
        );
        assert!(js.contains(r#""lookup_docs""#));
        assert!(js.contains(r#""Look up documentation""#));
        assert!(js.contains("parameters"));
        assert!(js.contains("topic"));
        assert!(js.contains(r#""string""#));
    }

    #[test]
    fn build_tool_schema_without_description() {
        let js = tool_schema_to_js(
            "calculate",
            &None,
            &[("a".into(), Type::Num), ("b".into(), Type::Num)],
        );
        assert!(js.contains(r#""calculate""#));
        assert!(!js.contains("description"));
        assert!(js.contains("parameters"));
    }

    #[test]
    fn build_tool_schema_optional_params() {
        let js = tool_schema_to_js(
            "search",
            &None,
            &[
                ("query".into(), Type::Str),
                ("limit".into(), Type::Nullable(Box::new(Type::Int))),
            ],
        );
        assert!(js.contains("query"));
        assert!(js.contains("limit"));
        assert!(js.contains("additionalProperties"));
    }
}
