use crate::ast::*;
use crate::lexer::SkillToken;

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

impl Diagnostic {
    fn error(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            severity: Severity::Error,
        }
    }

    #[allow(dead_code)]
    fn warning(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            severity: Severity::Warning,
        }
    }
}

/// Parse a token stream into a `SkillTemplate`.
///
/// The `name` argument is the skill name (taken from the DSL block header).
pub fn parse(tokens: &[SkillToken], name: &str) -> Result<SkillTemplate, Vec<Diagnostic>> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        diagnostics: Vec::new(),
    };

    let mut description: Option<String> = None;
    let mut input_fields: Vec<SkillField> = Vec::new();
    let mut steps: Vec<SkillStep> = Vec::new();
    let mut output_fields: Vec<SkillField> = Vec::new();

    while !parser.at_eof() {
        match parser.peek() {
            SkillToken::DirectiveDescription => {
                parser.advance(); // consume directive
                match parser.peek() {
                    SkillToken::StringLiteral(s) => {
                        description = Some(s.clone());
                        parser.advance();
                    }
                    _ => {
                        parser.diagnostics.push(Diagnostic::error(
                            "expected string literal after @description",
                        ));
                        // Try to recover by skipping to next directive.
                        parser.skip_to_next_directive();
                    }
                }
            }
            SkillToken::DirectiveInput => {
                parser.advance(); // consume directive
                match parser.parse_field_block() {
                    Ok(fields) => input_fields = fields,
                    Err(diag) => {
                        parser.diagnostics.push(diag);
                        parser.skip_to_next_directive();
                    }
                }
            }
            SkillToken::DirectiveOutput => {
                parser.advance(); // consume directive
                match parser.parse_field_block() {
                    Ok(fields) => output_fields = fields,
                    Err(diag) => {
                        parser.diagnostics.push(diag);
                        parser.skip_to_next_directive();
                    }
                }
            }
            SkillToken::DirectiveSteps => {
                parser.advance(); // consume directive
                steps = parser.parse_steps();
            }
            _ => {
                // Skip unexpected tokens.
                parser.advance();
            }
        }
    }

    if parser.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(parser.diagnostics);
    }

    Ok(SkillTemplate {
        name: name.to_string(),
        description,
        input_fields,
        steps,
        output_fields,
    })
}

struct Parser<'a> {
    tokens: &'a [SkillToken],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &SkillToken {
        self.tokens.get(self.pos).unwrap_or(&SkillToken::Eof)
    }

    fn advance(&mut self) -> &SkillToken {
        let tok = self.tokens.get(self.pos).unwrap_or(&SkillToken::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek(), SkillToken::Eof)
    }

    fn is_directive(tok: &SkillToken) -> bool {
        matches!(
            tok,
            SkillToken::DirectiveDescription
                | SkillToken::DirectiveInput
                | SkillToken::DirectiveSteps
                | SkillToken::DirectiveOutput
        )
    }

    fn skip_to_next_directive(&mut self) {
        while !self.at_eof() && !Self::is_directive(self.peek()) {
            self.advance();
        }
    }

    /// Parse a `{ name: type (= default)?, ... }` field block.
    fn parse_field_block(&mut self) -> Result<Vec<SkillField>, Diagnostic> {
        // Expect BraceOpen
        if !matches!(self.peek(), SkillToken::BraceOpen) {
            return Err(Diagnostic::error("expected '{' to start field block"));
        }
        self.advance(); // consume '{'

        let mut fields = Vec::new();

        loop {
            match self.peek() {
                SkillToken::BraceClose => {
                    self.advance();
                    break;
                }
                SkillToken::Eof => {
                    return Err(Diagnostic::error("unexpected end of input, expected '}'"));
                }
                SkillToken::Ident(_) => {
                    let field = self.parse_field()?;
                    fields.push(field);
                }
                _ => {
                    // Skip unexpected tokens inside field block.
                    self.advance();
                }
            }
        }

        Ok(fields)
    }

    /// Parse a single field: `name: type (= default)?`
    fn parse_field(&mut self) -> Result<SkillField, Diagnostic> {
        // Name
        let name = match self.advance().clone() {
            SkillToken::Ident(n) => n,
            _ => return Err(Diagnostic::error("expected field name identifier")),
        };

        // Colon
        if !matches!(self.peek(), SkillToken::Colon) {
            return Err(Diagnostic::error(format!(
                "expected ':' after field name '{}'",
                name
            )));
        }
        self.advance();

        // Type: either `ident` or `[ident]`
        let type_name = if matches!(self.peek(), SkillToken::ArrayOpen) {
            self.advance(); // consume '['
            let inner = match self.advance().clone() {
                SkillToken::Ident(t) => t,
                _ => return Err(Diagnostic::error("expected type name inside '[]'")),
            };
            if !matches!(self.peek(), SkillToken::ArrayClose) {
                return Err(Diagnostic::error("expected ']' after array type"));
            }
            self.advance(); // consume ']'
            format!("[{}]", inner)
        } else {
            match self.advance().clone() {
                SkillToken::Ident(t) => t,
                _ => {
                    return Err(Diagnostic::error(format!(
                        "expected type name for field '{}'",
                        name
                    )))
                }
            }
        };

        // Optional default: `= value`
        let default = if matches!(self.peek(), SkillToken::Equals) {
            self.advance(); // consume '='
            match self.advance().clone() {
                SkillToken::StringLiteral(s) => Some(format!("\"{}\"", s)),
                SkillToken::NumberLiteral(n) => {
                    if n == n.floor() && n.abs() < 1e15 {
                        Some(format!("{}", n as i64))
                    } else {
                        Some(format!("{}", n))
                    }
                }
                SkillToken::Ident(v) => Some(v),
                _ => return Err(Diagnostic::error("expected default value after '='")),
            }
        } else {
            None
        };

        Ok(SkillField {
            name,
            type_name,
            default,
        })
    }

    /// Parse steps: collect Text and Capture tokens, then split into numbered steps.
    fn parse_steps(&mut self) -> Vec<SkillStep> {
        // Collect all Text and Capture tokens until the next directive or Eof.
        let mut fragments: Vec<StepFragment> = Vec::new();

        while !self.at_eof() && !Self::is_directive(self.peek()) {
            match self.peek() {
                SkillToken::Text(s) => {
                    fragments.push(StepFragment::Text(s.clone()));
                    self.advance();
                }
                SkillToken::Capture(idx) => {
                    fragments.push(StepFragment::Capture(*idx));
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        // Now build a combined string with capture placeholders so we can split on
        // numbered lines. We track capture positions by character offset.
        let mut combined = String::new();
        let mut capture_positions: Vec<(usize, usize)> = Vec::new(); // (char_offset, capture_index)

        for frag in &fragments {
            match frag {
                StepFragment::Text(s) => {
                    combined.push_str(s);
                }
                StepFragment::Capture(idx) => {
                    capture_positions.push((combined.len(), *idx));
                    // Insert a placeholder that won't appear in normal text.
                    combined.push('\x00');
                }
            }
        }

        // Split on numbered line prefixes. We look for patterns like `N. ` at line start.
        // The first step may or may not start with `1. ` at the very beginning.
        let mut steps = Vec::new();
        let mut remaining = combined.as_str();
        let mut offset = 0usize; // track our position in the combined string

        // Trim leading whitespace/newlines.
        let trimmed = remaining.trim_start_matches(|c: char| c == '\n' || c == '\r' || c == ' ');
        let trim_count = remaining.len() - trimmed.len();
        offset += trim_count;
        remaining = trimmed;

        // Split on `\nN. ` boundaries, or handle leading `N. `.
        loop {
            // Trim leading whitespace/newlines between steps.
            let trimmed = remaining.trim_start_matches(|c: char| c == '\n' || c == '\r' || c == ' ');
            let trim_count = remaining.len() - trimmed.len();
            offset += trim_count;
            remaining = trimmed;

            if remaining.is_empty() {
                break;
            }

            // Try to parse a step number at the current position.
            let (step_num, text_start) = if let Some((num, skip)) = parse_step_number(remaining) {
                (num, skip)
            } else {
                // No step number — treat the rest as a single unnumbered step.
                let captures = captures_in_range(&capture_positions, offset, offset + remaining.len());
                let text = remaining.replace('\x00', "").trim().to_string();
                if !text.is_empty() {
                    steps.push(SkillStep {
                        number: (steps.len() as u32) + 1,
                        text,
                        captures,
                    });
                }
                break;
            };

            // Find the next `\nN. ` boundary.
            let body = &remaining[text_start..];
            let next_boundary = find_next_step_boundary(body);

            let step_body = match next_boundary {
                Some(rel_pos) => {
                    let s = &body[..rel_pos];
                    let total_consumed = text_start + rel_pos;
                    let captures =
                        captures_in_range(&capture_positions, offset, offset + total_consumed);
                    offset += total_consumed;
                    remaining = &remaining[total_consumed..];
                    let text = s.replace('\x00', "").trim().to_string();
                    (text, captures)
                }
                None => {
                    let captures =
                        captures_in_range(&capture_positions, offset, offset + remaining.len());
                    let text = body.replace('\x00', "").trim().to_string();
                    remaining = "";
                    (text, captures)
                }
            };

            if !step_body.0.is_empty() {
                steps.push(SkillStep {
                    number: step_num,
                    text: step_body.0,
                    captures: step_body.1,
                });
            }
        }

        steps
    }
}

#[derive(Debug)]
enum StepFragment {
    Text(String),
    Capture(usize),
}

/// Try to parse a step number at the start of `s` (e.g. `1. `, `12. `).
/// Returns (number, bytes_to_skip_past_the_dot_and_space).
fn parse_step_number(s: &str) -> Option<(u32, usize)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Parse digits.
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    // Expect '.'
    if i >= bytes.len() || bytes[i] != b'.' {
        return None;
    }
    let num: u32 = s[..i].parse().ok()?;
    i += 1; // skip '.'
    // Skip optional space after dot.
    if i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    Some((num, i))
}

/// Find the next `\nN. ` boundary in the string (where N is one or more digits).
/// Returns the byte offset of the `\n` character.
fn find_next_step_boundary(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            // Check if what follows is `N. `.
            let after = i + 1;
            let mut j = after;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > after && j < bytes.len() && bytes[j] == b'.' {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Collect capture indices whose positions fall within [start, end).
fn captures_in_range(positions: &[(usize, usize)], start: usize, end: usize) -> Vec<usize> {
    positions
        .iter()
        .filter(|(pos, _)| *pos >= start && *pos < end)
        .map(|(_, idx)| *idx)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::SkillToken;

    #[test]
    fn test_full_skill_template() {
        let tokens = vec![
            SkillToken::DirectiveDescription,
            SkillToken::StringLiteral("A search skill".to_string()),
            SkillToken::DirectiveInput,
            SkillToken::BraceOpen,
            SkillToken::Ident("query".to_string()),
            SkillToken::Colon,
            SkillToken::Ident("str".to_string()),
            SkillToken::BraceClose,
            SkillToken::DirectiveSteps,
            SkillToken::Text("1. Search the web\n2. Return results".to_string()),
            SkillToken::DirectiveOutput,
            SkillToken::BraceOpen,
            SkillToken::Ident("result".to_string()),
            SkillToken::Colon,
            SkillToken::Ident("str".to_string()),
            SkillToken::BraceClose,
            SkillToken::Eof,
        ];

        let template = parse(&tokens, "search").unwrap();
        assert_eq!(template.name, "search");
        assert_eq!(template.description, Some("A search skill".to_string()));
        assert_eq!(template.input_fields.len(), 1);
        assert_eq!(template.input_fields[0].name, "query");
        assert_eq!(template.input_fields[0].type_name, "str");
        assert_eq!(template.steps.len(), 2);
        assert_eq!(template.steps[0].number, 1);
        assert_eq!(template.steps[0].text, "Search the web");
        assert_eq!(template.steps[1].number, 2);
        assert_eq!(template.steps[1].text, "Return results");
        assert_eq!(template.output_fields.len(), 1);
        assert_eq!(template.output_fields[0].name, "result");
    }

    #[test]
    fn test_fields_with_defaults() {
        let tokens = vec![
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
            SkillToken::Ident("mode".to_string()),
            SkillToken::Colon,
            SkillToken::Ident("str".to_string()),
            SkillToken::Equals,
            SkillToken::StringLiteral("fast".to_string()),
            SkillToken::BraceClose,
            SkillToken::Eof,
        ];

        let template = parse(&tokens, "test").unwrap();
        assert_eq!(template.input_fields.len(), 3);
        assert_eq!(template.input_fields[0].name, "query");
        assert!(template.input_fields[0].default.is_none());
        assert_eq!(template.input_fields[1].name, "max");
        assert_eq!(template.input_fields[1].default, Some("10".to_string()));
        assert_eq!(template.input_fields[2].name, "mode");
        assert_eq!(
            template.input_fields[2].default,
            Some("\"fast\"".to_string())
        );
    }

    #[test]
    fn test_array_type_fields() {
        let tokens = vec![
            SkillToken::DirectiveInput,
            SkillToken::BraceOpen,
            SkillToken::Ident("tags".to_string()),
            SkillToken::Colon,
            SkillToken::ArrayOpen,
            SkillToken::Ident("str".to_string()),
            SkillToken::ArrayClose,
            SkillToken::BraceClose,
            SkillToken::Eof,
        ];

        let template = parse(&tokens, "test").unwrap();
        assert_eq!(template.input_fields.len(), 1);
        assert_eq!(template.input_fields[0].name, "tags");
        assert_eq!(template.input_fields[0].type_name, "[str]");
    }

    #[test]
    fn test_steps_with_captures() {
        let tokens = vec![
            SkillToken::DirectiveSteps,
            SkillToken::Text("1. Search for ".to_string()),
            SkillToken::Capture(0),
            SkillToken::Text("\n2. Summarize results".to_string()),
            SkillToken::Eof,
        ];

        let template = parse(&tokens, "test").unwrap();
        assert_eq!(template.steps.len(), 2);
        assert_eq!(template.steps[0].number, 1);
        assert_eq!(template.steps[0].text, "Search for");
        assert_eq!(template.steps[0].captures, vec![0]);
        assert_eq!(template.steps[1].number, 2);
        assert_eq!(template.steps[1].text, "Summarize results");
        assert!(template.steps[1].captures.is_empty());
    }

    #[test]
    fn test_error_missing_brace() {
        let tokens = vec![
            SkillToken::DirectiveInput,
            // Missing BraceOpen
            SkillToken::Ident("query".to_string()),
            SkillToken::Colon,
            SkillToken::Ident("str".to_string()),
            SkillToken::BraceClose,
            SkillToken::Eof,
        ];

        let result = parse(&tokens, "test");
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("'{'")
            && d.severity == Severity::Error));
    }

    #[test]
    fn test_error_missing_string_after_description() {
        let tokens = vec![
            SkillToken::DirectiveDescription,
            // Missing StringLiteral
            SkillToken::DirectiveInput,
            SkillToken::BraceOpen,
            SkillToken::BraceClose,
            SkillToken::Eof,
        ];

        let result = parse(&tokens, "test");
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert!(diags
            .iter()
            .any(|d| d.message.contains("string literal") && d.severity == Severity::Error));
    }

    #[test]
    fn test_empty_template() {
        let tokens = vec![SkillToken::Eof];
        let template = parse(&tokens, "empty").unwrap();
        assert_eq!(template.name, "empty");
        assert!(template.description.is_none());
        assert!(template.input_fields.is_empty());
        assert!(template.steps.is_empty());
        assert!(template.output_fields.is_empty());
    }

    #[test]
    fn test_description_only() {
        let tokens = vec![
            SkillToken::DirectiveDescription,
            SkillToken::StringLiteral("Just a description".to_string()),
            SkillToken::Eof,
        ];
        let template = parse(&tokens, "desc_only").unwrap();
        assert_eq!(
            template.description,
            Some("Just a description".to_string())
        );
    }

    #[test]
    fn test_ident_default_value() {
        let tokens = vec![
            SkillToken::DirectiveInput,
            SkillToken::BraceOpen,
            SkillToken::Ident("enabled".to_string()),
            SkillToken::Colon,
            SkillToken::Ident("bool".to_string()),
            SkillToken::Equals,
            SkillToken::Ident("true".to_string()),
            SkillToken::BraceClose,
            SkillToken::Eof,
        ];

        let template = parse(&tokens, "test").unwrap();
        assert_eq!(template.input_fields[0].default, Some("true".to_string()));
    }
}
