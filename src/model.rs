#[derive(Debug)]
pub struct Page {
    pub ns: u64,
    pub id: u64,
    pub title: String,
    pub revision: Revision,
}

impl Page {
    pub fn is_symbol_title(&self) -> bool {
        self.title
            .chars()
            .all(|ch| !ch.is_alphanumeric() && !is_kanji(ch))
    }

    pub fn symbol_text(&self) -> Option<&str> {
        if self.is_symbol_title() {
            Some(&self.revision.text)
        } else {
            None
        }
    }

    pub fn japanese_text(&self) -> Option<&str> {
        let start = self.find_language_start("ja")?;
        let end = self.next_language_section_offset(start);
        let section = &self.revision.text[start..end];
        Some(section.strip_suffix('\n').unwrap_or(section))
    }

    pub fn old_japanese_text_with_default_sort(&self) -> Option<String> {
        let start = self.find_language_start("ojp")?;
        let end = self.next_language_section_offset(start + 1);
        let section = &self.revision.text[start..end];
        Some(format!(
            "{}{}",
            self.default_sort_prefix(start),
            section.strip_suffix('\n').unwrap_or(section)
        ))
    }

    fn find_language_start(&self, language: &str) -> Option<usize> {
        let marked = format!("=={{{{{language}}}}}==");
        let marked_spaced = format!("== {{{{{language}}}}} ==");
        let linked = format!("=={{{{L|{language}}}}}==");
        let linked_spaced = format!("== {{{{L|{language}}}}} ==");
        let native_marked = format!("=={language}語==");
        let native_spaced = format!("== {language}語 ==");
        [
            linked,
            linked_spaced,
            marked,
            marked_spaced,
            native_marked,
            native_spaced,
            if language == "ojp" {
                "==古典日本語==".to_string()
            } else {
                String::new()
            },
            if language == "ojp" {
                "== 古典日本語 ==".to_string()
            } else {
                String::new()
            },
        ]
        .into_iter()
        .filter(|marker| !marker.is_empty())
        .filter_map(|marker| {
            self.revision
                .text
                .find(marker.as_str())
                .map(|start| (start, marker))
        })
        .min_by_key(|(start, _)| *start)
        .map(|(start, _)| start)
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
            "==日本語==",
            "== 日本語 ==",
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

fn is_kanji(ch: char) -> bool {
    matches!(ch, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}')
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
    fn extracts_old_japanese_sections() {
        let text = "=={{ja}}==\n現代\n\n=={{L|ojp}}==\n古語\n\n==日本手話==\n手話";
        let page = page(text);
        assert_eq!(
            page.old_japanese_text_with_default_sort().unwrap(),
            "=={{L|ojp}}==\n古語"
        );
    }

    #[test]
    fn extracts_unmarked_old_japanese_section() {
        let text = "== 古典日本語 ==\n{{ojp-verb}}【[[歩]]く】\n==日本手話==";
        let page = page(text);
        assert_eq!(
            page.old_japanese_text_with_default_sort().unwrap(),
            "== 古典日本語 ==\n{{ojp-verb}}【[[歩]]く】"
        );
    }

    #[test]
    fn old_japanese_section_keeps_defaultsort() {
        let text = "{{DEFAULTSORT:あるく}}\n=={{ja}}==\n現代\n=={{L|ojp}}==\n古語";
        let page = page(text);
        assert_eq!(
            page.old_japanese_text_with_default_sort().unwrap(),
            "{{DEFAULTSORT:あるく}}\n=={{L|ojp}}==\n古語"
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
