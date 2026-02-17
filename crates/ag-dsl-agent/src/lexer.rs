use ag_dsl_core::DslPart;
use ag_dsl_prompt::lexer::PromptToken;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentToken {
    DirectiveTools,
    DirectiveSkills,
    DirectiveAgents,
    DirectiveOn(String),
    Prompt(PromptToken),
}

pub fn lex(parts: &[DslPart]) -> Vec<AgentToken> {
    let mut tokens = Vec::new();
    let mut capture_index = 0;

    for part in parts {
        match part {
            DslPart::Text(text, _) => {
                lex_text(text, &mut tokens);
            }
            DslPart::Capture(_, _) => {
                tokens.push(AgentToken::Prompt(PromptToken::Capture(capture_index)));
                capture_index += 1;
            }
        }
    }

    tokens.push(AgentToken::Prompt(PromptToken::Eof));
    tokens
}

fn lex_text(text: &str, tokens: &mut Vec<AgentToken>) {
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
                // ── Agent-specific directives ────────────────────
                "tools" => {
                    flush_text(&mut current_text, tokens);
                    chars.next(); // skip '@'
                    for _ in 0..5 {
                        chars.next();
                    } // skip 'tools'
                    tokens.push(AgentToken::DirectiveTools);
                    skip_to_newline(&mut chars);
                    at_line_start = true;
                    continue;
                }
                "skills" => {
                    flush_text(&mut current_text, tokens);
                    chars.next();
                    for _ in 0..6 {
                        chars.next();
                    }
                    tokens.push(AgentToken::DirectiveSkills);
                    skip_to_newline(&mut chars);
                    at_line_start = true;
                    continue;
                }
                "agents" => {
                    flush_text(&mut current_text, tokens);
                    chars.next();
                    for _ in 0..6 {
                        chars.next();
                    }
                    tokens.push(AgentToken::DirectiveAgents);
                    skip_to_newline(&mut chars);
                    at_line_start = true;
                    continue;
                }
                "on" => {
                    flush_text(&mut current_text, tokens);
                    chars.next(); // skip '@'
                    chars.next(); // skip 'o'
                    chars.next(); // skip 'n'
                    // Skip whitespace on same line
                    while let Some(&c) = chars.peek() {
                        if c == ' ' || c == '\t' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // Read event name
                    let mut event_name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '\n' || c == '\r' || c == ' ' || c == '\t' {
                            break;
                        }
                        event_name.push(c);
                        chars.next();
                    }
                    tokens.push(AgentToken::DirectiveOn(event_name.trim().to_string()));
                    skip_to_newline(&mut chars);
                    at_line_start = true;
                    continue;
                }
                // ── Prompt directives (delegated as Prompt(...)) ─
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
                    tokens.push(AgentToken::Prompt(PromptToken::DirectiveRole(
                        role_name.trim().to_string(),
                    )));
                    at_line_start = true;
                    continue;
                }
                "model" => {
                    flush_text(&mut current_text, tokens);
                    chars.next(); // skip '@'
                    for _ in 0..5 {
                        chars.next();
                    } // skip 'model'
                    tokens.push(AgentToken::Prompt(PromptToken::DirectiveModel));
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
                    tokens.push(AgentToken::Prompt(PromptToken::DirectiveExamples));
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
                    tokens.push(AgentToken::Prompt(PromptToken::DirectiveOutput));
                    skip_ws(&mut chars);
                    if chars.peek() == Some(&'{') {
                        lex_brace_block(&mut chars, tokens);
                    }
                    at_line_start = true;
                    continue;
                }
                "constraints" => {
                    flush_text(&mut current_text, tokens);
                    chars.next();
                    for _ in 0..11 {
                        chars.next();
                    }
                    tokens.push(AgentToken::Prompt(PromptToken::DirectiveConstraints));
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
                    tokens.push(AgentToken::Prompt(PromptToken::DirectiveMessages));
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

fn flush_text(text: &mut String, tokens: &mut Vec<AgentToken>) {
    if !text.is_empty() {
        tokens.push(AgentToken::Prompt(PromptToken::Text(std::mem::take(text))));
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

fn lex_model_line(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    tokens: &mut Vec<AgentToken>,
) {
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
            tokens.push(AgentToken::Prompt(PromptToken::Ident(name)));
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
            tokens.push(AgentToken::Prompt(PromptToken::Pipe));
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

    skip_to_newline(chars);
}

fn lex_brace_block(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    tokens: &mut Vec<AgentToken>,
) {
    skip_ws(chars);

    if chars.peek() != Some(&'{') {
        return;
    }
    chars.next();
    tokens.push(AgentToken::Prompt(PromptToken::BraceOpen));

    let mut depth = 1u32;
    loop {
        skip_ws(chars);
        match chars.peek() {
            None => break,
            Some(&'}') => {
                chars.next();
                depth -= 1;
                if depth == 0 {
                    tokens.push(AgentToken::Prompt(PromptToken::BraceClose));
                    skip_ws(chars);
                    break;
                }
                tokens.push(AgentToken::Prompt(PromptToken::BraceClose));
            }
            Some(&'{') => {
                chars.next();
                depth += 1;
                tokens.push(AgentToken::Prompt(PromptToken::BraceOpen));
            }
            Some(&':') => {
                chars.next();
                tokens.push(AgentToken::Prompt(PromptToken::Colon));
            }
            Some(&'[') => {
                chars.next();
                tokens.push(AgentToken::Prompt(PromptToken::ArrayOpen));
            }
            Some(&']') => {
                chars.next();
                tokens.push(AgentToken::Prompt(PromptToken::ArrayClose));
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
                tokens.push(AgentToken::Prompt(PromptToken::StringLiteral(s)));
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
                    tokens.push(AgentToken::Prompt(PromptToken::NumberLiteral(n)));
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
                match ident.as_str() {
                    "true" => {
                        tokens.push(AgentToken::Prompt(PromptToken::NumberLiteral(1.0)));
                    }
                    "false" => {
                        tokens.push(AgentToken::Prompt(PromptToken::NumberLiteral(0.0)));
                    }
                    _ => tokens.push(AgentToken::Prompt(PromptToken::Ident(ident))),
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
    fn tools_directive_with_capture() {
        let parts = vec![make_text("@tools "), make_capture(), make_text("\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], AgentToken::DirectiveTools);
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::Capture(0))
        );
    }

    #[test]
    fn skills_directive_with_capture() {
        let parts = vec![make_text("@skills "), make_capture(), make_text("\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], AgentToken::DirectiveSkills);
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::Capture(0))
        );
    }

    #[test]
    fn agents_directive_with_capture() {
        let parts = vec![make_text("@agents "), make_capture(), make_text("\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], AgentToken::DirectiveAgents);
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::Capture(0))
        );
    }

    #[test]
    fn on_init_directive_with_capture() {
        let parts = vec![make_text("@on init\n"), make_capture(), make_text("\n")];
        let tokens = lex(&parts);
        assert_eq!(tokens[0], AgentToken::DirectiveOn("init".into()));
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::Capture(0))
        );
    }

    #[test]
    fn role_directive_passes_through_as_prompt() {
        let parts = vec![make_text("@role system\nYou are helpful.\n")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens[0],
            AgentToken::Prompt(PromptToken::DirectiveRole("system".into()))
        );
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::Text("You are helpful.\n".into()))
        );
    }

    #[test]
    fn model_directive_passes_through_as_prompt() {
        let parts = vec![make_text("@model claude-sonnet | gpt-4o\n")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens[0],
            AgentToken::Prompt(PromptToken::DirectiveModel)
        );
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::Ident("claude-sonnet".into()))
        );
        assert_eq!(tokens[2], AgentToken::Prompt(PromptToken::Pipe));
        assert_eq!(
            tokens[3],
            AgentToken::Prompt(PromptToken::Ident("gpt-4o".into()))
        );
    }

    #[test]
    fn constraints_directive_passes_through() {
        let parts = vec![make_text(
            "@constraints {\n  temperature: 0.7\n}\n",
        )];
        let tokens = lex(&parts);
        assert_eq!(
            tokens[0],
            AgentToken::Prompt(PromptToken::DirectiveConstraints)
        );
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::BraceOpen)
        );
        assert_eq!(
            tokens[2],
            AgentToken::Prompt(PromptToken::Ident("temperature".into()))
        );
        assert_eq!(tokens[3], AgentToken::Prompt(PromptToken::Colon));
        assert_eq!(
            tokens[4],
            AgentToken::Prompt(PromptToken::NumberLiteral(0.7))
        );
        assert_eq!(
            tokens[5],
            AgentToken::Prompt(PromptToken::BraceClose)
        );
    }

    #[test]
    fn examples_directive_passes_through() {
        let parts = vec![make_text(
            "@examples {\n  user: \"hello\"\n  assistant: \"hi\"\n}\n",
        )];
        let tokens = lex(&parts);
        assert_eq!(
            tokens[0],
            AgentToken::Prompt(PromptToken::DirectiveExamples)
        );
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::BraceOpen)
        );
        assert_eq!(
            tokens[2],
            AgentToken::Prompt(PromptToken::Ident("user".into()))
        );
    }

    #[test]
    fn at_sign_not_at_line_start_is_text() {
        let parts = vec![make_text("email me @tools\n")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens[0],
            AgentToken::Prompt(PromptToken::Text("email me @tools\n".into()))
        );
    }

    #[test]
    fn unknown_at_directive_is_text() {
        let parts = vec![make_text("@foobar something\n")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens[0],
            AgentToken::Prompt(PromptToken::Text("@foobar something\n".into()))
        );
    }

    #[test]
    fn mixed_agent_and_prompt_directives() {
        let parts = vec![
            make_text("@model claude-sonnet\n@role system\nYou are an agent.\n@tools "),
            make_capture(),
            make_text("\n@on init\n"),
            make_capture(),
            make_text("\n"),
        ];
        let tokens = lex(&parts);
        assert_eq!(
            tokens[0],
            AgentToken::Prompt(PromptToken::DirectiveModel)
        );
        assert_eq!(
            tokens[1],
            AgentToken::Prompt(PromptToken::Ident("claude-sonnet".into()))
        );
        assert_eq!(
            tokens[2],
            AgentToken::Prompt(PromptToken::DirectiveRole("system".into()))
        );
        assert_eq!(
            tokens[3],
            AgentToken::Prompt(PromptToken::Text("You are an agent.\n".into()))
        );
        assert_eq!(tokens[4], AgentToken::DirectiveTools);
        assert_eq!(
            tokens[5],
            AgentToken::Prompt(PromptToken::Capture(0))
        );
        // tokens[6] is the "\n" text between capture and @on directive
        assert_eq!(
            tokens[6],
            AgentToken::Prompt(PromptToken::Text("\n".into()))
        );
        assert_eq!(tokens[7], AgentToken::DirectiveOn("init".into()));
        assert_eq!(
            tokens[8],
            AgentToken::Prompt(PromptToken::Capture(1))
        );
    }
}
