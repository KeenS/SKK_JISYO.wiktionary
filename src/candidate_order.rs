use crate::model::Page;
use std::collections::HashMap;
use std::io;

pub type LinkFrequency = HashMap<String, usize>;

pub fn japanese_link_frequencies(xml: &str) -> io::Result<LinkFrequency> {
    let mut frequencies = LinkFrequency::new();
    let mut pages = crate::articles(xml)?;
    for page in pages.by_ref() {
        add_page_links(&mut frequencies, &page?);
    }
    if pages.skipped_pages() > 0 {
        eprintln!("sort_candidates: skipped {} pages", pages.skipped_pages());
    }
    Ok(frequencies)
}

pub fn add_page_links(frequencies: &mut LinkFrequency, page: &Page) {
    let Some(section) = page.japanese_text_with_default_sort() else {
        return;
    };
    for target in internal_link_targets(&section) {
        *frequencies.entry(target).or_default() += 1;
    }
}

pub fn internal_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut cursor = 0;

    while let Some(start) = text[cursor..].find("[[") {
        let start = cursor + start + 2;
        let Some(end) = text[start..].find("]]") else {
            break;
        };
        let end = start + end;
        if let Some(target) = link_target(&text[start..end]) {
            targets.push(target.to_string());
        }
        cursor = end + 2;
    }

    targets
}

fn link_target(link: &str) -> Option<&str> {
    let target = link.split('|').next()?;
    let target = target.split('#').next()?.trim();
    if target.is_empty() || target.starts_with(':') {
        return None;
    }
    Some(target)
}

pub fn sort_candidate_annotations(
    candidate_annotations: &[String],
    frequencies: &LinkFrequency,
) -> Vec<String> {
    let mut sorted = candidate_annotations.to_vec();
    sorted.sort_by(|left, right| {
        frequencies
            .get(candidate(left))
            .copied()
            .unwrap_or(0)
            .cmp(&frequencies.get(candidate(right)).copied().unwrap_or(0))
            .reverse()
            .then_with(|| {
                candidate_annotations
                    .iter()
                    .position(|value| value == left)
                    .unwrap_or(usize::MAX)
                    .cmp(
                        &candidate_annotations
                            .iter()
                            .position(|value| value == right)
                            .unwrap_or(usize::MAX),
                    )
            })
    });
    sorted
}

fn candidate(candidate_annotation: &str) -> &str {
    candidate_annotation.split(';').next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Revision;
    use std::collections::HashMap;

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
    fn extracts_japanese_links_only() {
        let mut frequencies = LinkFrequency::new();
        add_page_links(
            &mut frequencies,
            &page(
                "=={{ja}}==\n[[笑]] [[笑|えみ]] [[晒#日本語]]\n\
                 =={{en}}==\n[[唹]] [[Category:テスト]]",
            ),
        );

        let mut actual = frequencies.into_iter().collect::<Vec<_>>();
        actual.sort();
        assert_eq!(actual, vec![("晒".to_string(), 1), ("笑".to_string(), 2),]);
    }

    #[test]
    fn sorts_candidates_by_frequency_and_preserves_ties() {
        let mut frequencies = LinkFrequency::new();
        frequencies.insert("笑".to_string(), 42);
        frequencies.insert("咲".to_string(), 5);
        frequencies.insert("呵".to_string(), 1);

        let candidates = vec![
            "粲".to_string(),
            "笑".to_string(),
            "唹".to_string(),
            "咲".to_string(),
            "咥".to_string(),
            "呵;ア".to_string(),
            "听".to_string(),
        ];

        assert_eq!(
            sort_candidate_annotations(&candidates, &frequencies),
            vec![
                "笑".to_string(),
                "咲".to_string(),
                "呵;ア".to_string(),
                "粲".to_string(),
                "唹".to_string(),
                "咥".to_string(),
                "听".to_string(),
            ]
        );
    }

    #[test]
    fn sort_keeps_single_candidate_unchanged() {
        let candidates = vec!["学校".to_string()];
        assert_eq!(
            sort_candidate_annotations(&candidates, &HashMap::new()),
            candidates
        );
    }
}
