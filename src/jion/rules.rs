use super::segmentation::Segment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversion {
    pub candidate: String,
    pub reading: String,
    pub historical: String,
    pub segments: Vec<Segment>,
    pub compressed_after: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleError {
    EmptyInput,
    UnrestorableSokuon,
}

fn sokuon_sources(next: char) -> &'static [char] {
    match next {
        'か' | 'き' | 'く' | 'け' | 'こ' | 'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご' => {
            &['く', 'つ', 'ち']
        }
        'さ' | 'ざ' | 'し' | 'じ' | 'す' | 'ず' | 'せ' | 'ぜ' | 'そ' | 'ぞ' | 'た' | 'だ'
        | 'ち' | 'ぢ' | 'つ' | 'づ' | 'て' | 'で' | 'と' | 'ど' | 'な' | 'に' | 'ぬ' | 'ね'
        | 'の' | 'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' | 'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ' | 'ぱ'
        | 'ぴ' | 'ぷ' | 'ぺ' | 'ぽ' | 'ま' | 'み' | 'む' | 'め' | 'も' | 'や' | 'ゆ' | 'よ'
        | 'ら' | 'り' | 'る' | 'れ' | 'ろ' => &['つ', 'ち'],
        _ => &[],
    }
}

/// Readings produced by expanding each 促音 to a mora that can geminate.
///
/// 学校 is written "がっこう" while the mapping stores "がく" and "こう".
/// 日記 is "にっき" from "にち" and "き", and 結婚 is "けっこん" from "けつ".
/// Each っ therefore expands to every legal source mora. A final っ stays っ.
pub fn sokuon_expansions(reading: &str) -> Vec<String> {
    fn expand(chars: &[char], index: usize, current: &mut String, out: &mut Vec<String>) {
        if index == chars.len() {
            out.push(current.clone());
            return;
        }
        let ch = chars[index];
        if ch == 'っ' {
            if let Some(&next) = chars.get(index + 1) {
                let sources = sokuon_sources(next);
                if !sources.is_empty() {
                    for source in sources {
                        current.push(*source);
                        expand(chars, index + 1, current, out);
                        current.pop();
                    }
                    return;
                }
            }
        }
        current.push(ch);
        expand(chars, index + 1, current, out);
        current.pop();
    }

    let chars: Vec<char> = reading.chars().collect();
    let mut out = Vec::new();
    expand(&chars, 0, &mut String::new(), &mut out);
    out
}

/// True when the readings are equal or share an expanded 促音 spelling.
pub fn sokuon_equivalent(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let right = sokuon_expansions(right);
    sokuon_expansions(left)
        .iter()
        .any(|item| right.iter().any(|other| item == other))
}

/// Expand small yōon kana to their historical full-kana spelling.
pub fn normalize_historical(reading: &str) -> String {
    reading
        .replace('ぅ', "う")
        .replace('ゃ', "や")
        .replace('ゅ', "ゆ")
        .replace('ょ', "よ")
        .replace('ァ', "ア")
        .replace('ィ', "イ")
        .replace('ゥ', "ウ")
        .replace('ェ', "エ")
        .replace('ォ', "オ")
}

pub fn convert(
    candidate: &str,
    reading: &str,
    segments: &[Segment],
) -> Result<Conversion, RuleError> {
    if segments.is_empty() || reading.is_empty() {
        return Err(RuleError::EmptyInput);
    }

    let mut historical = String::new();
    let mut compressed_after = Vec::new();
    let mut source = normalize_historical(reading);
    for (index, segment) in segments.iter().enumerate() {
        let Some((rendered, rest, compressed)) =
            render_segment(&source, &segment.modern, &segment.historical, None)
        else {
            return Err(RuleError::UnrestorableSokuon);
        };
        if compressed {
            compressed_after.push(index);
        }
        historical.push_str(&rendered);
        source = rest.to_string();
    }

    if !source.is_empty() {
        return Err(RuleError::UnrestorableSokuon);
    }

    Ok(Conversion {
        candidate: candidate.to_string(),
        reading: reading.to_string(),
        historical: normalize_historical(&historical),
        segments: segments.to_vec(),
        compressed_after,
    })
}

pub(crate) fn render_segment<'a>(
    source: &'a str,
    modern: &str,
    historical: &str,
    actual: Option<&str>,
) -> Option<(String, &'a str, bool)> {
    let modern = &normalize_historical(modern);
    let actual = actual.map(normalize_historical).unwrap_or_default();
    let historical = &normalize_historical(historical);
    let actual = if actual.is_empty() {
        modern.clone()
    } else {
        actual
    };
    if actual.as_str() != modern.as_str() {
        let rest = source.strip_prefix(actual.as_str())?;
        return Some((normalize_historical(actual.as_str()), rest, false));
    }
    if let Some(consumed) = source.strip_prefix(actual.as_str()) {
        let historical = if actual.ends_with('っ') {
            let historical_head = historical
                .chars()
                .take(historical.chars().count().saturating_sub(1))
                .collect::<String>();
            format!("{historical_head}っ")
        } else {
            historical.to_string()
        };
        return Some((historical, consumed, false));
    }
    let modern_chars = modern.chars().collect::<Vec<_>>();
    if modern_chars.is_empty() {
        return None;
    }
    let compressed = format!(
        "{}っ",
        modern_chars[..modern_chars.len() - 1]
            .iter()
            .collect::<String>()
    );
    let consumed = source.strip_prefix(compressed.as_str())?;

    let historical_chars = historical.chars().collect::<Vec<_>>();
    if historical_chars.is_empty() {
        return None;
    }
    let historical_head = &historical_chars[..historical_chars.len() - 1];
    let rendered = format!("{}っ", historical_head.iter().collect::<String>());
    Some((rendered, consumed, true))
}

pub fn restore_modern(conversion: &Conversion) -> Option<String> {
    let mut modern = String::new();
    let mut compressed = conversion.compressed_after.iter().copied();
    let mut next_compressed = compressed.next();

    for (index, segment) in conversion.segments.iter().enumerate() {
        if next_compressed == Some(index) {
            let modern_chars = segment.modern.chars().collect::<Vec<_>>();
            if modern_chars.is_empty() {
                return None;
            }
            let head = &modern_chars[..modern_chars.len() - 1];
            modern.push_str(&head.iter().collect::<String>());
            modern.push('っ');
            next_compressed = compressed.next();
        } else {
            modern.push_str(&segment.modern);
        }
    }

    (modern == conversion.reading).then_some(modern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jion::mapping::Mapping;
    use crate::jion::segmentation::{segment, Segmentation};

    fn mappings() -> Vec<Mapping> {
        vec![
            Mapping {
                kanji: "方".into(),
                modern: "ほう".into(),
                historical: "はう".into(),
            },
            Mapping {
                kanji: "向".into(),
                modern: "こう".into(),
                historical: "かう".into(),
            },
            Mapping {
                kanji: "学".into(),
                modern: "がく".into(),
                historical: "がく".into(),
            },
            Mapping {
                kanji: "校".into(),
                modern: "こう".into(),
                historical: "かう".into(),
            },
            Mapping {
                kanji: "社".into(),
                modern: "しゃ".into(),
                historical: "しや".into(),
            },
            Mapping {
                kanji: "会".into(),
                modern: "かい".into(),
                historical: "くわい".into(),
            },
        ]
    }

    fn unique_segments(candidate: &str, reading: &str, mappings: &[Mapping]) -> Vec<Segment> {
        let mut found = Vec::new();
        for expansion in sokuon_expansions(reading) {
            if let Segmentation::Unique(segments) = segment(candidate, &expansion, mappings) {
                let key: Vec<_> = segments
                    .iter()
                    .map(|segment| (segment.kanji.clone(), segment.modern.clone()))
                    .collect();
                if !found
                    .iter()
                    .any(|(existing, _): &(Vec<_>, Vec<Segment>)| existing == &key)
                {
                    found.push((key, segments));
                }
            }
        }
        match found.len() {
            1 => found.remove(0).1,
            _ => panic!("expected unique segmentation, got {found:?}"),
        }
    }

    #[test]
    fn converts_basic_jukugo() {
        let mappings = mappings();
        let segments = unique_segments("方向", "ほうこう", &mappings);
        let conversion = convert("方向", "ほうこう", &segments).unwrap();
        assert_eq!(conversion.historical, "はうかう");
        assert_eq!(restore_modern(&conversion).as_deref(), Some("ほうこう"));
    }

    #[test]
    fn converts_sokuon_jukugo() {
        let mappings = mappings();
        let segments = unique_segments("学校", "がっこう", &mappings);
        let conversion = convert("学校", "がっこう", &segments).unwrap();
        assert_eq!(conversion.historical, "がっかう");
        assert_eq!(restore_modern(&conversion).as_deref(), Some("がっこう"));
    }

    #[test]
    fn converts_yoon_jukugo() {
        let mappings = mappings();
        let segments = unique_segments("社会", "しゃかい", &mappings);
        let conversion = convert("社会", "しゃかい", &segments).unwrap();
        assert_eq!(conversion.historical, "しやくわい");
        assert_eq!(restore_modern(&conversion).as_deref(), Some("しゃかい"));
    }

    #[test]
    fn normalizes_small_yoon() {
        assert_eq!(normalize_historical("しゃう"), "しやう");
        assert_eq!(normalize_historical("しゆ"), "しゆ");
    }

    #[test]
    fn expands_sokuon_to_every_legal_mora() {
        let gakkou = sokuon_expansions("がっこう");
        assert!(gakkou.contains(&"がくこう".to_string()));
        let nikki = sokuon_expansions("にっき");
        assert!(nikki.contains(&"にちき".to_string()));
        let jikken = sokuon_expansions("じっけん");
        assert!(jikken.contains(&"じつけん".to_string()));
        let issai = sokuon_expansions("いっさい");
        assert!(issai.contains(&"いつさい".to_string()));
        assert!(issai.contains(&"いちさい".to_string()));
        assert_eq!(sokuon_expansions("あっ"), vec!["あっ".to_string()]);
    }
}
