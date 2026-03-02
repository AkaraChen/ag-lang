use ag_ast::JsonSchema;
use ag_dsl_core::swc_helpers::{str_lit, bool_lit, make_prop};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

/// Convert a `JsonSchema` to a JSON Schema expression (SWC AST).
pub fn schema_to_expr(schema: &JsonSchema) -> swc::Expr {
    match schema {
        JsonSchema::String => obj(&[make_prop("type", str_lit("string"))]),
        JsonSchema::Number => obj(&[make_prop("type", str_lit("number"))]),
        JsonSchema::Integer => obj(&[make_prop("type", str_lit("integer"))]),
        JsonSchema::Boolean => obj(&[make_prop("type", str_lit("boolean"))]),
        JsonSchema::Null => obj(&[make_prop("type", str_lit("null"))]),
        JsonSchema::Any => obj(&[]),
        JsonSchema::Array(inner) => obj(&[
            make_prop("type", str_lit("array")),
            make_prop("items", schema_to_expr(inner)),
        ]),
        JsonSchema::Object { properties, required, additional_properties } => {
            let props: Vec<swc::PropOrSpread> = properties
                .iter()
                .map(|(name, schema)| make_prop(name, schema_to_expr(schema)))
                .collect();
            let mut schema_props = vec![
                make_prop("type", str_lit("object")),
                make_prop("properties", obj(&props)),
            ];
            if !required.is_empty() {
                schema_props.push(make_prop("required", swc::Expr::Array(swc::ArrayLit {
                    span: DUMMY_SP,
                    elems: required.iter().map(|name| Some(swc::ExprOrSpread {
                        spread: None,
                        expr: Box::new(str_lit(name)),
                    })).collect(),
                })));
            }
            if let Some(additional) = additional_properties {
                schema_props.push(make_prop("additionalProperties", schema_to_expr(additional)));
            }
            obj(&schema_props)
        }
        JsonSchema::AnyOf(schemas) => {
            obj(&[make_prop("anyOf", swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: schemas.iter().map(|s| Some(swc::ExprOrSpread {
                    spread: None,
                    expr: Box::new(schema_to_expr(s)),
                })).collect(),
            }))])
        }
    }
}

/// Build the full tool schema object:
/// `{ name, description?, parameters: { type: "object", properties, required, additionalProperties: false } }`
pub fn build_tool_schema(
    fn_name: &str,
    description: &Option<String>,
    params: &[(String, JsonSchema)],
) -> swc::Expr {
    let mut properties = Vec::new();
    let mut required = Vec::new();

    for (name, schema) in params {
        properties.push(make_prop(name, schema_to_expr(schema)));
        // All params are required at this level; nullability is already
        // handled by the checker stripping Nullable from JsonSchema
        required.push(Some(swc::ExprOrSpread {
            spread: None,
            expr: Box::new(str_lit(name)),
        }));
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

    fn schema_to_js(schema: &JsonSchema) -> String {
        let expr = schema_to_expr(schema);
        let stmt = swc::ModuleItem::Stmt(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(expr),
        }));
        emit_module(&[stmt])
    }

    fn tool_schema_to_js(name: &str, desc: &Option<String>, params: &[(String, JsonSchema)]) -> String {
        let expr = build_tool_schema(name, desc, params);
        let stmt = swc::ModuleItem::Stmt(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(expr),
        }));
        emit_module(&[stmt])
    }

    #[test]
    fn schema_primitives() {
        let js = schema_to_js(&JsonSchema::String);
        assert!(js.contains(r#""string""#));
        let js = schema_to_js(&JsonSchema::Number);
        assert!(js.contains(r#""number""#));
        let js = schema_to_js(&JsonSchema::Integer);
        assert!(js.contains(r#""integer""#));
        let js = schema_to_js(&JsonSchema::Boolean);
        assert!(js.contains(r#""boolean""#));
    }

    #[test]
    fn schema_array() {
        let js = schema_to_js(&JsonSchema::Array(Box::new(JsonSchema::String)));
        assert!(js.contains(r#""array""#));
        assert!(js.contains("items"));
        assert!(js.contains(r#""string""#));
    }

    #[test]
    fn schema_object() {
        let schema = JsonSchema::Object {
            properties: vec![
                ("name".into(), JsonSchema::String),
                ("age".into(), JsonSchema::Integer),
            ],
            required: vec!["name".into(), "age".into()],
            additional_properties: None,
        };
        let js = schema_to_js(&schema);
        assert!(js.contains(r#""object""#));
        assert!(js.contains("properties"));
        assert!(js.contains("name"));
        assert!(js.contains("age"));
        assert!(js.contains("required"));
    }

    #[test]
    fn schema_object_with_additional_properties() {
        let schema = JsonSchema::Object {
            properties: vec![],
            required: vec![],
            additional_properties: Some(Box::new(JsonSchema::Number)),
        };
        let js = schema_to_js(&schema);
        assert!(js.contains(r#""object""#));
        assert!(js.contains("additionalProperties"));
        assert!(js.contains(r#""number""#));
    }

    #[test]
    fn schema_any_of() {
        let schema = JsonSchema::AnyOf(vec![JsonSchema::String, JsonSchema::Number]);
        let js = schema_to_js(&schema);
        assert!(js.contains("anyOf"));
        assert!(js.contains(r#""string""#));
        assert!(js.contains(r#""number""#));
    }

    #[test]
    fn schema_any() {
        let js = schema_to_js(&JsonSchema::Any);
        assert!(js.contains("{}"));
    }

    #[test]
    fn build_tool_schema_with_description() {
        let js = tool_schema_to_js(
            "lookup_docs",
            &Some("Look up documentation".into()),
            &[("topic".into(), JsonSchema::String)],
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
            &[("a".into(), JsonSchema::Number), ("b".into(), JsonSchema::Number)],
        );
        assert!(js.contains(r#""calculate""#));
        assert!(!js.contains("description"));
        assert!(js.contains("parameters"));
    }

    #[test]
    fn build_tool_schema_all_params_required() {
        let js = tool_schema_to_js(
            "search",
            &None,
            &[
                ("query".into(), JsonSchema::String),
                ("limit".into(), JsonSchema::Integer),
            ],
        );
        assert!(js.contains("query"));
        assert!(js.contains("limit"));
        assert!(js.contains("additionalProperties"));
    }
}
