#[derive(Debug)]
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
        let end = self.next_language_section_offset(start);
        Some(&self.revision.text[start..end])
    }

    fn next_language_section_offset(&self, start: usize) -> usize {
        self.revision.text[start..]
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
            .unwrap_or(self.revision.text.len())
    }

    fn default_sort_prefix(&self, start: usize) -> String {
        let Some(default_sort_start) = self
            .revision
            .text
            .find("{{DEFAULTSORT:")
            .or_else(|| self.revision.text.find("{{DEFAULTSORT|"))
            .or_else(|| self.revision.text.find("{{kana-DEFAULTSORT|"))
        else {
            return String::new();
        };
        if default_sort_start >= start {
            return String::new();
        }
        self.revision.text[default_sort_start..]
            .find('\n')
            .map(|offset| {
                let end = default_sort_start + offset;
                format!("{}\n", &self.revision.text[default_sort_start..end])
            })
            .unwrap_or_else(|| self.revision.text[default_sort_start..].to_string())
    }

    pub fn japanese_text_with_default_sort(&self) -> Option<String> {
        let start = [
            "=={{L|ja}}==",
            "== {{L|ja}} ==",
            "=={{ja}}==",
            "== {{ja}} ==",
        ]
        .into_iter()
        .filter_map(|marker| self.revision.text.find(marker))
        .min()?;
        let end = self.next_language_section_offset(start);
        Some(format!(
            "{}{}",
            self.default_sort_prefix(start),
            &self.revision.text[start..end]
        ))
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
    fn keeps_defaultsort_before_japanese_section() {
        let text = "{{DEFAULTSORT:しめんそか}}\n=={{ja}}==\n{{ja-idiom|しめんそか}}\n=={{en}}==";
        assert_eq!(
            page(text).japanese_text_with_default_sort().unwrap(),
            "{{DEFAULTSORT:しめんそか}}\n=={{ja}}==\n{{ja-idiom|しめんそか}}"
        );
    }

    #[test]
    fn ignores_defaultsort_inside_japanese_section() {
        let text = "=={{ja}}==\n{{DEFAULTSORT:しめんそか}}\n{{ja-idiom|しめんそか}}";
        assert_eq!(
            page(text).japanese_text_with_default_sort().unwrap(),
            "=={{ja}}==\n{{DEFAULTSORT:しめんそか}}\n{{ja-idiom|しめんそか}}"
        );
    }

    #[test]
    fn returns_none_without_japanese_section() {
        assert!(page("=={{en}}==\nEnglish").japanese_text().is_none());
    }
}

#[derive(Debug)]
pub struct Revision {
    pub id: u64,
    pub comment: Option<String>,
    pub text: String,
}
