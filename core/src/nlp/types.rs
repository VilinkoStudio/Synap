#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NlpDocument {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub active: bool,
}

impl NlpDocument {
    pub fn new(id: impl Into<String>, content: impl Into<String>, tags: Vec<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            tags,
            active: true,
        }
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagSuggestion {
    pub tag: String,
    pub score: f32,
}
