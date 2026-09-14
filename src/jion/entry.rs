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

    pub fn joined_candidates(&self) -> String {
        self.candidates.join("/")
    }

    pub fn candidate_annotations(&self) -> Vec<String> {
        self.candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                self.annotations
                    .get(index)
                    .map(|annotation| format!("{candidate};{annotation}"))
                    .unwrap_or_else(|| candidate.clone())
            })
            .collect()
    }

    pub fn from_candidate_annotations(
        reading: impl Into<String>,
        candidate_annotations: Vec<String>,
    ) -> Self {
        let mut candidates = Vec::with_capacity(candidate_annotations.len());
        let mut annotations = Vec::new();

        for candidate_annotation in candidate_annotations {
            match candidate_annotation.split_once(';') {
                Some((candidate, annotation)) => {
                    candidates.push(candidate.to_string());
                    annotations.push(annotation.to_string());
                }
                None => candidates.push(candidate_annotation),
            }
        }

        Self {
            reading: reading.into(),
            candidates,
            annotations,
        }
    }

    pub fn to_line(&self) -> String {
        format!(
            "{} /{}/\n",
            self.reading,
            self.candidate_annotations().join("/")
        )
    }
}
