#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub reading: String,
    pub candidates: Vec<String>,
    pub annotations: Vec<String>,
}

impl Entry {
    pub fn new(reading: impl Into<String>, candidate: impl Into<String>) -> Self {
        Self {
            reading: reading.into(),
            candidates: vec![candidate.into()],
            annotations: Vec::new(),
        }
    }

    pub fn to_line(&self) -> String {
        let mut candidates = self.candidates.join("/");
        if !self.annotations.is_empty() {
            candidates.push(';');
            candidates.push_str(&self.annotations.join(";"));
        }
        format!("{} /{}/\n", self.reading, candidates)
    }
}
