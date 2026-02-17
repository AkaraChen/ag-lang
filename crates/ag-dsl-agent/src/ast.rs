// ── Agent AST definitions ────────────────────────────────

use ag_dsl_prompt::ast::{Constraints, ModelSpec, OutputSpec, PromptSection};

#[derive(Debug, Clone)]
pub struct AgentTemplate {
    pub name: String,
    pub sections: Vec<PromptSection>,
    pub model: Option<ModelSpec>,
    pub output: Option<OutputSpec>,
    pub constraints: Option<Constraints>,
    pub tools_capture: Option<usize>,
    pub skills_capture: Option<usize>,
    pub agents_capture: Option<usize>,
    pub on_hooks: Vec<OnHook>,
}

#[derive(Debug, Clone)]
pub struct OnHook {
    pub event: String,
    pub capture_index: usize,
}
