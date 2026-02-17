use crate::ComponentMeta;
use std::collections::HashSet;

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

const KNOWN_PRIMITIVE_TYPES: &[&str] = &["str", "num", "int", "bool", "nil", "any"];

pub fn validate(meta: &ComponentMeta) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check for empty props
    if meta.props.is_empty() {
        diagnostics.push(Diagnostic {
            message: "component has no props defined".to_string(),
            severity: Severity::Warning,
        });
    }

    // Check for duplicate prop names
    let mut seen = HashSet::new();
    for prop in &meta.props {
        if !seen.insert(&prop.name) {
            diagnostics.push(Diagnostic {
                message: format!("duplicate prop name `{}`", prop.name),
                severity: Severity::Error,
            });
        }
    }

    // Check for unknown prop types
    for prop in &meta.props {
        if !is_known_type(&prop.ty) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "prop `{}` has unknown type `{}`",
                    prop.name, prop.ty
                ),
                severity: Severity::Warning,
            });
        }
    }

    diagnostics
}

fn is_known_type(ty: &str) -> bool {
    if KNOWN_PRIMITIVE_TYPES.contains(&ty) {
        return true;
    }
    // Array syntax: [T]
    if ty.starts_with('[') && ty.ends_with(']') {
        let inner = &ty[1..ty.len() - 1];
        return is_known_type(inner);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ComponentProp;

    fn make_meta(props: Vec<ComponentProp>) -> ComponentMeta {
        ComponentMeta {
            name: "TestComponent".to_string(),
            description: None,
            props,
        }
    }

    fn prop(name: &str, ty: &str) -> ComponentProp {
        ComponentProp {
            name: name.to_string(),
            ty: ty.to_string(),
            description: None,
            has_default: false,
        }
    }

    #[test]
    fn valid_component() {
        let meta = make_meta(vec![
            prop("name", "str"),
            prop("count", "int"),
            prop("items", "[str]"),
        ]);
        let diags = validate(&meta);
        assert!(diags.iter().all(|d| d.severity != Severity::Error));
    }

    #[test]
    fn duplicate_props() {
        let meta = make_meta(vec![
            prop("name", "str"),
            prop("name", "int"),
        ]);
        let diags = validate(&meta);
        assert!(
            diags.iter().any(|d| d.severity == Severity::Error && d.message.contains("duplicate prop")),
            "expected duplicate prop error, got: {:?}",
            diags
        );
    }

    #[test]
    fn unknown_type() {
        let meta = make_meta(vec![prop("data", "CustomType")]);
        let diags = validate(&meta);
        assert!(
            diags.iter().any(|d| d.severity == Severity::Warning && d.message.contains("unknown type")),
            "expected unknown type warning, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_props_warning() {
        let meta = make_meta(vec![]);
        let diags = validate(&meta);
        assert!(
            diags.iter().any(|d| d.severity == Severity::Warning && d.message.contains("no props")),
            "expected no props warning, got: {:?}",
            diags
        );
    }
}
