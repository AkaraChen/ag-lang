use crate::ast::*;
use crate::lexer::AgentToken;
use ag_dsl_prompt::ast::*;
use ag_dsl_prompt::lexer::PromptToken;

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

pub fn parse(name: &str, tokens: &[AgentToken]) -> Result<AgentTemplate, Vec<Diagnostic>> {
    let mut parser = Parser::new(name, tokens);
    parser.parse_template()
}

struct Parser<'a> {
    name: String,
    tokens: &'a [AgentToken],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    seen_tools: bool,
    seen_skills: bool,
    seen_agents: bool,
}

impl<'a> Parser<'a> {
    fn new(name: &str, tokens: &'a [AgentToken]) -> Self {
        Self {
            name: name.to_string(),
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            seen_tools: false,
            seen_skills: false,
            seen_agents: false,
        }
    }

    fn peek(&self) -> &AgentToken {
        static EOF: AgentToken = AgentToken::Prompt(PromptToken::Eof);
        self.tokens.get(self.pos).unwrap_or(&EOF)
    }

    fn advance(&mut self) -> AgentToken {
        let tok = self
            .tokens
            .get(self.pos)
            .cloned()
            .unwrap_or(AgentToken::Prompt(PromptToken::Eof));
        self.pos += 1;
        tok
    }

    fn peek_prompt(&self) -> Option<&PromptToken> {
        match self.peek() {
            AgentToken::Prompt(pt) => Some(pt),
            _ => None,
        }
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek(), AgentToken::Prompt(PromptToken::Eof))
    }

    fn parse_template(&mut self) -> Result<AgentTemplate, Vec<Diagnostic>> {
        let mut sections = Vec::new();
        let mut model: Option<ModelSpec> = None;
        let mut output: Option<OutputSpec> = None;
        let mut constraints: Option<Constraints> = None;
        let mut tools_capture: Option<usize> = None;
        let mut skills_capture: Option<usize> = None;
        let mut agents_capture: Option<usize> = None;
        let mut on_hooks: Vec<OnHook> = Vec::new();

        // Collect text/captures before first directive -> implicit system role
        let implicit_body = self.collect_body();
        if !implicit_body.is_empty() {
            sections.push(PromptSection::Role {
                role: RoleName::System,
                body: implicit_body,
            });
        }

        loop {
            if self.is_eof() {
                break;
            }

            match self.peek().clone() {
                AgentToken::Prompt(PromptToken::Eof) => break,

                // ── Agent-specific directives ────────────────
                AgentToken::DirectiveTools => {
                    self.advance();
                    if self.seen_tools {
                        self.diagnostics.push(Diagnostic {
                            message: "duplicate @tools directive".to_string(),
                            severity: Severity::Error,
                        });
                    }
                    self.seen_tools = true;
                    match self.expect_capture("@tools") {
                        Ok(idx) => tools_capture = Some(idx),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                AgentToken::DirectiveSkills => {
                    self.advance();
                    if self.seen_skills {
                        self.diagnostics.push(Diagnostic {
                            message: "duplicate @skills directive".to_string(),
                            severity: Severity::Error,
                        });
                    }
                    self.seen_skills = true;
                    match self.expect_capture("@skills") {
                        Ok(idx) => skills_capture = Some(idx),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                AgentToken::DirectiveAgents => {
                    self.advance();
                    if self.seen_agents {
                        self.diagnostics.push(Diagnostic {
                            message: "duplicate @agents directive".to_string(),
                            severity: Severity::Error,
                        });
                    }
                    self.seen_agents = true;
                    match self.expect_capture("@agents") {
                        Ok(idx) => agents_capture = Some(idx),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                AgentToken::DirectiveOn(event) => {
                    self.advance();
                    match self.expect_capture(&format!("@on {}", event)) {
                        Ok(idx) => on_hooks.push(OnHook {
                            event,
                            capture_index: idx,
                        }),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }

                // ── Prompt directives ────────────────────────
                AgentToken::Prompt(PromptToken::DirectiveRole(role_name)) => {
                    self.advance();
                    let role = RoleName::from_str(&role_name);
                    let body = self.collect_body();
                    sections.push(PromptSection::Role { role, body });
                }
                AgentToken::Prompt(PromptToken::DirectiveModel) => {
                    self.advance();
                    model = Some(self.parse_model());
                }
                AgentToken::Prompt(PromptToken::DirectiveExamples) => {
                    self.advance();
                    match self.parse_examples() {
                        Ok(ex) => sections.push(PromptSection::Examples(ex)),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                AgentToken::Prompt(PromptToken::DirectiveOutput) => {
                    self.advance();
                    output = Some(self.parse_output());
                }
                AgentToken::Prompt(PromptToken::DirectiveConstraints) => {
                    self.advance();
                    match self.parse_constraints() {
                        Ok(c) => constraints = Some(c),
                        Err(msg) => self.diagnostics.push(Diagnostic {
                            message: msg,
                            severity: Severity::Error,
                        }),
                    }
                }
                AgentToken::Prompt(PromptToken::DirectiveMessages) => {
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

        let has_agent_features = tools_capture.is_some()
            || skills_capture.is_some()
            || agents_capture.is_some()
            || !on_hooks.is_empty();

        if sections.is_empty()
            && model.is_none()
            && output.is_none()
            && constraints.is_none()
            && !has_agent_features
        {
            self.diagnostics.push(Diagnostic {
                message: "empty agent template".to_string(),
                severity: Severity::Error,
            });
        }

        if !self.diagnostics.is_empty() {
            return Err(self.diagnostics.clone());
        }

        Ok(AgentTemplate {
            name: self.name.clone(),
            sections,
            model,
            output,
            constraints,
            tools_capture,
            skills_capture,
            agents_capture,
            on_hooks,
        })
    }

    fn expect_capture(&mut self, directive_name: &str) -> Result<usize, String> {
        match self.peek_prompt() {
            Some(PromptToken::Capture(idx)) => {
                let idx = *idx;
                self.advance();
                Ok(idx)
            }
            _ => Err(format!(
                "expected capture expression after {}",
                directive_name
            )),
        }
    }

    fn collect_body(&mut self) -> Vec<PromptPart> {
        let mut body = Vec::new();
        loop {
            match self.peek_prompt() {
                Some(PromptToken::Text(s)) => {
                    let s = s.clone();
                    self.advance();
                    body.push(PromptPart::Text(s));
                }
                Some(PromptToken::Capture(idx)) => {
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
            match self.peek_prompt() {
                Some(PromptToken::Ident(name)) => {
                    models.push(name.clone());
                    self.advance();
                }
                _ => break,
            }
            if matches!(self.peek_prompt(), Some(PromptToken::Pipe)) {
                self.advance();
            } else {
                break;
            }
        }
        ModelSpec { models }
    }

    fn parse_examples(&mut self) -> Result<Vec<Example>, String> {
        if !matches!(self.peek_prompt(), Some(PromptToken::BraceOpen)) {
            return Err("expected `{` after @examples".to_string());
        }
        self.advance(); // skip {

        let mut examples = Vec::new();
        let mut current_pairs: Vec<(RoleName, String)> = Vec::new();

        loop {
            match self.peek_prompt() {
                Some(PromptToken::BraceClose) => {
                    self.advance();
                    break;
                }
                Some(PromptToken::Eof) | None => break,
                Some(PromptToken::Ident(role_name)) => {
                    let role = RoleName::from_str(&role_name.clone());
                    self.advance();
                    if matches!(self.peek_prompt(), Some(PromptToken::Colon)) {
                        self.advance();
                    }
                    if let Some(PromptToken::StringLiteral(content)) = self.peek_prompt() {
                        let content = content.clone();
                        self.advance();
                        current_pairs.push((role, content));
                    }
                }
                _ => {
                    self.advance();
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
        match self.peek_prompt() {
            Some(PromptToken::Capture(idx)) => {
                let idx = *idx;
                self.advance();
                OutputSpec {
                    kind: OutputKind::CaptureRef(idx),
                }
            }
            Some(PromptToken::BraceOpen) => {
                self.advance();
                let mut fields = Vec::new();
                loop {
                    match self.peek_prompt() {
                        Some(PromptToken::BraceClose) => {
                            self.advance();
                            break;
                        }
                        Some(PromptToken::Eof) | None => break,
                        Some(PromptToken::Ident(name)) => {
                            let name = name.clone();
                            self.advance();
                            if matches!(self.peek_prompt(), Some(PromptToken::Colon)) {
                                self.advance();
                            }
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
            _ => OutputSpec {
                kind: OutputKind::Inline(Vec::new()),
            },
        }
    }

    fn parse_type_annotation(&mut self) -> String {
        match self.peek_prompt() {
            Some(PromptToken::ArrayOpen) => {
                self.advance();
                let inner = if let Some(PromptToken::Ident(ty)) = self.peek_prompt() {
                    let ty = ty.clone();
                    self.advance();
                    ty
                } else {
                    "any".to_string()
                };
                if matches!(self.peek_prompt(), Some(PromptToken::ArrayClose)) {
                    self.advance();
                }
                format!("[{}]", inner)
            }
            Some(PromptToken::Ident(ty)) => {
                let ty = ty.clone();
                self.advance();
                ty
            }
            _ => "any".to_string(),
        }
    }

    fn parse_constraints(&mut self) -> Result<Constraints, String> {
        if !matches!(self.peek_prompt(), Some(PromptToken::BraceOpen)) {
            return Err("expected `{` after @constraints".to_string());
        }
        self.advance();

        let mut fields = Vec::new();
        loop {
            match self.peek_prompt() {
                Some(PromptToken::BraceClose) => {
                    self.advance();
                    break;
                }
                Some(PromptToken::Eof) | None => break,
                Some(PromptToken::Ident(key)) => {
                    let key = key.clone();
                    self.advance();
                    if matches!(self.peek_prompt(), Some(PromptToken::Colon)) {
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
        match self.peek_prompt() {
            Some(PromptToken::NumberLiteral(n)) => {
                let n = *n;
                self.advance();
                ConstraintValue::Number(n)
            }
            Some(PromptToken::StringLiteral(s)) => {
                let s = s.clone();
                self.advance();
                ConstraintValue::String(s)
            }
            Some(PromptToken::ArrayOpen) => {
                self.advance();
                let mut items = Vec::new();
                loop {
                    match self.peek_prompt() {
                        Some(PromptToken::ArrayClose) => {
                            self.advance();
                            break;
                        }
                        Some(PromptToken::Eof) | None => break,
                        _ => {
                            items.push(self.parse_constraint_value());
                        }
                    }
                }
                ConstraintValue::Array(items)
            }
            Some(PromptToken::Ident(s)) => {
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
        match self.peek_prompt() {
            Some(PromptToken::Capture(idx)) => {
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

    fn pt(t: PromptToken) -> AgentToken {
        AgentToken::Prompt(t)
    }

    #[test]
    fn parse_agent_with_tools() {
        let tokens = vec![
            pt(PromptToken::DirectiveRole("system".into())),
            pt(PromptToken::Text("You are an agent.".into())),
            AgentToken::DirectiveTools,
            pt(PromptToken::Capture(0)),
            pt(PromptToken::Eof),
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.tools_capture, Some(0));
        assert_eq!(tpl.sections.len(), 1);
    }

    #[test]
    fn parse_agent_with_on_hooks() {
        let tokens = vec![
            pt(PromptToken::DirectiveRole("system".into())),
            pt(PromptToken::Text("Agent.".into())),
            AgentToken::DirectiveOn("init".into()),
            pt(PromptToken::Capture(0)),
            AgentToken::DirectiveOn("error".into()),
            pt(PromptToken::Capture(1)),
            pt(PromptToken::Eof),
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.on_hooks.len(), 2);
        assert_eq!(tpl.on_hooks[0].event, "init");
        assert_eq!(tpl.on_hooks[0].capture_index, 0);
        assert_eq!(tpl.on_hooks[1].event, "error");
        assert_eq!(tpl.on_hooks[1].capture_index, 1);
    }

    #[test]
    fn parse_mixed_agent_and_prompt_directives() {
        let tokens = vec![
            pt(PromptToken::DirectiveModel),
            pt(PromptToken::Ident("claude-sonnet".into())),
            pt(PromptToken::DirectiveRole("system".into())),
            pt(PromptToken::Text("You are an agent.".into())),
            AgentToken::DirectiveTools,
            pt(PromptToken::Capture(0)),
            AgentToken::DirectiveSkills,
            pt(PromptToken::Capture(1)),
            pt(PromptToken::DirectiveConstraints),
            pt(PromptToken::BraceOpen),
            pt(PromptToken::Ident("temperature".into())),
            pt(PromptToken::Colon),
            pt(PromptToken::NumberLiteral(0.7)),
            pt(PromptToken::BraceClose),
            pt(PromptToken::Eof),
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert!(tpl.model.is_some());
        assert_eq!(tpl.model.as_ref().unwrap().models, vec!["claude-sonnet"]);
        assert_eq!(tpl.sections.len(), 1);
        assert_eq!(tpl.tools_capture, Some(0));
        assert_eq!(tpl.skills_capture, Some(1));
        assert!(tpl.constraints.is_some());
    }

    #[test]
    fn parse_full_agent_template() {
        let tokens = vec![
            pt(PromptToken::DirectiveModel),
            pt(PromptToken::Ident("claude-sonnet".into())),
            pt(PromptToken::Pipe),
            pt(PromptToken::Ident("gpt-4o".into())),
            pt(PromptToken::DirectiveRole("system".into())),
            pt(PromptToken::Text("You are ".into())),
            pt(PromptToken::Capture(0)),
            pt(PromptToken::Text(".".into())),
            AgentToken::DirectiveTools,
            pt(PromptToken::Capture(1)),
            AgentToken::DirectiveSkills,
            pt(PromptToken::Capture(2)),
            AgentToken::DirectiveAgents,
            pt(PromptToken::Capture(3)),
            AgentToken::DirectiveOn("init".into()),
            pt(PromptToken::Capture(4)),
            AgentToken::DirectiveOn("message".into()),
            pt(PromptToken::Capture(5)),
            pt(PromptToken::DirectiveOutput),
            pt(PromptToken::BraceOpen),
            pt(PromptToken::Ident("result".into())),
            pt(PromptToken::Colon),
            pt(PromptToken::Ident("str".into())),
            pt(PromptToken::BraceClose),
            pt(PromptToken::DirectiveConstraints),
            pt(PromptToken::BraceOpen),
            pt(PromptToken::Ident("temperature".into())),
            pt(PromptToken::Colon),
            pt(PromptToken::NumberLiteral(0.5)),
            pt(PromptToken::BraceClose),
            pt(PromptToken::Eof),
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert!(tpl.model.is_some());
        assert_eq!(tpl.model.as_ref().unwrap().models.len(), 2);
        assert_eq!(tpl.sections.len(), 1); // Role
        assert_eq!(tpl.tools_capture, Some(1));
        assert_eq!(tpl.skills_capture, Some(2));
        assert_eq!(tpl.agents_capture, Some(3));
        assert_eq!(tpl.on_hooks.len(), 2);
        assert!(tpl.output.is_some());
        assert!(tpl.constraints.is_some());
    }

    #[test]
    fn parse_error_missing_capture_after_tools() {
        let tokens = vec![
            pt(PromptToken::DirectiveRole("system".into())),
            pt(PromptToken::Text("Hello".into())),
            AgentToken::DirectiveTools,
            pt(PromptToken::Text("not a capture".into())),
            pt(PromptToken::Eof),
        ];
        let result = parse("test", &tokens);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("expected capture"));
        assert!(errs[0].message.contains("@tools"));
    }

    #[test]
    fn parse_error_empty_agent() {
        let tokens = vec![pt(PromptToken::Eof)];
        let result = parse("test", &tokens);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("empty agent"));
    }

    #[test]
    fn parse_error_duplicate_tools() {
        let tokens = vec![
            pt(PromptToken::DirectiveRole("system".into())),
            pt(PromptToken::Text("Hello".into())),
            AgentToken::DirectiveTools,
            pt(PromptToken::Capture(0)),
            AgentToken::DirectiveTools,
            pt(PromptToken::Capture(1)),
            pt(PromptToken::Eof),
        ];
        let result = parse("test", &tokens);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].message.contains("duplicate @tools"));
    }

    #[test]
    fn parse_implicit_system_role() {
        let tokens = vec![
            pt(PromptToken::Text("You are a helpful agent.".into())),
            AgentToken::DirectiveTools,
            pt(PromptToken::Capture(0)),
            pt(PromptToken::Eof),
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
    fn parse_messages_directive() {
        let tokens = vec![
            pt(PromptToken::DirectiveRole("system".into())),
            pt(PromptToken::Text("Hello".into())),
            pt(PromptToken::DirectiveMessages),
            pt(PromptToken::Capture(0)),
            pt(PromptToken::Eof),
        ];
        let tpl = parse("test", &tokens).unwrap();
        assert_eq!(tpl.sections.len(), 2);
        match &tpl.sections[1] {
            PromptSection::Messages { capture_index } => assert_eq!(*capture_index, 0),
            _ => panic!("expected Messages section"),
        }
    }
}
