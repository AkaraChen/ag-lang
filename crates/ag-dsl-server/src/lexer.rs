use ag_dsl_core::DslPart;

#[derive(Debug, Clone, PartialEq)]
pub enum ServerToken {
    DirectivePort,
    DirectiveHost,
    DirectiveMiddleware,
    DirectiveGet,
    DirectivePost,
    DirectivePut,
    DirectiveDelete,
    DirectivePatch,
    NumberLiteral(u64),
    StringLiteral(String),
    Path(String),
    Capture(usize),
    Eof,
}

pub fn lex(parts: &[DslPart]) -> Vec<ServerToken> {
    let mut tokens = Vec::new();
    let mut capture_index = 0;

    for part in parts {
        match part {
            DslPart::Text(text, _) => {
                lex_text(text, &mut tokens);
            }
            DslPart::Capture(_, _) => {
                tokens.push(ServerToken::Capture(capture_index));
                capture_index += 1;
            }
        }
    }

    tokens.push(ServerToken::Eof);
    tokens
}

fn lex_text(text: &str, tokens: &mut Vec<ServerToken>) {
    let mut chars = text.chars().peekable();
    let mut at_line_start = true;

    while let Some(&ch) = chars.peek() {
        if ch == '@' && at_line_start {
            // Try to recognize a directive
            let mut lookahead = String::new();
            let mut chars_clone = chars.clone();
            chars_clone.next(); // skip '@'
            while let Some(&c) = chars_clone.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    lookahead.push(c);
                    chars_clone.next();
                } else {
                    break;
                }
            }

            match lookahead.as_str() {
                "port" => {
                    chars.next(); // skip '@'
                    for _ in 0..4 {
                        chars.next();
                    } // skip 'port'
                    tokens.push(ServerToken::DirectivePort);
                    lex_number_literal(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "host" => {
                    chars.next(); // skip '@'
                    for _ in 0..4 {
                        chars.next();
                    } // skip 'host'
                    tokens.push(ServerToken::DirectiveHost);
                    lex_string_literal(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "middleware" => {
                    chars.next(); // skip '@'
                    for _ in 0..10 {
                        chars.next();
                    } // skip 'middleware'
                    tokens.push(ServerToken::DirectiveMiddleware);
                    // Capture follows as separate DslPart
                    skip_to_newline(&mut chars);
                    at_line_start = true;
                    continue;
                }
                "get" => {
                    chars.next(); // skip '@'
                    for _ in 0..3 {
                        chars.next();
                    } // skip 'get'
                    tokens.push(ServerToken::DirectiveGet);
                    lex_path(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "post" => {
                    chars.next(); // skip '@'
                    for _ in 0..4 {
                        chars.next();
                    } // skip 'post'
                    tokens.push(ServerToken::DirectivePost);
                    lex_path(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "put" => {
                    chars.next(); // skip '@'
                    for _ in 0..3 {
                        chars.next();
                    } // skip 'put'
                    tokens.push(ServerToken::DirectivePut);
                    lex_path(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "delete" => {
                    chars.next(); // skip '@'
                    for _ in 0..6 {
                        chars.next();
                    } // skip 'delete'
                    tokens.push(ServerToken::DirectiveDelete);
                    lex_path(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "patch" => {
                    chars.next(); // skip '@'
                    for _ in 0..5 {
                        chars.next();
                    } // skip 'patch'
                    tokens.push(ServerToken::DirectivePatch);
                    lex_path(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                _ => {
                    // Unknown directive, skip the line
                    chars.next(); // skip '@'
                    skip_to_newline(&mut chars);
                    at_line_start = true;
                    continue;
                }
            }
        }

        if ch == '\n' {
            chars.next();
            at_line_start = true;
        } else {
            chars.next();
            if ch != ' ' && ch != '\t' && ch != '\r' {
                at_line_start = false;
            }
        }
    }
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&c) = chars.peek() {
        if c == ' ' || c == '\t' {
            chars.next();
        } else {
            break;
        }
    }
}

fn skip_to_newline(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&c) = chars.peek() {
        if c == '\n' {
            chars.next();
            break;
        }
        chars.next();
    }
}

fn lex_number_literal(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    tokens: &mut Vec<ServerToken>,
) {
    skip_whitespace(chars);
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if let Ok(n) = num_str.parse::<u64>() {
        tokens.push(ServerToken::NumberLiteral(n));
    }
    skip_to_newline(chars);
}

fn lex_string_literal(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    tokens: &mut Vec<ServerToken>,
) {
    skip_whitespace(chars);
    if chars.peek() != Some(&'"') {
        skip_to_newline(chars);
        return;
    }
    chars.next(); // skip opening '"'
    let mut s = String::new();
    loop {
        match chars.peek() {
            None | Some(&'\n') => break,
            Some(&'"') => {
                chars.next();
                break;
            }
            Some(&'\\') => {
                chars.next();
                match chars.peek() {
                    Some(&'n') => {
                        s.push('\n');
                        chars.next();
                    }
                    Some(&'t') => {
                        s.push('\t');
                        chars.next();
                    }
                    Some(&'"') => {
                        s.push('"');
                        chars.next();
                    }
                    Some(&'\\') => {
                        s.push('\\');
                        chars.next();
                    }
                    Some(&c) => {
                        s.push('\\');
                        s.push(c);
                        chars.next();
                    }
                    None => {}
                }
            }
            Some(&c) => {
                s.push(c);
                chars.next();
            }
        }
    }
    tokens.push(ServerToken::StringLiteral(s));
    skip_to_newline(chars);
}

fn lex_path(chars: &mut std::iter::Peekable<std::str::Chars>, tokens: &mut Vec<ServerToken>) {
    skip_whitespace(chars);
    if chars.peek() != Some(&'/') {
        // No path found; skip to end of line
        skip_to_newline(chars);
        return;
    }
    let mut path = String::new();
    while let Some(&c) = chars.peek() {
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
            break;
        }
        path.push(c);
        chars.next();
    }
    tokens.push(ServerToken::Path(path));
    // Skip remaining whitespace on the line (capture follows as separate DslPart)
    skip_to_newline(chars);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::Span;

    fn make_text(s: &str) -> DslPart {
        DslPart::Text(s.to_string(), Span::dummy())
    }

    fn make_capture() -> DslPart {
        DslPart::Capture(Box::new(0u32), Span::dummy())
    }

    #[test]
    fn lex_port_directive() {
        let parts = vec![make_text("@port 3000\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectivePort);
        assert_eq!(tokens[1], ServerToken::NumberLiteral(3000));
        assert_eq!(tokens[2], ServerToken::Eof);
    }

    #[test]
    fn lex_host_directive() {
        let parts = vec![make_text("@host \"0.0.0.0\"\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectiveHost);
        assert_eq!(tokens[1], ServerToken::StringLiteral("0.0.0.0".into()));
        assert_eq!(tokens[2], ServerToken::Eof);
    }

    #[test]
    fn lex_get_with_path_and_capture() {
        let parts = vec![make_text("@get /health\n"), make_capture()];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectiveGet);
        assert_eq!(tokens[1], ServerToken::Path("/health".into()));
        assert_eq!(tokens[2], ServerToken::Capture(0));
        assert_eq!(tokens[3], ServerToken::Eof);
    }

    #[test]
    fn lex_post_with_param_path_and_capture() {
        let parts = vec![make_text("@post /users/:id\n"), make_capture()];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectivePost);
        assert_eq!(tokens[1], ServerToken::Path("/users/:id".into()));
        assert_eq!(tokens[2], ServerToken::Capture(0));
        assert_eq!(tokens[3], ServerToken::Eof);
    }

    #[test]
    fn lex_middleware_with_capture() {
        let parts = vec![make_text("@middleware "), make_capture()];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectiveMiddleware);
        assert_eq!(tokens[1], ServerToken::Capture(0));
        assert_eq!(tokens[2], ServerToken::Eof);
    }

    #[test]
    fn lex_multiple_directives() {
        let parts = vec![
            make_text("@port 8080\n@host \"127.0.0.1\"\n@get /health\n"),
            make_capture(),
            make_text("\n@post /users\n"),
            make_capture(),
        ];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectivePort);
        assert_eq!(tokens[1], ServerToken::NumberLiteral(8080));
        assert_eq!(tokens[2], ServerToken::DirectiveHost);
        assert_eq!(tokens[3], ServerToken::StringLiteral("127.0.0.1".into()));
        assert_eq!(tokens[4], ServerToken::DirectiveGet);
        assert_eq!(tokens[5], ServerToken::Path("/health".into()));
        assert_eq!(tokens[6], ServerToken::Capture(0));
        assert_eq!(tokens[7], ServerToken::DirectivePost);
        assert_eq!(tokens[8], ServerToken::Path("/users".into()));
        assert_eq!(tokens[9], ServerToken::Capture(1));
        assert_eq!(tokens[10], ServerToken::Eof);
    }

    #[test]
    fn lex_all_http_methods() {
        let parts = vec![
            make_text("@get /a\n"),
            make_capture(),
            make_text("\n@post /b\n"),
            make_capture(),
            make_text("\n@put /c\n"),
            make_capture(),
            make_text("\n@delete /d\n"),
            make_capture(),
            make_text("\n@patch /e\n"),
            make_capture(),
        ];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectiveGet);
        assert_eq!(tokens[1], ServerToken::Path("/a".into()));
        assert_eq!(tokens[2], ServerToken::Capture(0));
        assert_eq!(tokens[3], ServerToken::DirectivePost);
        assert_eq!(tokens[4], ServerToken::Path("/b".into()));
        assert_eq!(tokens[5], ServerToken::Capture(1));
        assert_eq!(tokens[6], ServerToken::DirectivePut);
        assert_eq!(tokens[7], ServerToken::Path("/c".into()));
        assert_eq!(tokens[8], ServerToken::Capture(2));
        assert_eq!(tokens[9], ServerToken::DirectiveDelete);
        assert_eq!(tokens[10], ServerToken::Path("/d".into()));
        assert_eq!(tokens[11], ServerToken::Capture(3));
        assert_eq!(tokens[12], ServerToken::DirectivePatch);
        assert_eq!(tokens[13], ServerToken::Path("/e".into()));
        assert_eq!(tokens[14], ServerToken::Capture(4));
        assert_eq!(tokens[15], ServerToken::Eof);
    }

    #[test]
    fn lex_host_with_escape() {
        let parts = vec![make_text("@host \"hello\\\"world\"\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectiveHost);
        assert_eq!(tokens[1], ServerToken::StringLiteral("hello\"world".into()));
    }

    #[test]
    fn lex_unknown_directive_ignored() {
        let parts = vec![make_text("@unknown something\n@port 3000\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], ServerToken::DirectivePort);
        assert_eq!(tokens[1], ServerToken::NumberLiteral(3000));
        assert_eq!(tokens[2], ServerToken::Eof);
    }
}
