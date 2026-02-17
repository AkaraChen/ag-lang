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

const KNOWN_EVENTS: &[&str] = &["init", "message", "error"];

pub fn validate(template: &AgentTemplate) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check for duplicate @on hooks with the same event name
    let mut seen_events = Vec::new();
    for hook in &template.on_hooks {
        if seen_events.contains(&hook.event) {
            diagnostics.push(Diagnostic {
                message: format!("duplicate @on handler for event '{}'", hook.event),
                severity: Severity::Error,
            });
        } else {
            seen_events.push(hook.event.clone());
        }
    }

    // Check for unknown @on event names
    for hook in &template.on_hooks {
        if !KNOWN_EVENTS.contains(&hook.event.as_str()) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "unknown event '{}'; known events are: {}",
                    hook.event,
                    KNOWN_EVENTS.join(", ")
                ),
                severity: Severity::Warning,
            });
        }
    }

    // Check for missing role sections or body text
    let has_role = template
        .sections
        .iter()
        .any(|s| matches!(s, ag_dsl_prompt::ast::PromptSection::Role { .. }));
    if !has_role {
        diagnostics.push(Diagnostic {
            message: "no @role directive; agent has no prompt body".to_string(),
            severity: Severity::Warning,
        });
    }

    // Empty agent check (no sections and no agent-specific features)
    let has_agent_features = template.tools_capture.is_some()
        || template.skills_capture.is_some()
        || template.agents_capture.is_some()
        || !template.on_hooks.is_empty();

    if template.sections.is_empty()
        && template.model.is_none()
        && template.output.is_none()
        && template.constraints.is_none()
        && !has_agent_features
    {
        diagnostics.push(Diagnostic {
            message: "empty agent template".to_string(),
            severity: Severity::Error,
        });
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use ag_dsl_prompt::ast::*;

    fn minimal_agent() -> AgentTemplate {
        AgentTemplate {
            name: "test".to_string(),
            sections: vec![PromptSection::Role {
                role: RoleName::System,
                body: vec![PromptPart::Text("You are an agent.".to_string())],
            }],
            model: None,
            output: None,
            constraints: None,
            tools_capture: Some(0),
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        }
    }

    #[test]
    fn valid_agent_no_errors() {
        let tpl = minimal_agent();
        let diags = validate(&tpl);
        assert!(
            diags.iter().all(|d| d.severity != Severity::Error),
            "expected no errors, got: {:?}",
            diags
        );
    }

    #[test]
    fn duplicate_on_event_produces_error() {
        let mut tpl = minimal_agent();
        tpl.on_hooks = vec![
            OnHook {
                event: "init".to_string(),
                capture_index: 0,
            },
            OnHook {
                event: "init".to_string(),
                capture_index: 1,
            },
        ];
        let diags = validate(&tpl);
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Error && d.message.contains("duplicate @on")));
    }

    #[test]
    fn unknown_on_event_produces_warning() {
        let mut tpl = minimal_agent();
        tpl.on_hooks = vec![OnHook {
            event: "shutdown".to_string(),
            capture_index: 0,
        }];
        let diags = validate(&tpl);
        assert!(diags.iter().any(
            |d| d.severity == Severity::Warning && d.message.contains("unknown event 'shutdown'")
        ));
    }

    #[test]
    fn known_events_pass_without_warning() {
        let mut tpl = minimal_agent();
        tpl.on_hooks = vec![
            OnHook {
                event: "init".to_string(),
                capture_index: 0,
            },
            OnHook {
                event: "message".to_string(),
                capture_index: 1,
            },
            OnHook {
                event: "error".to_string(),
                capture_index: 2,
            },
        ];
        let diags = validate(&tpl);
        assert!(
            diags
                .iter()
                .all(|d| !d.message.contains("unknown event")),
            "expected no unknown event warnings, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_role_produces_warning() {
        let tpl = AgentTemplate {
            name: "test".to_string(),
            sections: vec![],
            model: None,
            output: None,
            constraints: None,
            tools_capture: Some(0),
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let diags = validate(&tpl);
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("no @role")));
    }

    #[test]
    fn empty_agent_produces_error() {
        let tpl = AgentTemplate {
            name: "test".to_string(),
            sections: vec![],
            model: None,
            output: None,
            constraints: None,
            tools_capture: None,
            skills_capture: None,
            agents_capture: None,
            on_hooks: vec![],
        };
        let diags = validate(&tpl);
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Error && d.message.contains("empty agent")));
    }
}
