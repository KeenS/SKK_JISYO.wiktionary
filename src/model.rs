use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Mediawiki {
    pub page: Vec<Page>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    pub ns: u64,
    pub id: u64,
    pub title: String,
    pub revision: Revision,
}

impl Page {
    pub fn japanese_text(&self) -> Option<&str> {
        let start = [
            "=={{L|ja}}==",
            "== {{L|ja}} ==",
            "=={{ja}}==",
            "== {{ja}} ==",
        ]
        .into_iter()
        .filter_map(|marker| self.revision.text.find(marker))
        .min()?;
        let remainder = &self.revision.text[start..];
        let end = remainder
            .char_indices()
            .filter(|(offset, _)| {
                self.revision.text.as_bytes()[start + offset..].starts_with(b"\n==")
            })
            .find(|(offset, _)| {
                self.revision
                    .text
                    .as_bytes()
                    .get(start + offset + 3)
                    .is_some_and(|byte| *byte != b'=')
            })
            .map(|(offset, _)| start + offset)
            .unwrap_or(self.revision.text.len());
        Some(&self.revision.text[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(text: &str) -> Page {
        Page {
            ns: 0,
            id: 1,
            title: "テスト".to_string(),
            revision: Revision {
                id: 1,
                comment: None,
                text: text.to_string(),
            },
        }
    }

    #[test]
    fn extracts_supported_japanese_sections() {
        for marker in [
            "=={{L|ja}}==",
            "== {{L|ja}} ==",
            "=={{ja}}==",
            "== {{ja}} ==",
        ] {
            let text = format!("{marker}\n{{{{ja-noun|テスト}}}}\n=={{en}}==\nEnglish");
            assert_eq!(
                page(&text).japanese_text(),
                Some(format!("{marker}\n{{{{ja-noun|テスト}}}}").as_str())
            );
        }
    }

    #[test]
    fn returns_none_without_japanese_section() {
        assert!(page("=={{en}}==\nEnglish").japanese_text().is_none());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Revision {
    pub id: u64,
    // timestamp: String,
    pub comment: Option<String>,
    pub text: String,
}
