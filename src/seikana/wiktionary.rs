use std::collections::BTreeMap;

use regex::Regex;

use super::entry::Entry;
use super::mapping::{to_index, Mapping};
use super::rules::{canonicalize_modern, normalize_historical, render_segment};
use super::segmentation::Segment;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JapanesePage {
    pub title: String,
    pub kanjitabs: Vec<Kanjitab>,
    pub wagokanji_readings: Vec<String>,
    pub noun_readings: Vec<String>,
    pub pron_readings: Vec<String>,
    pub verb_titles: Vec<String>,
    pub adjective_titles: Vec<String>,
    pub adverb_titles: Vec<String>,
    pub suru_readings: Vec<String>,
    pub old_japanese_titles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiktionaryEntry {
    pub reading: String,
    pub candidate: String,
    pub source: EntrySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySource {
    KanjiWord,
    WagoOkuri,
    Suru,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Kanjitab {
    /// Dictionary/base readings supplied as positional parameters.
    pub base_readings: Vec<String>,
    /// Actual readings after `kN` overrides such as rendaku and sokuon.
    pub readings: Vec<String>,
    pub yomi: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanjiWord {
    pub candidate: String,
    pub reading: String,
    pub segments: Vec<Segment>,
    pub compressed_after: Vec<usize>,
    pub historical: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KanjiWordError {
    NoReading,
    ReadingMismatch,
    MappingNotFound,
    RestoreFailed,
}

fn template_bodies<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let mut bodies = Vec::new();
    let marker = format!("{{{{{name}|");
    let mut cursor = 0;
    while let Some(start) = text[cursor..].find(&marker) {
        let start = cursor + start;
        let content_start = start + marker.len();
        let Some(content_end) = text[content_start..].find("}}") else {
            break;
        };
        bodies.push(&text[content_start..content_start + content_end]);
        cursor = content_start + content_end + 2;
    }
    bodies
}

fn split_params(body: &str) -> Vec<&str> {
    body.split('|').map(str::trim).collect()
}

fn clean_wikitext(value: &str) -> String {
    value
        .trim_matches(['[', ']', ' '])
        .replace("[[", "")
        .replace("]]", "")
}

fn hiragana_only(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ('ぁ'..='ゟ').contains(&ch))
}

pub fn is_kanji(ch: char) -> bool {
    matches!(ch, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}')
}

pub fn parse_japanese_page(title: &str, text: &str) -> JapanesePage {
    let kanjitab_regex =
        Regex::new(r"^[ぁ-ゖ0-9]+$").expect("internal error: invalid kanjitab regex");
    let mut page = JapanesePage {
        title: title.to_string(),
        ..JapanesePage::default()
    };

    for body in template_bodies(text, "ja-kanjitab") {
        let mut readings = Vec::new();
        let mut base_readings = Vec::new();
        let mut yomi = Vec::new();
        let mut overrides = BTreeMap::new();
        for param in split_params(body) {
            if let Some((name, value)) = param.split_once('=') {
                if name == "yomi" {
                    yomi = value.split(',').map(str::to_string).collect();
                } else if name.starts_with('k') && name[1..].chars().all(|ch| ch.is_ascii_digit()) {
                    overrides.insert(name.to_string(), clean_wikitext(value));
                }
                continue;
            }
            base_readings.push(clean_wikitext(param));
        }

        let title_len = title.chars().count();
        if base_readings.len() != title_len {
            continue;
        }
        for (index, base_reading) in base_readings.iter_mut().enumerate() {
            let raw = base_reading.trim_end_matches(|ch: char| ch.is_ascii_digit());
            *base_reading = raw.to_string();
            let reading = overrides
                .get(&format!("k{}", index + 1))
                .cloned()
                .unwrap_or_else(|| base_reading.clone());
            readings.push(reading);
        }
        let valid = base_readings
            .iter()
            .chain(readings.iter())
            .all(|reading| hiragana_only(reading) && kanjitab_regex.is_match(reading));
        if !valid {
            continue;
        }
        page.kanjitabs.push(Kanjitab {
            base_readings,
            readings,
            yomi,
        });
    }

    for body in template_bodies(text, "ja-wagokanji") {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let reading = clean_wikitext(param);
            if hiragana_only(&reading) && !page.wagokanji_readings.contains(&reading) {
                page.wagokanji_readings.push(reading);
            }
        }
    }
    if !page.wagokanji_readings.is_empty() {
        for reading in &page.wagokanji_readings {
            if !page.verb_titles.contains(reading) {
                page.verb_titles.push(reading.clone());
            }
        }
    }

    for body in template_bodies(text, "ja-noun") {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let reading = clean_wikitext(param);
            if hiragana_only(&reading) && !page.noun_readings.contains(&reading) {
                page.noun_readings.push(reading);
            }
        }
    }

    for body in template_bodies(text, "ja-pron") {
        for (index, param) in split_params(body).into_iter().enumerate() {
            if index > 0 || param.contains('=') {
                continue;
            }
            let reading = clean_wikitext(param);
            if hiragana_only(&reading) && !page.pron_readings.contains(&reading) {
                page.pron_readings.push(reading);
            }
        }
    }

    for body in template_bodies(text, "ja-verb") {
        for title in extract_bracket_titles(body) {
            if !page.verb_titles.contains(&title) {
                page.verb_titles.push(title);
            }
        }
    }
    if template_bodies(text, "ja-verb").is_empty() {
        for title in extract_bracket_titles(text) {
            if !page.verb_titles.contains(&title) {
                page.verb_titles.push(title);
            }
        }
    }
    for body in template_bodies(text, "ja-verb-suru") {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let reading = clean_wikitext(param);
            if hiragana_only(&reading) && !page.suru_readings.contains(&reading) {
                page.suru_readings.push(reading);
            }
        }
    }
    for body in template_bodies(text, "ja-noun-suru") {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let reading = clean_wikitext(param);
            if hiragana_only(&reading) && !page.suru_readings.contains(&reading) {
                page.suru_readings.push(reading);
            }
        }
    }

    for body in template_bodies(text, "ja-adjectival noun")
        .into_iter()
        .chain(template_bodies(text, "ja-adj"))
    {
        for title in extract_bracket_titles(body) {
            if !page.adjective_titles.contains(&title) {
                page.adjective_titles.push(title);
            }
        }
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let title = clean_wikitext(param);
            if !title.is_empty() && !page.adjective_titles.contains(&title) {
                page.adjective_titles.push(title);
            }
        }
    }

    if template_bodies(text, "ja-adj").is_empty() {
        for title in extract_bracket_titles(text) {
            if !page.adjective_titles.contains(&title) {
                page.adjective_titles.push(title);
            }
        }
    }

    for body in template_bodies(text, "ja-adjectival").into_iter() {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let title = clean_wikitext(param);
            if !title.is_empty() && !page.adjective_titles.contains(&title) {
                page.adjective_titles.push(title);
            }
        }
    }

    for body in template_bodies(text, "ja-adv") {
        for title in extract_bracket_titles(body) {
            if !page.adverb_titles.contains(&title) {
                page.adverb_titles.push(title);
            }
        }
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let value = clean_wikitext(param);
            if !value.is_empty() && !page.adverb_titles.contains(&value) {
                page.adverb_titles.push(value);
            }
        }
    }

    for name in [
        "ojp-verb",
        "ojp-noun",
        "ojp-adj",
        "ojp-adv",
        "ojp-wagokanji",
    ] {
        for body in template_bodies(text, name) {
            for title in extract_bracket_titles(body) {
                if !page.old_japanese_titles.contains(&title) {
                    page.old_japanese_titles.push(title);
                }
            }
        }
    }

    page
}

fn extract_bracket_titles(body: &str) -> Vec<String> {
    let mut titles = Vec::new();
    let mut cursor = 0;
    while let Some(start) = body[cursor..].find("【") {
        let start = cursor + start + "【".len();
        let Some(end) = body[start..].find("】") else {
            break;
        };
        let value = &body[start..start + end];
        for candidate in value.split(['、', '・']) {
            let candidate = candidate.trim_matches(['（', '）', ' ']);
            let candidate = clean_wikitext(candidate);
            if !candidate.is_empty() && !titles.contains(&candidate) {
                titles.push(candidate);
            }
        }
        cursor = start + end + "】".len();
    }
    titles
}

pub fn matched_readings(page: &JapanesePage) -> Vec<String> {
    let mut readings = page.noun_readings.clone();
    for reading in &page.pron_readings {
        if !readings.contains(reading) {
            readings.push(reading.clone());
        }
    }
    readings
}

pub fn kanji_words(page: &JapanesePage, mappings: &[Mapping]) -> Vec<KanjiWord> {
    let index = to_index(mappings);
    let whole_readings = matched_readings(page);
    let mut words = Vec::new();

    for kanjitab in &page.kanjitabs {
        let actual_reading = kanjitab.readings.concat();
        let base_reading = kanjitab.base_readings.concat();
        let matched_reading = whole_readings.iter().find(|candidate| {
            candidate.as_str() == actual_reading
                || candidate.as_str() == base_reading
                || canonicalize_modern(candidate) == canonicalize_modern(&actual_reading)
                || canonicalize_modern(candidate) == canonicalize_modern(&base_reading)
        });
        let Some(reading) = matched_reading.cloned() else {
            continue;
        };

        let title: Vec<char> = page.title.chars().collect();
        let mut segments = Vec::with_capacity(kanjitab.readings.len());
        let mut overridden_positions = Vec::new();
        let mut mapped = true;
        for (kanji, modern) in title.iter().zip(kanjitab.base_readings.iter()) {
            let Some(candidates) = index.get(&kanji.to_string()) else {
                mapped = false;
                break;
            };
            let Some(mapping) = candidates
                .iter()
                .find(|mapping| {
                    mapping.modern == *modern
                        || mapping.historical == *modern
                        || canonicalize_modern(mapping.modern.as_str())
                            == canonicalize_modern(modern.as_str())
                })
                .cloned()
            else {
                mapped = false;
                break;
            };
            segments.push(Segment {
                kanji: mapping.kanji,
                modern: mapping.modern,
                historical: mapping.historical,
            });
        }
        let mut source = normalize_historical(&actual_reading);
        let mut historical = String::new();
        let mut rendered = true;
        for (position, segment) in segments.iter().enumerate() {
            let actual = kanjitab.readings.get(position).cloned();
            let is_override = actual
                .as_deref()
                .is_some_and(|actual| actual != segment.modern);
            if is_override {
                overridden_positions.push(position);
            }
            let actual = actual.as_deref();
            let Some((rendered_historical, rest, _)) = render_segment(
                source.as_str(),
                &segment.modern,
                &segment.historical,
                actual,
            ) else {
                rendered = false;
                break;
            };
            source = rest.to_string();
            historical.push_str(&rendered_historical);
        }
        if mapped && rendered && source.is_empty() {
            words.push(KanjiWord {
                candidate: page.title.clone(),
                reading,
                segments,
                compressed_after: overridden_positions,
                historical,
            });
        }
    }

    words
}

pub fn convert_kanji_word(word: &KanjiWord) -> Option<Entry> {
    if word.historical.is_empty() {
        return None;
    }
    Some(Entry::new(word.historical.clone(), word.candidate.clone()))
}

pub fn kanji_word_entries(
    page: &JapanesePage,
    mappings: &[Mapping],
) -> (Vec<Entry>, Vec<(String, KanjiWordError)>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for word in kanji_words(page, mappings) {
        match convert_kanji_word(&word) {
            Some(entry) => entries.push(entry),
            None => errors.push((word.candidate.clone(), KanjiWordError::RestoreFailed)),
        }
    }
    if page.kanjitabs.is_empty() && matched_readings(page).is_empty() {
        errors.push((page.title.clone(), KanjiWordError::NoReading));
    } else if entries.is_empty() && errors.is_empty() {
        errors.push((page.title.clone(), KanjiWordError::MappingNotFound));
    }
    (entries, errors)
}

pub fn wiktionary_entries(page: &JapanesePage) -> Vec<WiktionaryEntry> {
    let mut entries = Vec::new();
    for candidate in &page.verb_titles {
        if let Some(entry) = okuri_candidate(page, candidate) {
            entries.push(entry);
        }
    }
    for candidate in &page.adjective_titles {
        if page.verb_titles.contains(candidate) {
            continue;
        }
        if let Some(entry) = okuri_candidate(page, candidate) {
            entries.push(entry);
        }
    }
    if let Some(entry) = okuri_candidate(page, page.title.as_str()) {
        entries.push(entry);
    }

    for reading in &page.suru_readings {
        entries.push(WiktionaryEntry {
            reading: reading.clone(),
            candidate: format!("{}/する", page.title),
            source: EntrySource::Suru,
        });
    }

    let mut unique_entries = Vec::new();
    for entry in entries {
        if !unique_entries.iter().any(|other: &WiktionaryEntry| {
            other.reading == entry.reading && other.candidate == entry.candidate
        }) {
            unique_entries.push(entry);
        }
    }
    unique_entries
}

fn okuri_candidate(page: &JapanesePage, candidate: &str) -> Option<WiktionaryEntry> {
    if !candidate.contains(is_kanji) || candidate.ends_with("する") {
        return None;
    }
    let (stem, suffix) = split_candidate(candidate);
    if stem.is_empty() || suffix.is_empty() {
        return None;
    }

    let (romaji, reading_suffix) = if suffix.chars().count() == 1 {
        let ch = suffix.chars().next()?;
        (okuri_romaji(ch)?, suffix)
    } else if candidate.ends_with('い') {
        (okuri_romaji(suffix.chars().next()?)?, suffix)
    } else {
        (
            okuri_romaji(suffix.chars().last()?)?,
            &suffix[suffix.char_indices().nth(1)?.0..],
        )
    };
    let stem_reading = page
        .wagokanji_readings
        .iter()
        .filter_map(|reading| reading.strip_suffix(reading_suffix))
        .max_by_key(|reading| reading.chars().count())?;

    let mut key = String::with_capacity(stem_reading.len() + 1);
    key.push_str(stem_reading);
    key.push(romaji);
    Some(WiktionaryEntry {
        reading: key,
        candidate: stem.to_string(),
        source: EntrySource::WagoOkuri,
    })
}

pub fn to_entry(entry: &WiktionaryEntry) -> Entry {
    Entry::new(entry.reading.as_str(), entry.candidate.as_str())
}

fn split_candidate(candidate: &str) -> (&str, &str) {
    let boundary = candidate
        .char_indices()
        .rev()
        .find(|(_, ch)| is_kanji(*ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(candidate.len());
    candidate.split_at(boundary)
}

fn okuri_romaji(ch: char) -> Option<char> {
    Some(match ch {
        'あ' => 'a',
        'い' => 'i',
        'う' => 'u',
        'え' => 'e',
        'お' => 'o',
        'か' | 'が' | 'き' | 'ぎ' | 'く' | 'ぐ' | 'け' | 'げ' | 'こ' | 'ご' => 'k',
        'さ' | 'ざ' | 'し' | 'じ' | 'す' | 'ず' | 'せ' | 'ぜ' | 'そ' | 'ぞ' => 's',
        'た' | 'だ' | 'ち' | 'ぢ' | 'つ' | 'づ' | 'て' | 'で' | 'と' | 'ど' => 't',
        'な' | 'に' | 'ぬ' | 'ね' | 'の' => 'n',
        'は' | 'ば' | 'ぱ' | 'ひ' | 'び' | 'ぴ' | 'ふ' | 'ぶ' | 'ぷ' | 'へ' | 'べ' | 'ぺ'
        | 'ほ' | 'ぼ' | 'ぽ' => 'h',
        'ま' | 'み' | 'む' | 'め' | 'も' => 'm',
        'や' | 'ゆ' | 'よ' => 'y',
        'ら' | 'り' | 'る' | 'れ' | 'ろ' => 'r',
        'わ' | 'ゐ' | 'ゑ' | 'を' => 'w',
        'ん' => 'n',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kanjitab_with_override() {
        let page = parse_japanese_page("学校", "{{ja-kanjitab|がく|k1=がっ|こう|yomi=o}}");
        assert_eq!(
            page.kanjitabs,
            vec![Kanjitab {
                base_readings: vec!["がく".into(), "こう".into()],
                readings: vec!["がっ".into(), "こう".into()],
                yomi: vec!["o".into()],
            }]
        );
    }

    #[test]
    fn parses_wagokanji() {
        let page = parse_japanese_page("歩く", "{{ja-wagokanji|あるく}}");
        assert_eq!(page.wagokanji_readings, vec!["あるく".to_string()]);
    }

    #[test]
    fn converts_wagokanji_title_without_verb_template() {
        let page = parse_japanese_page("歩く", "{{ja-wagokanji|あるく}}");
        assert_eq!(
            wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "あるk".into(),
                candidate: "歩".into(),
                source: EntrySource::WagoOkuri,
            }]
        );
    }

    #[test]
    fn parses_noun_link_reading() {
        let page = parse_japanese_page("方向", "{{ja-noun|[[ほうこう]]}}");
        assert_eq!(page.noun_readings, vec!["ほうこう".to_string()]);
    }

    #[test]
    fn ignores_named_arguments() {
        let page = parse_japanese_page("学校", "{{ja-noun|がっこう|kyu=學校}}");
        assert_eq!(page.noun_readings, vec!["がっこう".to_string()]);
    }

    #[test]
    fn rejects_kanjitab_with_wrong_arity() {
        let page = parse_japanese_page("学校", "{{ja-kanjitab|がっこう|yomi=o}}");
        assert!(page.kanjitabs.is_empty());
    }

    #[test]
    fn parses_verb_brackets() {
        let page = parse_japanese_page("まぜる", "{{ja-verb}}【[[混]]ぜる、[[交]]ぜる】");
        assert_eq!(
            page.verb_titles,
            vec!["混ぜる".to_string(), "交ぜる".to_string()]
        );
    }

    #[test]
    fn parses_suru_verb_reading() {
        let page = parse_japanese_page("移動", "{{ja-verb-suru|いどう}}");
        assert_eq!(page.suru_readings, vec!["いどう".to_string()]);
    }

    #[test]
    fn parses_adjective_titles() {
        let page = parse_japanese_page("大きい", "{{ja-adj}}【[[大]]きい】");
        assert_eq!(page.adjective_titles, vec!["大きい".to_string()]);

        let page = parse_japanese_page("温かい", "{{ja-adj|温かい|暖かい}}");
        assert_eq!(
            page.adjective_titles,
            vec!["温かい".to_string(), "暖かい".to_string()]
        );
    }

    #[test]
    fn splits_verb_candidate() {
        let (stem, suffix) = split_candidate("混ぜる");
        assert_eq!(stem, "混");
        assert_eq!(suffix, "ぜる");
    }

    #[test]
    fn converts_verb_title_to_okuri_entry() {
        let page = parse_japanese_page(
            "まぜる",
            "{{ja-wagokanji|まぜる}}{{ja-verb}}【[[混]]ぜる、[[交]]ぜる】",
        );
        let entries = wiktionary_entries(&page);
        assert_eq!(
            entries,
            vec![
                WiktionaryEntry {
                    reading: "まぜr".into(),
                    candidate: "混".into(),
                    source: EntrySource::WagoOkuri,
                },
                WiktionaryEntry {
                    reading: "まぜr".into(),
                    candidate: "交".into(),
                    source: EntrySource::WagoOkuri,
                }
            ]
        );
        assert_eq!(
            entries.iter().map(to_entry).collect::<Vec<_>>(),
            vec![Entry::new("まぜr", "混"), Entry::new("まぜr", "交")]
        );
    }

    #[test]
    fn converts_adjective_title_to_okuri_entry() {
        let page = parse_japanese_page(
            "あかるい",
            "{{ja-wagokanji|あかるい}}{{ja-adj}}【[[明]]るい】",
        );
        let entries = wiktionary_entries(&page);
        assert_eq!(
            entries.iter().map(to_entry).collect::<Vec<_>>(),
            vec![Entry::new("あかr", "明")]
        );
    }

    #[test]
    fn converts_suru_entry_with_source() {
        let page = parse_japanese_page("移動", "{{ja-verb-suru|いどう}}");
        let entries = wiktionary_entries(&page);
        assert_eq!(
            entries.iter().map(to_entry).collect::<Vec<_>>(),
            vec![Entry::new("いどう", "移動/する")]
        );
    }

    fn mappings() -> Vec<Mapping> {
        vec![
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
                kanji: "日".into(),
                modern: "にち".into(),
                historical: "にち".into(),
            },
            Mapping {
                kanji: "本".into(),
                modern: "ほん".into(),
                historical: "ほん".into(),
            },
            Mapping {
                kanji: "本".into(),
                modern: "ぽん".into(),
                historical: "ぽん".into(),
            },
            Mapping {
                kanji: "語".into(),
                modern: "ご".into(),
                historical: "ご".into(),
            },
        ]
    }

    #[test]
    fn converts_kanjitab_word() {
        let page = parse_japanese_page(
            "学校",
            "{{ja-kanjitab|がく|k1=がっ|こう|yomi=o}}{{ja-noun|がっこう}}",
        );
        let (entries, errors) = kanji_word_entries(&page, &mappings());
        assert!(errors.is_empty());
        assert_eq!(entries, vec![Entry::new("がっかう", "学校")]);
    }

    #[test]
    fn handles_rendaku_in_kanjitab() {
        let page = parse_japanese_page(
            "日本語",
            "{{ja-kanjitab|にち|k1=にっ|ほん|k2=ぽん|ご|yomi=goon,o,goon}}{{ja-noun|にほんご|にっぽんご}}",
        );
        let words = kanji_words(&page, &mappings());
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].reading, "にっぽんご");
        assert_eq!(
            convert_kanji_word(&words[0]),
            Some(Entry::new("にっぽんご", "日本語"))
        );
    }

    #[test]
    fn rejects_kanjitab_reading_mismatch() {
        let page = parse_japanese_page(
            "日本",
            "{{ja-kanjitab|にち|k1=にっ|ほん|k2=ぽん|yomi=o}}{{ja-noun|にほん}}",
        );
        let words = kanji_words(&page, &mappings());
        assert!(words.is_empty());
    }

    #[test]
    fn handles_sokuon_and_rendaku_together() {
        assert_eq!(
            render_segment("にっぽんご", "にち", "にち", Some("にっ")),
            Some(("にっ".into(), "ぽんご", false))
        );
        assert_eq!(
            render_segment("ぽんご", "ほん", "ほん", Some("ぽん")),
            Some(("ぽん".into(), "ご", false))
        );
    }
}
