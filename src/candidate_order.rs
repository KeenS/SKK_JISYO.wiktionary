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

/// Okuri-ari headings already follow the order written on the reading page.
/// Link frequency of the bare kanji would move a rare spelling such as 関く
/// ahead of 吐く, because 関 itself is linked often.
fn okuri_consonant(reading: &str) -> Option<char> {
    let consonant = reading.chars().next_back()?;
    consonant
        .is_ascii_alphabetic()
        .then_some(consonant.to_ascii_lowercase())
}

pub fn sort_candidate_annotations(
    reading: &str,
    candidate_annotations: &[String],
    frequencies: &LinkFrequency,
) -> Vec<String> {
    if okuri_consonant(reading).is_some() {
        return candidate_annotations.to_vec();
    }
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
            sort_candidate_annotations("わらう", &candidates, &frequencies),
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
    fn okuri_ari_keeps_the_order_written_on_the_page() {
        let mut frequencies = LinkFrequency::new();
        frequencies.insert("関".to_string(), 100);
        frequencies.insert("着".to_string(), 80);
        frequencies.insert("吐".to_string(), 3);

        let candidates = vec![
            "吐".to_string(),
            "嘔".to_string(),
            "掃".to_string(),
            "刷".to_string(),
            "履".to_string(),
            "穿".to_string(),
            "佩".to_string(),
            "着".to_string(),
            "関".to_string(),
        ];

        assert_eq!(
            sort_candidate_annotations("はk", &candidates, &frequencies),
            candidates
        );
    }

    #[test]
    fn sort_keeps_single_candidate_unchanged() {
        let candidates = vec!["学校".to_string()];
        assert_eq!(
            sort_candidate_annotations("がっこう", &candidates, &LinkFrequency::new()),
            candidates
        );
    }
}
