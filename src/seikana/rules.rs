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

/// Make a modern reading comparable with the kanji readings in the mapping.
///
/// The mapping stores the historical stem of each reading, while actual
/// dictionary readings often contain a small tsu. For example, 学校 is written
/// as "がっこう" but its readings are "がく" and "こう".
pub fn canonicalize_modern(reading: &str) -> String {
    let mut result = String::with_capacity(reading.len());
    let mut chars = reading.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != 'っ' {
            result.push(ch);
            continue;
        }

        let Some(next) = chars.peek().copied() else {
            result.push(ch);
            continue;
        };
        let expanded = match next {
            'こ' | 'ご' => 'く',
            'か' | 'が' | 'く' | 'ぐ' | 'き' | 'ぎ' => 'く',
            'さ' | 'ざ' | 'し' | 'じ' | 'す' | 'ず' | 'せ' | 'ぜ' | 'そ' | 'ぞ' | 'た' | 'だ'
            | 'ち' | 'ぢ' | 'つ' | 'づ' | 'て' | 'で' | 'と' | 'ど' | 'な' | 'に' | 'ぬ' | 'ね'
            | 'の' | 'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' | 'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ' | 'ぱ'
            | 'ぴ' | 'ぷ' | 'ぺ' | 'ぽ' | 'ま' | 'み' | 'む' | 'め' | 'も' | 'や' | 'ゆ' | 'よ'
            | 'ら' | 'り' | 'る' | 'れ' | 'ろ' => 'つ',
            _ => {
                result.push(ch);
                continue;
            }
        };
        result.push(expanded);
    }
    result
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
            render_segment(&source, &segment.modern, &segment.historical)
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

fn render_segment<'a>(
    source: &'a str,
    modern: &str,
    historical: &str,
) -> Option<(String, &'a str, bool)> {
    let modern = &normalize_historical(modern);
    let historical = &normalize_historical(historical);
    if let Some(consumed) = source.strip_prefix(modern.as_str()) {
        return Some((historical.to_string(), consumed, false));
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
    use crate::seikana::mapping::Mapping;
    use crate::seikana::segmentation::{segment, Segmentation};

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
        match segment(candidate, &canonicalize_modern(reading), mappings) {
            Segmentation::Unique(segments) => segments,
            result => panic!("expected unique segmentation, got {result:?}"),
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
    fn canonicalizes_sokuon() {
        assert_eq!(canonicalize_modern("がっこう"), "がくこう");
        assert_eq!(canonicalize_modern("いっさい"), "いつさい");
        assert_eq!(canonicalize_modern("あっ"), "あっ");
    }
}
