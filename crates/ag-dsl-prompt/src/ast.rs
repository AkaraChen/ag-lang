// ── Prompt AST definitions ────────────────────────────────

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub sections: Vec<PromptSection>,
    pub model: Option<ModelSpec>,
    pub output: Option<OutputSpec>,
    pub constraints: Option<Constraints>,
}

#[derive(Debug, Clone)]
pub enum PromptSection {
    Role {
        role: RoleName,
        body: Vec<PromptPart>,
    },
    Examples(Vec<Example>),
    Messages {
        capture_index: usize,
    },
}

#[derive(Debug, Clone)]
pub enum PromptPart {
    Text(String),
    Capture(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoleName {
    System,
    User,
    Assistant,
    Custom(String),
}

impl RoleName {
    pub fn as_str(&self) -> &str {
        match self {
            RoleName::System => "system",
            RoleName::User => "user",
            RoleName::Assistant => "assistant",
            RoleName::Custom(s) => s,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "system" => RoleName::System,
            "user" => RoleName::User,
            "assistant" => RoleName::Assistant,
            other => RoleName::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Example {
    pub pairs: Vec<(RoleName, String)>,
}

#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub models: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OutputSpec {
    pub kind: OutputKind,
}

#[derive(Debug, Clone)]
pub enum OutputKind {
    CaptureRef(usize),
    Inline(Vec<OutputField>),
}

#[derive(Debug, Clone)]
pub struct OutputField {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone)]
pub struct Constraints {
    pub fields: Vec<(String, ConstraintValue)>,
}

#[derive(Debug, Clone)]
pub enum ConstraintValue {
    Number(f64),
    String(String),
    Array(Vec<ConstraintValue>),
    Bool(bool),
}
