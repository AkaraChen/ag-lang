use crate::ast::*;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

pub fn validate(template: &ServerTemplate) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check: no routes defined
    if template.routes.is_empty() {
        diagnostics.push(Diagnostic {
            message: "no routes defined".to_string(),
            severity: Severity::Warning,
        });
    }

    // Check: port == 0
    if let Some(port) = template.port {
        if port == 0 {
            diagnostics.push(Diagnostic {
                message: "port must not be 0".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Check: duplicate routes (same method + same path segments)
    for i in 0..template.routes.len() {
        for j in (i + 1)..template.routes.len() {
            let a = &template.routes[i];
            let b = &template.routes[j];
            if a.method == b.method && a.path == b.path {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "duplicate route: {} {}",
                        method_name(a.method),
                        format_path(&a.path)
                    ),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Check: wildcard not in last position
    for route in &template.routes {
        for (i, seg) in route.path.iter().enumerate() {
            if *seg == PathSegment::Wildcard && i != route.path.len() - 1 {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "wildcard must be the last path segment in {} {}",
                        method_name(route.method),
                        format_path(&route.path)
                    ),
                    severity: Severity::Error,
                });
            }
        }
    }

    diagnostics
}

fn method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Patch => "PATCH",
    }
}

fn format_path(segments: &[PathSegment]) -> String {
    if segments.is_empty() {
        return "/".to_string();
    }
    let mut path = String::new();
    for seg in segments {
        path.push('/');
        match seg {
            PathSegment::Literal(s) => path.push_str(s),
            PathSegment::Param(s) => {
                path.push(':');
                path.push_str(s);
            }
            PathSegment::Wildcard => path.push('*'),
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_route(method: HttpMethod, path: Vec<PathSegment>, capture: usize) -> Route {
        Route {
            method,
            path,
            handler_capture: capture,
        }
    }

    #[test]
    fn valid_server_passes() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: Some(3000),
            host: None,
            middlewares: vec![],
            routes: vec![
                make_route(
                    HttpMethod::Get,
                    vec![PathSegment::Literal("health".into())],
                    0,
                ),
                make_route(
                    HttpMethod::Post,
                    vec![PathSegment::Literal("users".into())],
                    1,
                ),
            ],
        };
        let diags = validate(&tpl);
        assert!(diags.iter().all(|d| d.severity != Severity::Error));
        assert!(diags.iter().all(|d| d.severity != Severity::Warning));
    }

    #[test]
    fn no_routes_warning() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: Some(3000),
            host: None,
            middlewares: vec![],
            routes: vec![],
        };
        let diags = validate(&tpl);
        assert!(diags.iter().any(|d| d.message.contains("no routes defined")
            && d.severity == Severity::Warning));
    }

    #[test]
    fn duplicate_routes_error() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: Some(3000),
            host: None,
            middlewares: vec![],
            routes: vec![
                make_route(
                    HttpMethod::Get,
                    vec![PathSegment::Literal("users".into())],
                    0,
                ),
                make_route(
                    HttpMethod::Get,
                    vec![PathSegment::Literal("users".into())],
                    1,
                ),
            ],
        };
        let diags = validate(&tpl);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("duplicate route") && d.severity == Severity::Error));
    }

    #[test]
    fn duplicate_routes_different_methods_ok() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: Some(3000),
            host: None,
            middlewares: vec![],
            routes: vec![
                make_route(
                    HttpMethod::Get,
                    vec![PathSegment::Literal("users".into())],
                    0,
                ),
                make_route(
                    HttpMethod::Post,
                    vec![PathSegment::Literal("users".into())],
                    1,
                ),
            ],
        };
        let diags = validate(&tpl);
        assert!(diags.iter().all(|d| d.severity != Severity::Error));
    }

    #[test]
    fn wildcard_not_last_error() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: Some(3000),
            host: None,
            middlewares: vec![],
            routes: vec![make_route(
                HttpMethod::Get,
                vec![
                    PathSegment::Wildcard,
                    PathSegment::Literal("files".into()),
                ],
                0,
            )],
        };
        let diags = validate(&tpl);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("wildcard must be the last") && d.severity == Severity::Error));
    }

    #[test]
    fn wildcard_in_last_position_ok() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: Some(3000),
            host: None,
            middlewares: vec![],
            routes: vec![make_route(
                HttpMethod::Get,
                vec![
                    PathSegment::Literal("files".into()),
                    PathSegment::Wildcard,
                ],
                0,
            )],
        };
        let diags = validate(&tpl);
        assert!(diags.iter().all(|d| d.severity != Severity::Error));
    }

    #[test]
    fn port_zero_error() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: Some(0),
            host: None,
            middlewares: vec![],
            routes: vec![make_route(
                HttpMethod::Get,
                vec![PathSegment::Literal("health".into())],
                0,
            )],
        };
        let diags = validate(&tpl);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("port must not be 0") && d.severity == Severity::Error));
    }

    #[test]
    fn no_port_no_error() {
        let tpl = ServerTemplate {
            name: "api".to_string(),
            port: None,
            host: None,
            middlewares: vec![],
            routes: vec![make_route(
                HttpMethod::Get,
                vec![PathSegment::Literal("health".into())],
                0,
            )],
        };
        let diags = validate(&tpl);
        assert!(diags.iter().all(|d| d.severity != Severity::Error));
    }
}
