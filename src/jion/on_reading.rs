use once_cell::sync::OnceCell;
use regex::Regex;
use std::collections::HashMap;
use wana_kana::ConvertJapanese;

use super::mapping::Mapping;

const MIN_INFERENCE_SUPPORT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnReading {
    pub modern: String,
    pub historical: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingMapping {
    pub modern: String,
    pub historicals: Vec<String>,
}

fn is_on_reading(reading: &str) -> bool {
    !reading.is_empty()
        && reading
            .chars()
            .all(|ch| matches!(ch, 'ァ'..='ヶ' | 'ゃ'..='ょ'))
}

fn on_reading_regex() -> &'static Regex {
    static REGEX: OnceCell<Regex> = OnceCell::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?:呉音|漢音|唐音|宋音|慣用音)(?:\d+)?\s*=\s*([^|;}]+)")
            .expect("internal error: invalid regex")
    })
}

fn kanji_template_regex() -> &'static Regex {
    static REGEX: OnceCell<Regex> = OnceCell::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\{\{ja-kanji\|([^\n]+?)\}\}").expect("internal error: invalid regex")
    })
}

pub fn kanji_template_params(text: &str) -> Option<&str> {
    kanji_template_regex()
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|params| params.as_str())
}

pub fn has_on_reading(text: &str, reading: &str) -> bool {
    let Some(params) = kanji_template_params(text) else {
        return false;
    };
    let reading = reading.to_hiragana();
    parse_on_readings(params)
        .iter()
        .any(|candidate| candidate.modern == reading)
}

pub fn parse_on_readings(params: &str) -> Vec<OnReading> {
    let mut readings = Vec::new();
    for captures in on_reading_regex().captures_iter(params) {
        for pair in captures[1].split(',') {
            let (modern, historical) = pair.split_once('<').unwrap_or((pair, ""));
            let modern = modern.split(';').next().unwrap_or_default().trim();
            if !is_on_reading(modern) {
                continue;
            }
            let historical = historical.trim();
            let reading = OnReading {
                modern: modern.to_hiragana(),
                historical: if is_on_reading(historical) {
                    Some(historical.to_hiragana())
                } else {
                    None
                },
            };
            if !readings.contains(&reading) {
                readings.push(reading);
            }
        }
    }
    readings
}

pub fn parse_kun_readings(params: &str) -> Vec<String> {
    parse_named_readings(params, "訓")
}

pub fn parse_common_readings(params: &str) -> Vec<String> {
    let mut readings = parse_named_readings(params, "常用");
    for reading in parse_kun_readings(params) {
        if !readings.contains(&reading) {
            readings.push(reading);
        }
    }
    readings
}

pub fn parse_dictionary_on_readings(params: &str) -> Vec<String> {
    parse_on_readings(params)
        .into_iter()
        .map(|reading| reading.modern)
        .filter(|reading| is_hiragana_reading(reading))
        .collect()
}

fn parse_named_readings(params: &str, name: &str) -> Vec<String> {
    let mut readings = Vec::new();
    let pattern = format!(
        r"{name}(?:\d+)?\s*=\s*([^|;}}]+)",
        name = regex::escape(name)
    );
    let regex = Regex::new(&pattern).expect("internal error: invalid named reading regex");
    for captures in regex.captures_iter(params) {
        for pair in captures[1].split(',') {
            let reading = pair.split('-').next().unwrap_or_default().trim();
            let reading = reading.split('<').next().unwrap_or_default().trim();
            let reading = katakana_to_hiragana(reading);
            if is_hiragana_reading(&reading) && !readings.contains(&reading) {
                readings.push(reading);
            }
        }
    }
    readings
}

pub fn katakana_to_hiragana(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ('ァ'..='ヶ').contains(&ch) {
                char::from_u32(ch as u32 - 0x60).unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect()
}

fn is_hiragana_reading(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ('ぁ'..='ゖ').contains(&ch))
}

pub fn build_historical_inference(mappings: &[Mapping]) -> HashMap<String, String> {
    let mut counts = HashMap::new();
    for mapping in mappings {
        if mapping.modern == mapping.historical {
            continue;
        }
        *counts
            .entry((mapping.modern.clone(), mapping.historical.clone()))
            .or_insert(0) += 1;
    }

    let mut by_modern: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for ((modern, historical), count) in counts {
        by_modern
            .entry(modern)
            .or_default()
            .push((historical, count));
    }

    let mut result = HashMap::new();
    for (modern, historicals) in by_modern {
        let total: usize = historicals.iter().map(|(_, count)| count).sum();
        let mut best: Option<(String, usize)> = None;
        let mut tied = false;
        for (historical, count) in historicals {
            match &best {
                Some((_, best_count)) if count > *best_count => {
                    best = Some((historical, count));
                    tied = false;
                }
                Some((_, best_count)) if count == *best_count => tied = true,
                None => best = Some((historical, count)),
                _ => {}
            }
        }
        let Some((historical, count)) = best else {
            continue;
        };
        let others = total - count;
        if tied || count < MIN_INFERENCE_SUPPORT || count <= others {
            continue;
        }
        result.insert(modern, historical);
    }
    result
}

pub fn resolve_readings(
    readings: &[OnReading],
    inference: &HashMap<String, String>,
) -> Vec<ReadingMapping> {
    let mut mappings: Vec<ReadingMapping> = Vec::new();
    for reading in readings {
        let Some(mapping) = mappings
            .iter_mut()
            .find(|mapping| mapping.modern == reading.modern)
        else {
            mappings.push(ReadingMapping {
                modern: reading.modern.clone(),
                historicals: Vec::new(),
            });
            if let Some(historical) = &reading.historical {
                mappings
                    .last_mut()
                    .unwrap()
                    .historicals
                    .push(historical.clone());
            }
            continue;
        };
        if let Some(historical) = &reading.historical {
            if !mapping.historicals.contains(historical) {
                mapping.historicals.push(historical.clone());
            }
        }
    }

    for mapping in &mut mappings {
        if mapping.historicals.is_empty() {
            if let Some(historical) = inference.get(&mapping.modern) {
                mapping.historicals.push(historical.clone());
            }
        }
        if mapping.historicals.is_empty() {
            mapping.historicals.push(mapping.modern.clone());
        }
    }
    mappings
}

impl ReadingMapping {
    pub fn historical_entries(&self) -> impl Iterator<Item = &str> {
        self.historicals
            .iter()
            .map(String::as_str)
            .filter(|historical| *historical != self.modern)
    }

    pub fn annotations(&self) -> Vec<String> {
        self.historical_entries()
            .map(|historical| historical.to_katakana())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_and_unpaired_readings() {
        let readings =
            parse_on_readings("常用=セイ|施策=教育:1|呉音=ショウ<シャウ|漢音=セイ|唐音=チン,シイ");
        assert_eq!(
            readings,
            vec![
                OnReading {
                    modern: "しょう".into(),
                    historical: Some("しゃう".into()),
                },
                OnReading {
                    modern: "せい".into(),
                    historical: None,
                },
                OnReading {
                    modern: "ちん".into(),
                    historical: None,
                },
                OnReading {
                    modern: "しい".into(),
                    historical: None,
                },
            ]
        );
    }

    #[test]
    fn ignores_invalid_reading_pairs() {
        let readings = parse_on_readings("呉音=*|漢音=ゲ;[[芸]]");
        assert_eq!(
            readings,
            vec![OnReading {
                modern: "げ".into(),
                historical: None,
            }]
        );
    }

    #[test]
    fn parses_common_readings_with_kun_and_historical_forms() {
        let readings =
            parse_common_readings("常用=リョウ|施策=教育:4|呉音=リョウ<レウ|漢音=リョウ<レウ");
        assert_eq!(readings, vec!["りょう".to_string()]);
    }

    #[test]
    fn parses_kun_readings_from_kanji_template() {
        let readings = parse_kun_readings(
            "常用=カイ,エ,あ-う|施策=教育:2|呉音=エ&lt;ヱ,ケ|漢音=カイ&lt;クヮイ|訓=あ-う,たまたま,あつ-まる,あつ-める",
        );
        assert!(readings.contains(&"あ".to_string()));
        assert!(readings.contains(&"たまたま".to_string()));
        assert!(readings.contains(&"あつ".to_string()));
    }

    #[test]
    fn parses_kun_readings_with_okuri_marker() {
        let readings = parse_kun_readings("訓=あ-う,たまたま,あつ-まる,あつ-める");
        assert_eq!(
            readings,
            vec!["あ".to_string(), "たまたま".to_string(), "あつ".to_string()]
        );
    }

    #[test]
    fn parses_all_dictionary_on_reading_fields() {
        let readings = parse_dictionary_on_readings(
            "常用=セイ|呉音1=ショウ|呉音2=ス|漢音=セイ|唐音=チン,シイ|宋音=ソン|慣用音=ゼイ",
        );
        assert_eq!(
            readings,
            vec![
                "しょう".to_string(),
                "す".to_string(),
                "せい".to_string(),
                "ちん".to_string(),
                "しい".to_string(),
                "そん".to_string(),
                "ぜい".to_string(),
            ]
        );
    }

    #[test]
    fn parses_annotated_common_readings_until_annotation() {
        let readings =
            parse_common_readings("常用=セイ,ショウ,あお,あお-い,あき;[[w:明|明]]|名乗=あき");
        assert_eq!(
            readings,
            vec![
                "せい".to_string(),
                "しょう".to_string(),
                "あお".to_string(),
                "あき".to_string()
            ]
        );
    }

    #[test]
    fn resolves_unpaired_readings_with_inference() {
        let readings = parse_on_readings("呉音=ショウ<シャウ|漢音=セイ|唐音=チン");
        let inference = HashMap::from([
            ("せい".to_string(), "せぃ".to_string()),
            ("ちん".to_string(), "ちむ".to_string()),
        ]);
        let mappings = resolve_readings(&readings, &inference);
        assert_eq!(
            mappings,
            vec![
                ReadingMapping {
                    modern: "しょう".into(),
                    historicals: vec!["しゃう".into()],
                },
                ReadingMapping {
                    modern: "せい".into(),
                    historicals: vec!["せぃ".into()],
                },
                ReadingMapping {
                    modern: "ちん".into(),
                    historicals: vec!["ちむ".into()],
                },
            ]
        );
    }

    #[test]
    fn keeps_identity_mapping_for_segmentation_without_dictionary_entry() {
        let readings = parse_on_readings("呉音=シ|漢音=シ|唐音=ス");
        let mappings = resolve_readings(&readings, &HashMap::new());
        assert_eq!(
            mappings,
            vec![
                ReadingMapping {
                    modern: "し".into(),
                    historicals: vec!["し".into()],
                },
                ReadingMapping {
                    modern: "す".into(),
                    historicals: vec!["す".into()],
                },
            ]
        );
        assert!(mappings[0].historical_entries().next().is_none());
        assert!(mappings[0].annotations().is_empty());
    }

    #[test]
    fn inference_uses_explicit_non_identity_pairs() {
        let explicit = (0..5)
            .map(|index| Mapping {
                kanji: format!("字{index}"),
                modern: "せい".into(),
                historical: "せぃ".into(),
            })
            .chain(std::iter::once(Mapping {
                kanji: "同".into(),
                modern: "どう".into(),
                historical: "どう".into(),
            }))
            .collect::<Vec<_>>();
        let inference = build_historical_inference(&explicit);
        assert_eq!(inference.get("せい").map(String::as_str), Some("せぃ"));
        assert!(!inference.contains_key("どう"));
    }

    fn inference_mapping(modern: &str, historical: &str) -> Mapping {
        Mapping {
            kanji: String::new(),
            modern: modern.into(),
            historical: historical.into(),
        }
    }

    #[test]
    fn inference_rejects_a_split_without_a_strict_majority() {
        let samples = (0..3)
            .map(|_| inference_mapping("せい", "せぃ"))
            .chain((0..2).map(|_| inference_mapping("せい", "さい")))
            .collect::<Vec<_>>();
        assert!(build_historical_inference(&samples).is_empty());
    }

    #[test]
    fn inference_rejects_a_tie() {
        let samples = (0..5)
            .map(|_| inference_mapping("せい", "せぃ"))
            .chain((0..5).map(|_| inference_mapping("せい", "さい")))
            .collect::<Vec<_>>();
        assert!(build_historical_inference(&samples).is_empty());
    }
}
