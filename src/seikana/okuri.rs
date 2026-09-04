use super::entry::Entry;
use super::wiktionary::{is_kanji, JapanesePage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkuriError {
    NoReading,
    InvalidTitle,
    ReadingMismatch,
    InvalidOkurigana,
}

fn okurigana_romaji(ch: char) -> Option<char> {
    Some(match ch {
        'あ' => 'a',
        'い' => 'i',
        'う' => 'u',
        'え' => 'e',
        'お' => 'o',
        'か' | 'が' => 'k',
        'き' | 'ぎ' => 'k',
        'く' | 'ぐ' => 'k',
        'け' | 'げ' => 'k',
        'こ' | 'ご' => 'k',
        'さ' | 'ざ' => 's',
        'し' | 'じ' => 's',
        'す' | 'ず' => 's',
        'せ' | 'ぜ' => 's',
        'そ' | 'ぞ' => 's',
        'た' | 'だ' => 't',
        'ち' | 'ぢ' => 't',
        'つ' | 'づ' => 't',
        'て' | 'で' => 't',
        'と' | 'ど' => 't',
        'な' => 'n',
        'に' => 'n',
        'ぬ' => 'n',
        'ね' => 'n',
        'の' => 'n',
        'は' | 'ば' | 'ぱ' => 'h',
        'ひ' | 'び' | 'ぴ' => 'h',
        'ふ' | 'ぶ' | 'ぷ' => 'h',
        'へ' | 'べ' | 'ぺ' => 'h',
        'ほ' | 'ぼ' | 'ぽ' => 'h',
        'ま' => 'm',
        'み' => 'm',
        'む' => 'm',
        'め' => 'm',
        'も' => 'm',
        'や' => 'y',
        'ゆ' => 'y',
        'よ' => 'y',
        'ら' => 'r',
        'り' => 'r',
        'る' => 'r',
        'れ' => 'r',
        'ろ' => 'r',
        'わ' => 'w',
        'ゐ' => 'w',
        'ゑ' => 'w',
        'を' => 'w',
        'ん' => 'n',
        _ => return None,
    })
}

fn split_okurigana(title: &str) -> Option<(&str, &str)> {
    let mut boundary = None;
    for (index, ch) in title.char_indices() {
        if is_kanji(ch) {
            boundary = Some((index, index + ch.len_utf8()));
        } else if boundary.is_some() {
            break;
        }
    }
    let (_, boundary) = boundary?;
    let (kanji, okurigana) = (&title[..boundary], &title[boundary..]);
    if kanji.is_empty() || okurigana.is_empty() {
        return None;
    }
    if !kanji.chars().all(is_kanji)
        || !okurigana
            .chars()
            .all(|ch| !ch.is_ascii() && ('ぁ'..='ゖ').contains(&ch))
    {
        return None;
    }
    Some((kanji, okurigana))
}

pub fn okuri_entries(page: &JapanesePage) -> (Vec<Entry>, Vec<(String, OkuriError)>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    if page.wagokanji_readings.is_empty() {
        errors.push((page.title.clone(), OkuriError::NoReading));
        return (entries, errors);
    }

    let Some((kanji, okurigana)) = split_okurigana(&page.title) else {
        errors.push((page.title.clone(), OkuriError::InvalidTitle));
        return (entries, errors);
    };
    let Some(romaji) = okurigana.chars().next().and_then(okurigana_romaji) else {
        errors.push((page.title.clone(), OkuriError::InvalidOkurigana));
        return (entries, errors);
    };

    for reading in &page.wagokanji_readings {
        let Some(stem) = reading.strip_suffix(okurigana) else {
            errors.push((page.title.clone(), OkuriError::ReadingMismatch));
            continue;
        };
        let mut key = String::with_capacity(stem.len() + 1);
        key.push_str(stem);
        key.push(romaji);
        entries.push(Entry::new(key, kanji));
    }

    (entries, errors)
}

pub fn suru_entries(page: &JapanesePage) -> Vec<Entry> {
    page.suru_readings
        .iter()
        .map(|reading| Entry::new(reading.as_str(), format!("{}/する", page.title)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::wiktionary::parse_japanese_page;
    use super::*;

    #[test]
    fn converts_wago_with_okurigana() {
        let page = parse_japanese_page("歩く", "{{ja-wagokanji|あるく}}");
        let (entries, errors) = okuri_entries(&page);
        assert!(errors.is_empty());
        assert_eq!(entries, vec![Entry::new("あるk", "歩")]);
    }

    #[test]
    fn converts_adjective_with_multiple_okurigana() {
        let page = parse_japanese_page("明るい", "{{ja-wagokanji|あかるい}}");
        let (entries, errors) = okuri_entries(&page);
        assert!(errors.is_empty());
        assert_eq!(entries, vec![Entry::new("あかr", "明")]);
    }

    #[test]
    fn reports_reading_mismatch() {
        let page = parse_japanese_page("歩く", "{{ja-wagokanji|ある}}");
        let (entries, errors) = okuri_entries(&page);
        assert!(entries.is_empty());
        assert_eq!(errors, vec![("歩く".into(), OkuriError::ReadingMismatch)]);
    }

    #[test]
    fn converts_suru_verb() {
        let page = parse_japanese_page("移動", "{{ja-verb-suru|いどう}}");
        assert_eq!(suru_entries(&page), vec![Entry::new("いどう", "移動/する")]);
    }

    #[test]
    fn converts_suru_noun() {
        let page = parse_japanese_page("保護", "{{ja-noun-suru|ほご}}");
        assert_eq!(suru_entries(&page), vec![Entry::new("ほご", "保護/する")]);
    }
}
