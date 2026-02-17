#[derive(Debug, Clone)]
pub struct SkillTemplate {
    pub name: String,
    pub description: Option<String>,
    pub input_fields: Vec<SkillField>,
    pub steps: Vec<SkillStep>,
    pub output_fields: Vec<SkillField>,
}

#[derive(Debug, Clone)]
pub struct SkillField {
    pub name: String,
    pub type_name: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SkillStep {
    pub number: u32,
    pub text: String,
    pub captures: Vec<usize>,
}
