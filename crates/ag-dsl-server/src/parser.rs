use crate::ast::*;
use crate::lexer::ServerToken;

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

pub fn parse(tokens: &[ServerToken], name: &str) -> Result<ServerTemplate, Vec<Diagnostic>> {
    let mut parser = Parser::new(name, tokens);
    parser.parse_template()
}

struct Parser<'a> {
    name: String,
    tokens: &'a [ServerToken],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(name: &str, tokens: &'a [ServerToken]) -> Self {
        Self {
            name: name.to_string(),
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn peek(&self) -> &ServerToken {
        self.tokens.get(self.pos).unwrap_or(&ServerToken::Eof)
    }

    fn advance(&mut self) -> &ServerToken {
        let tok = self.tokens.get(self.pos).unwrap_or(&ServerToken::Eof);
        self.pos += 1;
        tok
    }

    fn parse_template(&mut self) -> Result<ServerTemplate, Vec<Diagnostic>> {
        let mut port: Option<u16> = None;
        let mut host: Option<String> = None;
        let mut middlewares: Vec<usize> = Vec::new();
        let mut routes: Vec<Route> = Vec::new();

        loop {
            match self.peek().clone() {
                ServerToken::Eof => break,
                ServerToken::DirectivePort => {
                    self.advance();
                    match self.peek() {
                        ServerToken::NumberLiteral(n) => {
                            let n = *n;
                            self.advance();
                            if n > 65535 {
                                self.diagnostics.push(Diagnostic {
                                    message: format!("port {} exceeds maximum 65535", n),
                                    severity: Severity::Error,
                                });
                            } else {
                                port = Some(n as u16);
                            }
                        }
                        _ => {
                            self.diagnostics.push(Diagnostic {
                                message: "expected port number after @port".to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
                }
                ServerToken::DirectiveHost => {
                    self.advance();
                    match self.peek() {
                        ServerToken::StringLiteral(s) => {
                            let s = s.clone();
                            self.advance();
                            host = Some(s);
                        }
                        _ => {
                            self.diagnostics.push(Diagnostic {
                                message: "expected string literal after @host".to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
                }
                ServerToken::DirectiveMiddleware => {
                    self.advance();
                    match self.peek() {
                        ServerToken::Capture(idx) => {
                            let idx = *idx;
                            self.advance();
                            middlewares.push(idx);
                        }
                        _ => {
                            self.diagnostics.push(Diagnostic {
                                message: "expected capture expression after @middleware".to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
                }
                ServerToken::DirectiveGet
                | ServerToken::DirectivePost
                | ServerToken::DirectivePut
                | ServerToken::DirectiveDelete
                | ServerToken::DirectivePatch => {
                    let method_token = self.advance().clone();
                    let method = match method_token {
                        ServerToken::DirectiveGet => HttpMethod::Get,
                        ServerToken::DirectivePost => HttpMethod::Post,
                        ServerToken::DirectivePut => HttpMethod::Put,
                        ServerToken::DirectiveDelete => HttpMethod::Delete,
                        ServerToken::DirectivePatch => HttpMethod::Patch,
                        _ => unreachable!(),
                    };

                    // Expect Path token
                    let path_segments = match self.peek() {
                        ServerToken::Path(p) => {
                            let p = p.clone();
                            self.advance();
                            match parse_path_segments(&p) {
                                Ok(segs) => segs,
                                Err(msg) => {
                                    self.diagnostics.push(Diagnostic {
                                        message: format!("invalid path: {}", msg),
                                        severity: Severity::Error,
                                    });
                                    Vec::new()
                                }
                            }
                        }
                        _ => {
                            self.diagnostics.push(Diagnostic {
                                message: format!("expected path after @{}", method_name(method)),
                                severity: Severity::Error,
                            });
                            Vec::new()
                        }
                    };

                    // Expect Capture token
                    match self.peek() {
                        ServerToken::Capture(idx) => {
                            let idx = *idx;
                            self.advance();
                            routes.push(Route {
                                method,
                                path: path_segments,
                                handler_capture: idx,
                            });
                        }
                        _ => {
                            self.diagnostics.push(Diagnostic {
                                message: format!(
                                    "expected handler capture after @{} route path",
                                    method_name(method)
                                ),
                                severity: Severity::Error,
                            });
                        }
                    }
                }
                // Stray tokens — skip
                _ => {
                    self.advance();
                }
            }
        }

        if !self.diagnostics.is_empty() {
            return Err(self.diagnostics.clone());
        }

        Ok(ServerTemplate {
            name: self.name.clone(),
            port,
            host,
            middlewares,
            routes,
        })
    }
}

fn method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Delete => "delete",
        HttpMethod::Patch => "patch",
    }
}

fn parse_path_segments(path: &str) -> Result<Vec<PathSegment>, String> {
    if path == "/" {
        return Ok(Vec::new());
    }
    if !path.starts_with('/') {
        return Err("path must start with /".into());
    }
    let parts: Vec<&str> = path[1..].split('/').collect();
    let mut segments = Vec::new();
    for part in parts {
        if part.is_empty() {
            return Err("empty path segment".into());
        }
        if part == "*" {
            segments.push(PathSegment::Wildcard);
        } else if part.starts_with(':') {
            segments.push(PathSegment::Param(part[1..].to_string()));
        } else {
            segments.push(PathSegment::Literal(part.to_string()));
        }
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_server() {
        let tokens = vec![
            ServerToken::DirectivePort,
            ServerToken::NumberLiteral(3000),
            ServerToken::DirectiveGet,
            ServerToken::Path("/health".into()),
            ServerToken::Capture(0),
            ServerToken::DirectivePost,
            ServerToken::Path("/users".into()),
            ServerToken::Capture(1),
            ServerToken::Eof,
        ];
        let tpl = parse(&tokens, "api").unwrap();
        assert_eq!(tpl.name, "api");
        assert_eq!(tpl.port, Some(3000));
        assert_eq!(tpl.routes.len(), 2);
        assert_eq!(tpl.routes[0].method, HttpMethod::Get);
        assert_eq!(
            tpl.routes[0].path,
            vec![PathSegment::Literal("health".into())]
        );
        assert_eq!(tpl.routes[0].handler_capture, 0);
        assert_eq!(tpl.routes[1].method, HttpMethod::Post);
        assert_eq!(
            tpl.routes[1].path,
            vec![PathSegment::Literal("users".into())]
        );
        assert_eq!(tpl.routes[1].handler_capture, 1);
    }

    #[test]
    fn parse_server_with_middleware() {
        let tokens = vec![
            ServerToken::DirectivePort,
            ServerToken::NumberLiteral(8080),
            ServerToken::DirectiveMiddleware,
            ServerToken::Capture(0),
            ServerToken::DirectiveMiddleware,
            ServerToken::Capture(1),
            ServerToken::DirectiveGet,
            ServerToken::Path("/".into()),
            ServerToken::Capture(2),
            ServerToken::Eof,
        ];
        let tpl = parse(&tokens, "app").unwrap();
        assert_eq!(tpl.middlewares, vec![0, 1]);
        assert_eq!(tpl.routes.len(), 1);
    }

    #[test]
    fn parse_path_segments_literals() {
        let segs = parse_path_segments("/users/list").unwrap();
        assert_eq!(
            segs,
            vec![
                PathSegment::Literal("users".into()),
                PathSegment::Literal("list".into()),
            ]
        );
    }

    #[test]
    fn parse_path_segments_params() {
        let segs = parse_path_segments("/users/:id/posts/:pid").unwrap();
        assert_eq!(
            segs,
            vec![
                PathSegment::Literal("users".into()),
                PathSegment::Param("id".into()),
                PathSegment::Literal("posts".into()),
                PathSegment::Param("pid".into()),
            ]
        );
    }

    #[test]
    fn parse_path_segments_wildcard() {
        let segs = parse_path_segments("/files/*").unwrap();
        assert_eq!(
            segs,
            vec![
                PathSegment::Literal("files".into()),
                PathSegment::Wildcard,
            ]
        );
    }

    #[test]
    fn parse_path_segments_root() {
        let segs = parse_path_segments("/").unwrap();
        assert!(segs.is_empty());
    }

    #[test]
    fn parse_path_segments_error_no_leading_slash() {
        let result = parse_path_segments("users");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with /"));
    }

    #[test]
    fn parse_path_segments_error_empty_segment() {
        let result = parse_path_segments("/users//list");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty path segment"));
    }

    #[test]
    fn parse_error_missing_capture_after_route() {
        let tokens = vec![
            ServerToken::DirectiveGet,
            ServerToken::Path("/health".into()),
            ServerToken::Eof,
        ];
        let result = parse(&tokens, "api");
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("expected handler capture"));
    }

    #[test]
    fn parse_error_missing_port_number() {
        let tokens = vec![
            ServerToken::DirectivePort,
            ServerToken::DirectiveGet,
            ServerToken::Path("/health".into()),
            ServerToken::Capture(0),
            ServerToken::Eof,
        ];
        let result = parse(&tokens, "api");
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("expected port number"));
    }

    #[test]
    fn parse_server_with_host() {
        let tokens = vec![
            ServerToken::DirectivePort,
            ServerToken::NumberLiteral(3000),
            ServerToken::DirectiveHost,
            ServerToken::StringLiteral("0.0.0.0".into()),
            ServerToken::DirectiveGet,
            ServerToken::Path("/".into()),
            ServerToken::Capture(0),
            ServerToken::Eof,
        ];
        let tpl = parse(&tokens, "api").unwrap();
        assert_eq!(tpl.host, Some("0.0.0.0".into()));
    }

    #[test]
    fn parse_all_http_methods() {
        let tokens = vec![
            ServerToken::DirectiveGet,
            ServerToken::Path("/a".into()),
            ServerToken::Capture(0),
            ServerToken::DirectivePost,
            ServerToken::Path("/b".into()),
            ServerToken::Capture(1),
            ServerToken::DirectivePut,
            ServerToken::Path("/c".into()),
            ServerToken::Capture(2),
            ServerToken::DirectiveDelete,
            ServerToken::Path("/d".into()),
            ServerToken::Capture(3),
            ServerToken::DirectivePatch,
            ServerToken::Path("/e".into()),
            ServerToken::Capture(4),
            ServerToken::Eof,
        ];
        let tpl = parse(&tokens, "api").unwrap();
        assert_eq!(tpl.routes.len(), 5);
        assert_eq!(tpl.routes[0].method, HttpMethod::Get);
        assert_eq!(tpl.routes[1].method, HttpMethod::Post);
        assert_eq!(tpl.routes[2].method, HttpMethod::Put);
        assert_eq!(tpl.routes[3].method, HttpMethod::Delete);
        assert_eq!(tpl.routes[4].method, HttpMethod::Patch);
    }

    #[test]
    fn parse_error_port_exceeds_max() {
        let tokens = vec![
            ServerToken::DirectivePort,
            ServerToken::NumberLiteral(70000),
            ServerToken::Eof,
        ];
        let result = parse(&tokens, "api");
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("exceeds maximum"));
    }

    #[test]
    fn parse_error_missing_host_string() {
        let tokens = vec![
            ServerToken::DirectiveHost,
            ServerToken::NumberLiteral(123),
            ServerToken::Eof,
        ];
        let result = parse(&tokens, "api");
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("expected string literal"));
    }

    #[test]
    fn parse_error_missing_middleware_capture() {
        let tokens = vec![
            ServerToken::DirectiveMiddleware,
            ServerToken::DirectiveGet,
            ServerToken::Path("/".into()),
            ServerToken::Capture(0),
            ServerToken::Eof,
        ];
        let result = parse(&tokens, "api");
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("expected capture expression"));
    }
}
