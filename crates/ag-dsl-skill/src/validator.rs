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

/// Validate a parsed `SkillTemplate`, returning any diagnostics found.
///
/// An empty return value means the template is valid.
pub fn validate(template: &SkillTemplate) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if template.description.is_none() {
        diagnostics.push(Diagnostic::error("missing @description directive"));
    }

    if template.input_fields.is_empty() {
        diagnostics.push(Diagnostic::error("missing @input directive"));
    }

    if template.steps.is_empty() {
        diagnostics.push(Diagnostic::error("missing @steps directive"));
    }

    // output_fields is optional — no error if empty.

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_template() -> SkillTemplate {
        SkillTemplate {
            name: "test_skill".to_string(),
            description: Some("A test skill".to_string()),
            input_fields: vec![SkillField {
                name: "query".to_string(),
                type_name: "str".to_string(),
                default: None,
            }],
            steps: vec![SkillStep {
                number: 1,
                text: "Do the thing".to_string(),
                captures: vec![],
            }],
            output_fields: vec![SkillField {
                name: "result".to_string(),
                type_name: "str".to_string(),
                default: None,
            }],
        }
    }

    #[test]
    fn test_valid_template_passes() {
        let diags = validate(&valid_template());
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    }

    #[test]
    fn test_missing_description() {
        let mut tmpl = valid_template();
        tmpl.description = None;
        let diags = validate(&tmpl);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("@description"));
    }

    #[test]
    fn test_missing_input() {
        let mut tmpl = valid_template();
        tmpl.input_fields.clear();
        let diags = validate(&tmpl);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("@input"));
    }

    #[test]
    fn test_missing_steps() {
        let mut tmpl = valid_template();
        tmpl.steps.clear();
        let diags = validate(&tmpl);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("@steps"));
    }

    #[test]
    fn test_missing_output_is_ok() {
        let mut tmpl = valid_template();
        tmpl.output_fields.clear();
        let diags = validate(&tmpl);
        assert!(
            diags.is_empty(),
            "output is optional, but got diagnostics: {:?}",
            diags
        );
    }

    #[test]
    fn test_multiple_missing() {
        let tmpl = SkillTemplate {
            name: "empty".to_string(),
            description: None,
            input_fields: vec![],
            steps: vec![],
            output_fields: vec![],
        };
        let diags = validate(&tmpl);
        assert_eq!(diags.len(), 3);
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
        let messages: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.contains(&"missing @description directive"));
        assert!(messages.contains(&"missing @input directive"));
        assert!(messages.contains(&"missing @steps directive"));
    }
}
