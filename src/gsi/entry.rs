use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub reading: String,
    pub candidates: BTreeMap<String, usize>,
}

impl Entry {
    pub fn add_candidate(&mut self, candidate: impl Into<String>) {
        let next_order = self.candidates.values().copied().max().unwrap_or_default() + 1;
        self.candidates
            .entry(candidate.into())
            .or_insert(next_order);
    }

    pub fn to_line(&self) -> String {
        format!(
            "{} /{}/\n",
            self.reading,
            joined_candidates(&self.candidates)
        )
    }
}

fn joined_candidates(candidates: &BTreeMap<String, usize>) -> String {
    let mut candidates = candidates.iter().collect::<Vec<_>>();
    candidates.sort_by_key(|(_, order)| **order);
    candidates
        .into_iter()
        .map(|(candidate, _)| candidate.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_candidates_in_insertion_order() {
        let mut entry = Entry {
            reading: "なかつ".to_string(),
            candidates: BTreeMap::new(),
        };
        entry.add_candidate("中津");
        entry.add_candidate("中津市");
        assert_eq!(entry.to_line(), "なかつ /中津/中津市/\n");
    }
}
