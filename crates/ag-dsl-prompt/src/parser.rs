use crate::ast::*;
use crate::lexer::PromptToken;

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

pub fn parse(name: &str, tokens: &[PromptToken]) -> Result<PromptTemplate, Vec<Diagnostic>> {
    let mut parser = Parser::new(name, tokens);
    parser.parse_template()
}

struct Parser<'a> {
    name: String,
    tokens: &'a [PromptToken],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(name: &str, tokens: &'a [PromptToken]) -> Self {
        Self {
            name: name.to_string(),
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn peek(&self) -> &PromptToken {
        self.tokens.get(self.pos).unwrap_or(&PromptToken::Eof)
    }

    fn advance(&mut self) -> &PromptToken {
        let tok = self.tokens.get(self.pos).unwrap_or(&PromptToken::Eof);
        self.pos += 1;
        tok
    }

    fn parse_template(&mut self) -> Result<PromptTemplate, Vec<Diagnostic>> {
        let mut sections = Vec::new();
        let mut model: Option<ModelSpec> = None;
        let mut output: Option<OutputSpec> = None;
        let mut constraints: Option<Constraints> = None;

        // Collect text/captures before first directive → implicit system role
        let implicit_body = self.collect_body();
        if !implicit_body.is_empty() {
            sections.push(PromptSection::Role {
                role: RoleName::System,
                body: implicit_body,
            });
        }

        loop {
            match self.peek().clone() {
                PromptToken::Eof => break,
                PromptToken::DirectiveRole(role_name) => {
                    self.advance();
                    let role = RoleName::from_str(&role_name);
                    let body = self.collect_body();
                    sections.push(PromptSection::Role { role, body });
                }
                PromptToken::DirectiveModel => {
                    self.advance();
                    model = Some(self.parse_model());
                }
                PromptToken::DirectiveExamples => {
                    self.advance();
                    match self.parse_examples() {
                        Ok(ex) => sections.push(PromptSection::Examples(ex)),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                PromptToken::DirectiveOutput => {
                    self.advance();
                    output = Some(self.parse_output());
                }
                PromptToken::DirectiveConstraints => {
                    self.advance();
                    match self.parse_constraints() {
                        Ok(c) => constraints = Some(c),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                PromptToken::DirectiveMessages => {
                    self.advance();
                    match self.parse_messages() {
                        Ok(sec) => sections.push(sec),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                // Stray tokens — skip
                _ => {
                    self.advance();
                }
            }
        }

        if sections.is_empty() && model.is_none() && output.is_none() && constraints.is_none() {
            self.diagnostics.push(Diagnostic {
                message: "empty prompt template".to_string(),
                severity: Severity::Error,
            });
        }

        if !self.diagnostics.is_empty() {
            return Err(self.diagnostics.clone());
        }

        Ok(PromptTemplate {
            name: self.name.clone(),
            sections,
            model,
            output,
            constraints,
        })
    }

    fn collect_body(&mut self) -> Vec<PromptPart> {
        let mut body = Vec::new();
        loop {
            match self.peek() {
                PromptToken::Text(s) => {
                    let s = s.clone();
                    self.advance();
                    body.push(PromptPart::Text(s));
                }
                PromptToken::Capture(idx) => {
                    let idx = *idx;
                    self.advance();
                    body.push(PromptPart::Capture(idx));
                }
                _ => break,
            }
        }
        // Trim trailing whitespace-only text
        while let Some(PromptPart::Text(s)) = body.last() {
            if s.trim().is_empty() {
                body.pop();
            } else {
                break;
            }
        }
        body
    }

    fn parse_model(&mut self) -> ModelSpec {
        let mut models = Vec::new();
        loop {
            match self.peek() {
                PromptToken::Ident(name) => {
                    models.push(name.clone());
                    self.advance();
                }
                _ => break,
            }
            if matches!(self.peek(), PromptToken::Pipe) {
                self.advance();
            } else {
                break;
            }
        }
        ModelSpec { models }
    }

    fn parse_examples(&mut self) -> Result<Vec<Example>, String> {
        if !matches!(self.peek(), PromptToken::BraceOpen) {
            return Err("expected `{` after @examples".to_string());
        }
        self.advance(); // skip {

        let mut examples = Vec::new();
        let mut current_pairs: Vec<(RoleName, String)> = Vec::new();

        loop {
            match self.peek() {
                PromptToken::BraceClose => {
                    self.advance();
                    break;
                }
                PromptToken::Eof => break,
                PromptToken::Ident(role_name) => {
                    let role = RoleName::from_str(&role_name.clone());
                    self.advance();
                    // Expect colon
                    if matches!(self.peek(), PromptToken::Colon) {
                        self.advance();
                    }
                    // Expect string literal
                    if let PromptToken::StringLiteral(content) = self.peek() {
                        let content = content.clone();
                        self.advance();
                        current_pairs.push((role, content));
                    }
                }
                _ => {
                    self.advance(); // skip unexpected
                }
            }
        }

        if !current_pairs.is_empty() {
            examples.push(Example {
                pairs: current_pairs,
            });
        }

        Ok(examples)
    }

    fn parse_output(&mut self) -> OutputSpec {
        match self.peek() {
            PromptToken::Capture(idx) => {
                let idx = *idx;
                self.advance();
                OutputSpec {
                    kind: OutputKind::CaptureRef(idx),
                }
            }
            PromptToken::BraceOpen => {
                self.advance();
                let mut fields = Vec::new();
                loop {
                    match self.peek() {
                        PromptToken::BraceClose => {
                            self.advance();
                            break;
                        }
                        PromptToken::Eof => break,
                        PromptToken::Ident(name) => {
                            let name = name.clone();
                            self.advance();
                            if matches!(self.peek(), PromptToken::Colon) {
                                self.advance();
                            }
                            // Read type: could be Ident, or ArrayOpen Ident ArrayClose
                            let ty = self.parse_type_annotation();
                            fields.push(OutputField { name, ty });
                        }
                        _ => {
                            self.advance();
                        }
                    }
                }
                OutputSpec {
                    kind: OutputKind::Inline(fields),
                }
            }
            _ => {
                // No schema provided, empty inline
                OutputSpec {
                    kind: OutputKind::Inline(Vec::new()),
                }
            }
        }
    }

    fn parse_type_annotation(&mut self) -> String {
        match self.peek() {
            PromptToken::ArrayOpen => {
                self.advance();
                let inner = if let PromptToken::Ident(ty) = self.peek() {
                    let ty = ty.clone();
                    self.advance();
                    ty
                } else {
                    "any".to_string()
                };
                if matches!(self.peek(), PromptToken::ArrayClose) {
                    self.advance();
                }
                format!("[{}]", inner)
            }
            PromptToken::Ident(ty) => {
                let ty = ty.clone();
                self.advance();
                ty
            }
            _ => "any".to_string(),
        }
    }

    fn parse_constraints(&mut self) -> Result<Constraints, String> {
        if !matches!(self.peek(), PromptToken::BraceOpen) {
            return Err("expected `{` after @constraints".to_string());
        }
        self.advance();

        let mut fields = Vec::new();
        loop {
            match self.peek() {
                PromptToken::BraceClose => {
                    self.advance();
                    break;
                }
                PromptToken::Eof => break,
                PromptToken::Ident(key) => {
                    let key = key.clone();
                    self.advance();
                    if matches!(self.peek(), PromptToken::Colon) {
                        self.advance();
                    }
                    let value = self.parse_constraint_value();
                    fields.push((key, value));
                }
                _ => {
                    self.advance();
                }
            }
        }

        Ok(Constraints { fields })
    }

    fn parse_constraint_value(&mut self) -> ConstraintValue {
        match self.peek() {
            PromptToken::NumberLiteral(n) => {
                let n = *n;
                self.advance();
                // Check for booleans encoded as 0.0/1.0
                if n == 1.0 || n == 0.0 {
                    // Could be bool, but we keep as number since parser can't distinguish
                    ConstraintValue::Number(n)
                } else {
                    ConstraintValue::Number(n)
                }
            }
            PromptToken::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                ConstraintValue::String(s)
            }
            PromptToken::ArrayOpen => {
                self.advance();
                let mut items = Vec::new();
                loop {
                    match self.peek() {
                        PromptToken::ArrayClose => {
                            self.advance();
                            break;
                        }
                        PromptToken::Eof => break,
                        _ => {
                            items.push(self.parse_constraint_value());
                        }
                    }
                }
                ConstraintValue::Array(items)
            }
            PromptToken::Ident(s) => {
                let s = s.clone();
                self.advance();
                match s.as_str() {
                    "true" => ConstraintValue::Bool(true),
                    "false" => ConstraintValue::Bool(false),
                    _ => ConstraintValue::String(s),
                }
            }
            _ => {
                self.advance();
                ConstraintValue::String(String::new())
            }
        }
    }

    fn parse_messages(&mut self) -> Result<PromptSection, String> {
        match self.peek() {
            PromptToken::Capture(idx) => {
                let idx = *idx;
                self.advance();
                Ok(PromptSection::Messages {
                    capture_index: idx,
                })
            }
            _ => Err("expected capture expression after @messages".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::PromptToken;

    #[test]
    fn parse_simple_role() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("You are helpful.".into()),
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.sections.len(), 1);
        match &tpl.sections[0] {
            PromptSection::Role { role, body } => {
                assert_eq!(*role, RoleName::System);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("expected Role section"),
        }
    }

    #[test]
    fn parse_implicit_system() {
        let tokens = vec![
            PromptToken::Text("You are a helpful assistant.".into()),
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.sections.len(), 1);
        match &tpl.sections[0] {
            PromptSection::Role { role, .. } => {
                assert_eq!(*role, RoleName::System);
            }
            _ => panic!("expected implicit system role"),
        }
    }

    #[test]
    fn parse_multi_role() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Be helpful.".into()),
            PromptToken::DirectiveRole("user".into()),
            PromptToken::Text("Hello ".into()),
            PromptToken::Capture(0),
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.sections.len(), 2);
    }

    #[test]
    fn parse_model_spec() {
        let tokens = vec![
            PromptToken::DirectiveModel,
            PromptToken::Ident("claude-sonnet".into()),
            PromptToken::Pipe,
            PromptToken::Ident("gpt-4o".into()),
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Hello".into()),
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        let model = tpl.model.unwrap();
        assert_eq!(model.models, vec!["claude-sonnet", "gpt-4o"]);
    }

    #[test]
    fn parse_examples() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Hello".into()),
            PromptToken::DirectiveExamples,
            PromptToken::BraceOpen,
            PromptToken::Ident("user".into()),
            PromptToken::Colon,
            PromptToken::StringLiteral("hello".into()),
            PromptToken::Ident("assistant".into()),
            PromptToken::Colon,
            PromptToken::StringLiteral("hi".into()),
            PromptToken::BraceClose,
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.sections.len(), 2); // Role + Examples
    }

    #[test]
    fn parse_constraints() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Hello".into()),
            PromptToken::DirectiveConstraints,
            PromptToken::BraceOpen,
            PromptToken::Ident("temperature".into()),
            PromptToken::Colon,
            PromptToken::NumberLiteral(0.7),
            PromptToken::Ident("max_tokens".into()),
            PromptToken::Colon,
            PromptToken::NumberLiteral(4096.0),
            PromptToken::BraceClose,
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        let c = tpl.constraints.unwrap();
        assert_eq!(c.fields.len(), 2);
        assert_eq!(c.fields[0].0, "temperature");
    }

    #[test]
    fn parse_output_inline() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Answer".into()),
            PromptToken::DirectiveOutput,
            PromptToken::BraceOpen,
            PromptToken::Ident("answer".into()),
            PromptToken::Colon,
            PromptToken::Ident("str".into()),
            PromptToken::Ident("confidence".into()),
            PromptToken::Colon,
            PromptToken::Ident("num".into()),
            PromptToken::BraceClose,
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        let out = tpl.output.unwrap();
        match out.kind {
            OutputKind::Inline(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "answer");
                assert_eq!(fields[0].ty, "str");
            }
            _ => panic!("expected inline output"),
        }
    }

    #[test]
    fn parse_output_capture_ref() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Answer".into()),
            PromptToken::DirectiveOutput,
            PromptToken::Capture(0),
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        let out = tpl.output.unwrap();
        match out.kind {
            OutputKind::CaptureRef(idx) => assert_eq!(idx, 0),
            _ => panic!("expected capture ref"),
        }
    }

    #[test]
    fn parse_messages() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Hello".into()),
            PromptToken::DirectiveMessages,
            PromptToken::Capture(0),
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.sections.len(), 2);
        match &tpl.sections[1] {
            PromptSection::Messages { capture_index } => assert_eq!(*capture_index, 0),
            _ => panic!("expected Messages section"),
        }
    }

    #[test]
    fn parse_error_empty_prompt() {
        let tokens = vec![PromptToken::Eof];
        let result = parse("test", &tokens);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("empty prompt"));
    }

    #[test]
    fn parse_error_examples_no_brace() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Hello".into()),
            PromptToken::DirectiveExamples,
            PromptToken::Text("invalid".into()),
            PromptToken::Eof,
        ];
        let result = parse("test", &tokens);
        assert!(result.is_err());
    }

    #[test]
    fn parse_error_messages_no_capture() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Hello".into()),
            PromptToken::DirectiveMessages,
            PromptToken::Text("invalid".into()),
            PromptToken::Eof,
        ];
        let result = parse("test", &tokens);
        assert!(result.is_err());
    }

    #[test]
    fn parse_output_array_type() {
        let tokens = vec![
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("Answer".into()),
            PromptToken::DirectiveOutput,
            PromptToken::BraceOpen,
            PromptToken::Ident("sources".into()),
            PromptToken::Colon,
            PromptToken::ArrayOpen,
            PromptToken::Ident("str".into()),
            PromptToken::ArrayClose,
            PromptToken::BraceClose,
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        let out = tpl.output.unwrap();
        match out.kind {
            OutputKind::Inline(fields) => {
                assert_eq!(fields[0].ty, "[str]");
            }
            _ => panic!("expected inline output"),
        }
    }

    #[test]
    fn parse_full_template() {
        let tokens = vec![
            PromptToken::DirectiveModel,
            PromptToken::Ident("claude-sonnet".into()),
            PromptToken::Pipe,
            PromptToken::Ident("gpt-4o".into()),
            PromptToken::DirectiveRole("system".into()),
            PromptToken::Text("You are ".into()),
            PromptToken::Capture(0),
            PromptToken::Text(".".into()),
            PromptToken::DirectiveExamples,
            PromptToken::BraceOpen,
            PromptToken::Ident("user".into()),
            PromptToken::Colon,
            PromptToken::StringLiteral("hello".into()),
            PromptToken::Ident("assistant".into()),
            PromptToken::Colon,
            PromptToken::StringLiteral("hi".into()),
            PromptToken::BraceClose,
            PromptToken::DirectiveConstraints,
            PromptToken::BraceOpen,
            PromptToken::Ident("temperature".into()),
            PromptToken::Colon,
            PromptToken::NumberLiteral(0.7),
            PromptToken::BraceClose,
            PromptToken::Eof,
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert!(tpl.model.is_some());
        assert_eq!(tpl.model.as_ref().unwrap().models.len(), 2);
        assert_eq!(tpl.sections.len(), 2); // Role + Examples
        assert!(tpl.constraints.is_some());
    }
}
