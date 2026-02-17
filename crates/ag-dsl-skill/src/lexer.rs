use ag_dsl_core::DslPart;

#[derive(Debug, Clone, PartialEq)]
pub enum SkillToken {
    DirectiveDescription,
    DirectiveInput,
    DirectiveSteps,
    DirectiveOutput,
    Text(String),
    Capture(usize),
    BraceOpen,
    BraceClose,
    Colon,
    Equals,
    StringLiteral(String),
    NumberLiteral(f64),
    ArrayOpen,
    ArrayClose,
    Ident(String),
    Eof,
}

/// Lex a sequence of `DslPart`s into `SkillToken`s.
pub fn lex(parts: &[DslPart]) -> Vec<SkillToken> {
    let mut tokens = Vec::new();
    let mut capture_index: usize = 0;

    for part in parts {
        match part {
            DslPart::Text(text, _span) => {
                lex_text(text, &mut tokens);
            }
            DslPart::Capture(_, _span) => {
                tokens.push(SkillToken::Capture(capture_index));
                capture_index += 1;
            }
        }
    }

    tokens.push(SkillToken::Eof);
    tokens
}

/// Internal state machine for the text lexer.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    /// Scanning for directives at line starts, or plain text.
    TopLevel,
    /// Inside a `{...}` brace block (for @input / @output).
    BraceBlock,
    /// After @steps: collecting text until the next line-start directive.
    Steps,
}

fn lex_text(text: &str, tokens: &mut Vec<SkillToken>) {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut pos = 0;
    let mut mode = Mode::TopLevel;

    // Determine if `pos` is at the start of a line (pos==0 or chars[pos-1]=='\n').
    let at_line_start = |pos: usize, chars: &[char]| -> bool {
        pos == 0 || (pos > 0 && chars[pos - 1] == '\n')
    };

    while pos < len {
        match mode {
            Mode::TopLevel => {
                // Check for directive at line start.
                if at_line_start(pos, &chars) && pos < len && chars[pos] == '@' {
                    if let Some((directive, advance)) = try_directive(&chars, pos) {
                        pos += advance;
                        match directive {
                            Directive::Description => {
                                tokens.push(SkillToken::DirectiveDescription);
                                // Skip whitespace, then expect a quoted string.
                                pos = skip_whitespace(&chars, pos);
                                if pos < len && chars[pos] == '"' {
                                    let (s, adv) = lex_string_literal(&chars, pos);
                                    tokens.push(SkillToken::StringLiteral(s));
                                    pos += adv;
                                }
                            }
                            Directive::Input => {
                                tokens.push(SkillToken::DirectiveInput);
                                mode = Mode::BraceBlock;
                            }
                            Directive::Output => {
                                tokens.push(SkillToken::DirectiveOutput);
                                mode = Mode::BraceBlock;
                            }
                            Directive::Steps => {
                                tokens.push(SkillToken::DirectiveSteps);
                                // Skip trailing whitespace and newline after @steps
                                while pos < len && (chars[pos] == ' ' || chars[pos] == '\t') {
                                    pos += 1;
                                }
                                if pos < len && chars[pos] == '\n' {
                                    pos += 1;
                                }
                                mode = Mode::Steps;
                            }
                        }
                        continue;
                    }
                }
                // Not a directive — skip to next line.
                let start = pos;
                while pos < len && chars[pos] != '\n' {
                    pos += 1;
                }
                if pos < len {
                    pos += 1; // consume '\n'
                }
                let frag: String = chars[start..pos].iter().collect();
                if !frag.is_empty() {
                    tokens.push(SkillToken::Text(frag));
                }
            }

            Mode::BraceBlock => {
                pos = skip_whitespace_and_newlines(&chars, pos);
                if pos >= len {
                    break;
                }
                let ch = chars[pos];
                match ch {
                    '{' => {
                        tokens.push(SkillToken::BraceOpen);
                        pos += 1;
                    }
                    '}' => {
                        tokens.push(SkillToken::BraceClose);
                        pos += 1;
                        mode = Mode::TopLevel;
                    }
                    ':' => {
                        tokens.push(SkillToken::Colon);
                        pos += 1;
                    }
                    '=' => {
                        tokens.push(SkillToken::Equals);
                        pos += 1;
                    }
                    '[' => {
                        tokens.push(SkillToken::ArrayOpen);
                        pos += 1;
                    }
                    ']' => {
                        tokens.push(SkillToken::ArrayClose);
                        pos += 1;
                    }
                    ',' => {
                        // Skip commas.
                        pos += 1;
                    }
                    '"' => {
                        let (s, adv) = lex_string_literal(&chars, pos);
                        tokens.push(SkillToken::StringLiteral(s));
                        pos += adv;
                    }
                    _ if ch.is_ascii_digit() || ch == '-' || ch == '.' => {
                        let (n, adv) = lex_number(&chars, pos);
                        tokens.push(SkillToken::NumberLiteral(n));
                        pos += adv;
                    }
                    _ if is_ident_start(ch) => {
                        let (ident, adv) = lex_ident(&chars, pos);
                        tokens.push(SkillToken::Ident(ident));
                        pos += adv;
                    }
                    _ => {
                        // Skip unexpected characters inside brace block.
                        pos += 1;
                    }
                }
            }

            Mode::Steps => {
                // Collect text until the next line-start directive.
                // We look for `\n@` followed by a known directive keyword.
                let start = pos;
                loop {
                    if pos >= len {
                        break;
                    }
                    if at_line_start(pos, &chars) && pos < len && chars[pos] == '@' {
                        if try_directive(&chars, pos).is_some() {
                            break;
                        }
                    }
                    pos += 1;
                }
                let frag: String = chars[start..pos].iter().collect();
                if !frag.is_empty() {
                    tokens.push(SkillToken::Text(frag));
                }
                mode = Mode::TopLevel;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Directive {
    Description,
    Input,
    Steps,
    Output,
}

const DIRECTIVES: &[(&str, Directive)] = &[
    ("description", Directive::Description),
    ("input", Directive::Input),
    ("steps", Directive::Steps),
    ("output", Directive::Output),
];

/// Try to match a directive keyword starting at `pos` (which points at `@`).
/// Returns the directive and the number of characters consumed (including `@`).
fn try_directive(chars: &[char], pos: usize) -> Option<(Directive, usize)> {
    debug_assert!(chars[pos] == '@');
    let rest_start = pos + 1; // skip '@'
    for &(kw, directive) in DIRECTIVES {
        let kw_chars: Vec<char> = kw.chars().collect();
        let kw_len = kw_chars.len();
        if rest_start + kw_len <= chars.len() {
            let matches = chars[rest_start..rest_start + kw_len]
                .iter()
                .zip(kw_chars.iter())
                .all(|(a, b)| a == b);
            if matches {
                // After the keyword there must be a non-ident char or end.
                let after = rest_start + kw_len;
                if after >= chars.len() || !is_ident_continue(chars[after]) {
                    return Some((directive, 1 + kw_len)); // '@' + keyword
                }
            }
        }
    }
    None
}

fn skip_whitespace(chars: &[char], mut pos: usize) -> usize {
    while pos < chars.len() && (chars[pos] == ' ' || chars[pos] == '\t') {
        pos += 1;
    }
    pos
}

fn skip_whitespace_and_newlines(chars: &[char], mut pos: usize) -> usize {
    while pos < chars.len()
        && (chars[pos] == ' ' || chars[pos] == '\t' || chars[pos] == '\n' || chars[pos] == '\r')
    {
        pos += 1;
    }
    pos
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Lex a quoted string literal starting at `pos` (which points at `"`).
/// Returns (string_content, characters_consumed).
fn lex_string_literal(chars: &[char], pos: usize) -> (String, usize) {
    debug_assert!(chars[pos] == '"');
    let mut result = String::new();
    let mut i = pos + 1; // skip opening quote
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                other => {
                    result.push('\\');
                    result.push(other);
                }
            }
            i += 2;
        } else if chars[i] == '"' {
            i += 1; // consume closing quote
            return (result, i - pos);
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    // Unterminated string — return what we have.
    (result, i - pos)
}

/// Lex a number literal (integer or float) starting at `pos`.
/// Returns (value, characters_consumed).
fn lex_number(chars: &[char], pos: usize) -> (f64, usize) {
    let mut i = pos;
    if i < chars.len() && chars[i] == '-' {
        i += 1;
    }
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    let s: String = chars[pos..i].iter().collect();
    let val = s.parse::<f64>().unwrap_or(0.0);
    (val, i - pos)
}

/// Lex an identifier starting at `pos`.
/// Returns (ident_string, characters_consumed).
fn lex_ident(chars: &[char], pos: usize) -> (String, usize) {
    let mut i = pos;
    while i < chars.len() && is_ident_continue(chars[i]) {
        i += 1;
    }
    let s: String = chars[pos..i].iter().collect();
    (s, i - pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_core::{DslPart, Span};

    fn text(s: &str) -> DslPart {
        DslPart::Text(s.to_string(), Span::dummy())
    }

    fn capture() -> DslPart {
        DslPart::Capture(Box::new(0u32), Span::dummy())
    }

    #[test]
    fn test_description_directive() {
        let parts = vec![text("@description \"some desc\"")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveDescription,
                SkillToken::StringLiteral("some desc".to_string()),
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_input_directive() {
        let parts = vec![text("@input { query: str, max: int = 10 }")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveInput,
                SkillToken::BraceOpen,
                SkillToken::Ident("query".to_string()),
                SkillToken::Colon,
                SkillToken::Ident("str".to_string()),
                SkillToken::Ident("max".to_string()),
                SkillToken::Colon,
                SkillToken::Ident("int".to_string()),
                SkillToken::Equals,
                SkillToken::NumberLiteral(10.0),
                SkillToken::BraceClose,
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_steps_directive() {
        let parts = vec![text("@steps\n1. Do thing\n2. Do other")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveSteps,
                SkillToken::Text("1. Do thing\n2. Do other".to_string()),
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_output_directive() {
        let parts = vec![text("@output { result: str }")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveOutput,
                SkillToken::BraceOpen,
                SkillToken::Ident("result".to_string()),
                SkillToken::Colon,
                SkillToken::Ident("str".to_string()),
                SkillToken::BraceClose,
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_capture_passthrough() {
        let parts = vec![text("@steps\n1. Search for "), capture(), text("\n2. Done")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveSteps,
                SkillToken::Text("1. Search for ".to_string()),
                SkillToken::Capture(0),
                SkillToken::Text("\n".to_string()),
                SkillToken::Text("2. Done".to_string()),
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_unknown_at_treated_as_text() {
        let parts = vec![text("@foo bar baz")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::Text("@foo bar baz".to_string()),
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_description_with_escape() {
        let parts = vec![text("@description \"hello \\\"world\\\"\"")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveDescription,
                SkillToken::StringLiteral("hello \"world\"".to_string()),
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_steps_until_next_directive() {
        let parts = vec![text("@steps\n1. First step\n2. Second step\n@output { result: str }")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveSteps,
                SkillToken::Text("1. First step\n2. Second step\n".to_string()),
                SkillToken::DirectiveOutput,
                SkillToken::BraceOpen,
                SkillToken::Ident("result".to_string()),
                SkillToken::Colon,
                SkillToken::Ident("str".to_string()),
                SkillToken::BraceClose,
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_array_type_in_input() {
        let parts = vec![text("@input { tags: [str] }")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveInput,
                SkillToken::BraceOpen,
                SkillToken::Ident("tags".to_string()),
                SkillToken::Colon,
                SkillToken::ArrayOpen,
                SkillToken::Ident("str".to_string()),
                SkillToken::ArrayClose,
                SkillToken::BraceClose,
                SkillToken::Eof,
            ]
        );
    }

    #[test]
    fn test_string_default_in_input() {
        let parts = vec![text("@input { mode: str = \"fast\" }")];
        let tokens = lex(&parts);
        assert_eq!(
            tokens,
            vec![
                SkillToken::DirectiveInput,
                SkillToken::BraceOpen,
                SkillToken::Ident("mode".to_string()),
                SkillToken::Colon,
                SkillToken::Ident("str".to_string()),
                SkillToken::Equals,
                SkillToken::StringLiteral("fast".to_string()),
                SkillToken::BraceClose,
                SkillToken::Eof,
            ]
        );
    }
}
