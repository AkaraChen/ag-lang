use ag_dsl_core::DslPart;

#[derive(Debug, Clone, PartialEq)]
pub enum PromptToken {
    DirectiveRole(String),
    DirectiveModel,
    DirectiveExamples,
    DirectiveOutput,
    DirectiveConstraints,
    DirectiveMessages,

    Text(String),
    Capture(usize),

    BraceOpen,
    BraceClose,
    Colon,
    Pipe,
    StringLiteral(String),
    NumberLiteral(f64),
    ArrayOpen,
    ArrayClose,
    Ident(String),

    Eof,
}

pub fn lex(parts: &[DslPart]) -> Vec<PromptToken> {
    let mut tokens = Vec::new();
    let mut capture_index = 0;

    for part in parts {
        match part {
            DslPart::Text(text, _) => {
                lex_text(text, &mut tokens);
            }
            DslPart::Capture(_, _) => {
                tokens.push(PromptToken::Capture(capture_index));
                capture_index += 1;
            }
        }
    }

    tokens.push(PromptToken::Eof);
    tokens
}

fn lex_text(text: &str, tokens: &mut Vec<PromptToken>) {
    let mut chars = text.chars().peekable();
    let mut current_text = String::new();
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
                "role" => {
                    flush_text(&mut current_text, tokens);
                    chars.next(); // skip '@'
                    for _ in 0..4 {
                        chars.next();
                    } // skip 'role'
                    // Skip whitespace
                    while let Some(&c) = chars.peek() {
                        if c == ' ' || c == '\t' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // Read role name
                    let mut role_name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '\n' || c == '\r' {
                            break;
                        }
                        role_name.push(c);
                        chars.next();
                    }
                    // Skip newline
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    tokens.push(PromptToken::DirectiveRole(role_name.trim().to_string()));
                    at_line_start = true;
                    continue;
                }
                "model" => {
                    flush_text(&mut current_text, tokens);
                    chars.next(); // skip '@'
                    for _ in 0..5 {
                        chars.next();
                    } // skip 'model'
                    tokens.push(PromptToken::DirectiveModel);
                    // Parse the rest of the line for model names
                    lex_model_line(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "examples" => {
                    flush_text(&mut current_text, tokens);
                    chars.next();
                    for _ in 0..8 {
                        chars.next();
                    }
                    tokens.push(PromptToken::DirectiveExamples);
                    lex_brace_block(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "output" => {
                    flush_text(&mut current_text, tokens);
                    chars.next();
                    for _ in 0..6 {
                        chars.next();
                    }
                    tokens.push(PromptToken::DirectiveOutput);
                    // Check for inline { ... } or capture follows
                    skip_ws(&mut chars);
                    if chars.peek() == Some(&'{') {
                        lex_brace_block(&mut chars, tokens);
                    }
                    // If followed by capture, it will be a separate DslPart::Capture
                    at_line_start = true;
                    continue;
                }
                "constraints" => {
                    flush_text(&mut current_text, tokens);
                    chars.next();
                    for _ in 0..11 {
                        chars.next();
                    }
                    tokens.push(PromptToken::DirectiveConstraints);
                    lex_brace_block(&mut chars, tokens);
                    at_line_start = true;
                    continue;
                }
                "messages" => {
                    flush_text(&mut current_text, tokens);
                    chars.next();
                    for _ in 0..8 {
                        chars.next();
                    }
                    tokens.push(PromptToken::DirectiveMessages);
                    // Skip to end of line
                    skip_to_newline(&mut chars);
                    at_line_start = true;
                    continue;
                }
                _ => {
                    // Not a known directive, treat as text
                    current_text.push(ch);
                    chars.next();
                    at_line_start = false;
                    continue;
                }
            }
        }

        if ch == '\n' {
            current_text.push(ch);
            chars.next();
            at_line_start = true;
        } else {
            current_text.push(ch);
            chars.next();
            if ch != ' ' && ch != '\t' && ch != '\r' {
                at_line_start = false;
            }
        }
    }

    flush_text(&mut current_text, tokens);
}

fn flush_text(text: &mut String, tokens: &mut Vec<PromptToken>) {
    if !text.is_empty() {
        tokens.push(PromptToken::Text(std::mem::take(text)));
    }
}

fn skip_ws(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&c) = chars.peek() {
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
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

fn lex_model_line(chars: &mut std::iter::Peekable<std::str::Chars>, tokens: &mut Vec<PromptToken>) {
    // Skip whitespace
    while let Some(&c) = chars.peek() {
        if c == ' ' || c == '\t' {
            chars.next();
        } else {
            break;
        }
    }

    loop {
        // Read model name (ident, may contain hyphens and digits)
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if !name.is_empty() {
            tokens.push(PromptToken::Ident(name));
        }

        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c == ' ' || c == '\t' {
                chars.next();
            } else {
                break;
            }
        }

        // Check for pipe
        if chars.peek() == Some(&'|') {
            chars.next();
            tokens.push(PromptToken::Pipe);
            // Skip whitespace
            while let Some(&c) = chars.peek() {
                if c == ' ' || c == '\t' {
                    chars.next();
                } else {
                    break;
                }
            }
        } else {
            break;
        }
    }

    // Skip to end of line
    skip_to_newline(chars);
}

fn lex_brace_block(chars: &mut std::iter::Peekable<std::str::Chars>, tokens: &mut Vec<PromptToken>) {
    skip_ws(chars);

    if chars.peek() != Some(&'{') {
        return;
    }
    chars.next();
    tokens.push(PromptToken::BraceOpen);

    let mut depth = 1u32;
    loop {
        skip_ws(chars);
        match chars.peek() {
            None => break,
            Some(&'}') => {
                chars.next();
                depth -= 1;
                if depth == 0 {
                    tokens.push(PromptToken::BraceClose);
                    // Skip trailing whitespace/newline
                    skip_ws(chars);
                    break;
                }
                tokens.push(PromptToken::BraceClose);
            }
            Some(&'{') => {
                chars.next();
                depth += 1;
                tokens.push(PromptToken::BraceOpen);
            }
            Some(&':') => {
                chars.next();
                tokens.push(PromptToken::Colon);
            }
            Some(&'[') => {
                chars.next();
                tokens.push(PromptToken::ArrayOpen);
            }
            Some(&']') => {
                chars.next();
                tokens.push(PromptToken::ArrayClose);
            }
            Some(&'"') => {
                chars.next();
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
                tokens.push(PromptToken::StringLiteral(s));
            }
            Some(&c) if c.is_ascii_digit() || c == '-' => {
                // Check if '-' is followed by a digit (negative number)
                if c == '-' {
                    let mut lookahead = chars.clone();
                    lookahead.next(); // skip '-'
                    if !lookahead.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                        chars.next(); // skip unknown char
                        continue;
                    }
                }
                let mut num_str = String::new();
                if c == '-' {
                    num_str.push(c);
                    chars.next();
                }
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(n) = num_str.parse::<f64>() {
                    tokens.push(PromptToken::NumberLiteral(n));
                }
            }
            Some(&c) if c.is_ascii_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                // Check for booleans
                match ident.as_str() {
                    "true" => tokens.push(PromptToken::NumberLiteral(1.0)), // will be handled as bool in parser
                    "false" => tokens.push(PromptToken::NumberLiteral(0.0)),
                    _ => tokens.push(PromptToken::Ident(ident)),
                }
            }
            Some(&',') => {
                chars.next(); // skip commas
            }
            Some(_) => {
                chars.next(); // skip unknown chars
            }
        }
    }
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
    fn role_directive() {
        let parts = vec![make_text("@role system\nYou are helpful.\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::DirectiveRole("system".into()));
        assert_eq!(tokens[1], PromptToken::Text("You are helpful.\n".into()));
    }

    #[test]
    fn model_directive() {
        let parts = vec![make_text("@model claude-sonnet | gpt-4o\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::DirectiveModel);
        assert_eq!(tokens[1], PromptToken::Ident("claude-sonnet".into()));
        assert_eq!(tokens[2], PromptToken::Pipe);
        assert_eq!(tokens[3], PromptToken::Ident("gpt-4o".into()));
    }

    #[test]
    fn text_with_capture() {
        let parts = vec![
            make_text("@role system\nHello "),
            make_capture(),
            make_text("!\n"),
        ];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::DirectiveRole("system".into()));
        assert_eq!(tokens[1], PromptToken::Text("Hello ".into()));
        assert_eq!(tokens[2], PromptToken::Capture(0));
        assert_eq!(tokens[3], PromptToken::Text("!\n".into()));
    }

    #[test]
    fn at_sign_not_directive() {
        let parts = vec![make_text("email me @alice\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::Text("email me @alice\n".into()));
    }

    #[test]
    fn constraints_directive() {
        let parts = vec![make_text("@constraints {\n  temperature: 0.7\n  max_tokens: 4096\n}\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::DirectiveConstraints);
        assert_eq!(tokens[1], PromptToken::BraceOpen);
        assert_eq!(tokens[2], PromptToken::Ident("temperature".into()));
        assert_eq!(tokens[3], PromptToken::Colon);
        assert_eq!(tokens[4], PromptToken::NumberLiteral(0.7));
        assert_eq!(tokens[5], PromptToken::Ident("max_tokens".into()));
        assert_eq!(tokens[6], PromptToken::Colon);
        assert_eq!(tokens[7], PromptToken::NumberLiteral(4096.0));
        assert_eq!(tokens[8], PromptToken::BraceClose);
    }

    #[test]
    fn examples_directive() {
        let parts = vec![make_text("@examples {\n  user: \"hello\"\n  assistant: \"hi\"\n}\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::DirectiveExamples);
        assert_eq!(tokens[1], PromptToken::BraceOpen);
        assert_eq!(tokens[2], PromptToken::Ident("user".into()));
        assert_eq!(tokens[3], PromptToken::Colon);
        assert_eq!(tokens[4], PromptToken::StringLiteral("hello".into()));
    }

    #[test]
    fn messages_with_capture() {
        let parts = vec![
            make_text("@messages "),
            make_capture(),
            make_text("\n"),
        ];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::DirectiveMessages);
        assert_eq!(tokens[1], PromptToken::Capture(0));
    }

    #[test]
    fn output_with_inline_schema() {
        let parts = vec![make_text("@output {\n  answer: str\n  confidence: num\n}\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], PromptToken::DirectiveOutput);
        assert_eq!(tokens[1], PromptToken::BraceOpen);
        assert_eq!(tokens[2], PromptToken::Ident("answer".into()));
        assert_eq!(tokens[3], PromptToken::Colon);
        assert_eq!(tokens[4], PromptToken::Ident("str".into()));
    }
}
