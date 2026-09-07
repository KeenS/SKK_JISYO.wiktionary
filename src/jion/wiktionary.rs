use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use regex::Regex;

use super::entry::Entry;
use super::mapping::MappingIndex;
use super::rules::{canonicalize_modern, normalize_historical, render_segment};
use super::segmentation::Segment;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JapanesePage {
    pub title: String,
    pub default_sorts: Vec<String>,
    pub wagokanji_sources: Vec<WagokanjiSource>,
    pub kanjitabs: Vec<Kanjitab>,
    pub wagokanji_readings: Vec<String>,
    pub noun_candidates: Vec<String>,
    pub noun_readings: Vec<String>,
    pub pron_readings: Vec<String>,
    pub verb_titles: Vec<String>,
    pub adjective_titles: Vec<String>,
    pub adverb_titles: Vec<String>,
    pub suru_readings: Vec<String>,
    pub noun_suru_readings: Vec<String>,
    pub old_japanese_titles: Vec<String>,
    pub kangokana_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WagokanjiSource {
    pub candidate: String,
    pub reading: String,
}

fn bold_wagokanji_pairs(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for source in bold_wagokanji_sources(text) {
        if !pairs.contains(&source) {
            pairs.push(source);
        }
    }
    pairs
}

fn bold_wagokanji_sources(text: &str) -> impl Iterator<Item = (String, String)> + use<'_> {
    template_bodies(text, "lang|ja")
        .into_iter()
        .chain(text.lines())
        .filter_map(bold_wagokanji_pair)
}

fn bold_wagokanji_pair(body: &str) -> Option<(String, String)> {
    let end = body.find('）')?;
    let content = &body[..end + '）'.len_utf8()];
    let start = content.find("'''")? + 3;
    let bold_end = content[start..].find("'''")? + start;
    let candidate = clean_wikitext(&content[start..bold_end]);
    if !candidate.contains(is_kanji) {
        return None;
    }

    let reading_start = content[bold_end..].find('（')? + bold_end + '（'.len_utf8();
    let reading_end = content[bold_end..].find('）')? + bold_end;
    let reading = clean_wikitext(&content[reading_start..reading_end]);
    if !hiragana_only(&reading) {
        return None;
    }
    Some((candidate, reading))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiktionaryEntry {
    pub reading: String,
    pub candidate: String,
    pub suru: bool,
    pub source: EntrySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySource {
    KanjiWord,
    Idiom,
    Noun,
    WagoOkuri,
    Suru,
    SuruNoun,
    Kangokana,
}

fn default_sort_bodies(text: &str) -> Vec<String> {
    ["DEFAULTSORT", "kana-DEFAULTSORT"]
        .into_iter()
        .flat_map(|name| template_bodies(text, name))
        .map(str::to_string)
        .collect()
}

fn valid_reading(value: &str) -> bool {
    !value.is_empty() && value.chars().count() <= 32 && kana_only(value)
}

fn default_sort_readings(title: &str, body: &str) -> Vec<String> {
    if title.chars().all(|ch| ('ァ'..='ヶ').contains(&ch)) {
        return Vec::new();
    }
    let mut readings: Vec<String> = body
        .split_whitespace()
        .filter(|value| valid_reading(value))
        .map(str::to_string)
        .collect();
    if readings.len() > 1 {
        readings.remove(0);
    }
    readings
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
    let markers = [format!("{{{{{name}|"), format!("{{{{{name}:")];
    let mut cursor = 0;
    for marker in markers {
        while let Some(start) = text[cursor..].find(marker.as_str()) {
            let start = cursor + start;
            let content_start = start + marker.len();
            let Some(content_end) = text[content_start..].find("}}") else {
                break;
            };
            bodies.push(&text[content_start..content_start + content_end]);
            cursor = content_start + content_end + 2;
        }
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

fn kana_only(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ('ぁ'..='ゟ').contains(&ch) || ('ァ'..='ヶ').contains(&ch))
}

pub fn is_kanji(ch: char) -> bool {
    matches!(ch, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}')
}

static KANJITAB_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[ぁ-ゖ0-9]+$").expect("internal error: invalid kanjitab regex"));

static JAPANESE_CATEGORY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\[\[(?:category|カテゴリ):(?:\{\{ja\}\}|日本語|Japanese)(?:[_ ][^|\]]*)?\|([^|\]]+)\]\]",
    )
    .expect("internal error: invalid Japanese category regex")
});

fn japanese_category_sort_bodies(text: &str) -> Vec<String> {
    JAPANESE_CATEGORY_REGEX
        .captures_iter(text)
        .map(|captures| captures[1].trim().to_string())
        .collect()
}

fn category_sort_readings(title: &str, body: &str) -> Vec<String> {
    let mut title_chars = title.chars();
    if !(title_chars.next().is_some_and(is_kanji) && title_chars.next().is_none()) {
        return Vec::new();
    }
    let mut readings = Vec::new();
    for value in body.split_whitespace() {
        let reading = value.to_lowercase();
        if hiragana_only(&reading) && !readings.contains(&reading) {
            readings.push(reading);
        }
    }
    readings
}

pub fn parse_japanese_page(title: &str, text: &str) -> JapanesePage {
    let kanjitab_regex = &*KANJITAB_REGEX;
    let mut page = JapanesePage {
        title: title.to_string(),
        ..JapanesePage::default()
    };

    for body in default_sort_bodies(text) {
        for reading in default_sort_readings(title, &body) {
            if !page.default_sorts.contains(&reading) {
                page.default_sorts.push(reading);
            }
        }
    }
    for body in japanese_category_sort_bodies(text) {
        for reading in category_sort_readings(title, &body) {
            if !page.default_sorts.contains(&reading) {
                page.default_sorts.push(reading);
            }
        }
    }

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

    for (candidate, reading) in bold_wagokanji_pairs(text) {
        let source = WagokanjiSource { candidate, reading };
        if !page.wagokanji_sources.contains(&source) {
            page.wagokanji_sources.push(source);
        }
    }
    for reading in &page.wagokanji_readings {
        if !page.verb_titles.contains(reading) {
            page.verb_titles.push(reading.clone());
        }
    }

    for body in template_bodies(text, "ja-noun") {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let value = clean_wikitext(param);
            if value.chars().any(is_kanji) {
                if !value.is_empty() && !page.noun_candidates.contains(&value) {
                    page.noun_candidates.push(value);
                }
            } else if hiragana_only(&value) && !page.noun_readings.contains(&value) {
                page.noun_readings.push(value);
            }
        }
    }

    for body in template_bodies(text, "ja-idiom") {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            let reading = clean_wikitext(param);
            if valid_reading(&reading) && !page.noun_readings.contains(&reading) {
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
            if hiragana_only(&reading)
                && !page.suru_readings.contains(&reading)
                && !page.noun_suru_readings.contains(&reading)
            {
                page.noun_suru_readings.push(reading);
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

    for body in template_bodies(text, "ja-adjectival") {
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

    page.kangokana_candidates = kangokana_candidates(text);

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

fn kangokana_candidate_bodies(text: &str) -> Vec<&str> {
    let mut bodies = Vec::new();
    let marker = "{{ja-kangokana}}";
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(marker) {
        let start = cursor + offset + marker.len();
        let end = text[start..]
            .find("\n==")
            .map(|offset| start + offset)
            .unwrap_or(text.len());
        bodies.push(&text[start..end]);
        cursor = end;
    }
    bodies
}

fn kangokana_page_bodies(text: &str) -> Vec<&str> {
    let mut bodies = Vec::new();
    let marker = "{{ja-kangokana|";
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(marker) {
        let start = cursor + offset + marker.len();
        let Some(end) = text[start..].find("}}") else {
            break;
        };
        bodies.push(&text[start..start + end]);
        cursor = start + end + "}}".len();
    }
    bodies
}

fn kangokana_candidates(text: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut add_candidate = |candidate: String| {
        if candidate.chars().all(is_kanji) && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };
    for body in kangokana_candidate_bodies(text) {
        for title in extract_bracket_titles(body) {
            add_candidate(title);
        }
    }
    for body in kangokana_page_bodies(text) {
        for param in split_params(body) {
            if param.contains('=') {
                continue;
            }
            add_candidate(clean_wikitext(param));
        }
    }
    for body in template_bodies(text, "ja-k") {
        for param in split_params(body).into_iter() {
            if param.contains('=') {
                continue;
            }
            add_candidate(clean_wikitext(param));
        }
    }
    candidates
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

pub fn kanji_words(page: &JapanesePage, mappings: &MappingIndex) -> Vec<KanjiWord> {
    let index = mappings;
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
    mappings: &MappingIndex,
) -> (Vec<Entry>, Vec<Entry>, Vec<(String, KanjiWordError)>) {
    let mut entries = Vec::new();
    let mut jion_entries = Vec::new();
    let mut errors = Vec::new();
    for word in kanji_words(page, mappings) {
        entries.push(Entry::new(word.reading.clone(), word.candidate.clone()));
        if let Some(entry) = convert_kanji_word(&word) {
            jion_entries.push(entry);
        } else {
            errors.push((word.candidate.clone(), KanjiWordError::RestoreFailed));
        }
    }
    if page.kanjitabs.is_empty() && matched_readings(page).is_empty() {
        errors.push((page.title.clone(), KanjiWordError::NoReading));
    } else if entries.is_empty() && errors.is_empty() {
        errors.push((page.title.clone(), KanjiWordError::MappingNotFound));
    }
    (entries, jion_entries, errors)
}

pub fn wiktionary_entries(page: &JapanesePage) -> Vec<WiktionaryEntry> {
    let mut entries = Vec::new();
    if kana_only(&page.title) {
        for candidate in &page.kangokana_candidates {
            entries.push(WiktionaryEntry {
                reading: page.title.clone(),
                candidate: candidate.clone(),
                suru: false,
                source: EntrySource::Kangokana,
            });
        }
    }

    if page.title.chars().any(is_kanji) {
        for reading in page.default_sorts.iter().filter(|reading| {
            !page.wagokanji_readings.contains(reading)
                && !page
                    .wagokanji_sources
                    .iter()
                    .any(|source| &source.reading == *reading)
        }) {
            entries.push(WiktionaryEntry {
                reading: reading.clone(),
                candidate: page.title.clone(),
                suru: false,
                source: EntrySource::Idiom,
            });
        }
    }

    if kana_only(&page.title) {
        for candidate in &page.noun_candidates {
            if candidate.chars().any(is_kanji) {
                entries.push(WiktionaryEntry {
                    reading: page.title.clone(),
                    candidate: candidate.clone(),
                    suru: false,
                    source: EntrySource::Noun,
                });
            }
        }
    }

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
    for candidate in &page.adverb_titles {
        if page.verb_titles.contains(candidate) || page.adjective_titles.contains(candidate) {
            continue;
        }
        if let Some(entry) = okuri_candidate(page, candidate) {
            entries.push(entry);
        }
    }
    if let Some(entry) = okuri_candidate(page, page.title.as_str()) {
        entries.push(entry);
    }
    for source in &page.wagokanji_sources {
        if let Some(entry) = wago_source_entry(source) {
            entries.push(entry);
        }
    }
    if page.title.chars().any(is_kanji) && !page.title.ends_with("する") {
        for reading in &page.suru_readings {
            entries.push(WiktionaryEntry {
                reading: reading.clone(),
                candidate: page.title.clone(),
                suru: true,
                source: EntrySource::Suru,
            });
        }
        for reading in &page.noun_suru_readings {
            entries.push(WiktionaryEntry {
                reading: reading.clone(),
                candidate: page.title.clone(),
                suru: false,
                source: EntrySource::SuruNoun,
            });
        }
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

    let romaji = okuri_romaji(suffix.chars().next()?)?;
    let stem_reading = page
        .wagokanji_readings
        .iter()
        .filter_map(|reading| reading.strip_suffix(suffix))
        .max_by_key(|reading| reading.chars().count())?;

    let mut key = String::with_capacity(stem_reading.len() + 1);
    key.push_str(stem_reading);
    key.push(romaji);

    Some(WiktionaryEntry {
        reading: key,
        candidate: stem.to_string(),
        suru: false,
        source: EntrySource::WagoOkuri,
    })
}

fn wago_source_entry(source: &WagokanjiSource) -> Option<WiktionaryEntry> {
    let (stem, suffix) = split_candidate(&source.candidate);
    if stem.is_empty() || suffix.is_empty() {
        return None;
    }
    let stem_reading = source.reading.strip_suffix(&suffix)?;
    if stem_reading.chars().count() < stem.chars().count() {
        return None;
    }
    let romaji = okuri_romaji(suffix.chars().next()?)?;
    let mut key = String::with_capacity(stem_reading.len() + 1);
    key.push_str(stem_reading);
    key.push(romaji);
    Some(WiktionaryEntry {
        reading: key,
        candidate: stem.to_string(),
        suru: false,
        source: EntrySource::WagoOkuri,
    })
}

pub fn to_entry(entry: &WiktionaryEntry) -> Entry {
    Entry {
        reading: entry.reading.clone(),
        candidates: vec![entry.candidate.clone()],
        annotations: Vec::new(),
    }
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
        'か' | 'き' | 'く' | 'け' | 'こ' => 'k',
        'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご' => 'g',
        'さ' | 'し' | 'す' | 'せ' | 'そ' => 's',
        'ざ' | 'じ' | 'ず' | 'ぜ' | 'ぞ' => 'z',
        'た' | 'ち' | 'つ' | 'て' | 'と' => 't',
        'だ' | 'ぢ' | 'づ' | 'で' | 'ど' => 'd',
        'な' | 'に' | 'ぬ' | 'ね' | 'の' => 'n',
        'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' => 'h',
        'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ' => 'b',
        'ぱ' | 'ぴ' | 'ぷ' | 'ぺ' | 'ぽ' => 'p',
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
    use crate::jion::mapping::Mapping;

    #[test]
    fn parses_defaultsort_colon_form() {
        let page = parse_japanese_page("四面楚歌", "{{DEFAULTSORT:しめんそか}}");
        assert_eq!(page.default_sorts, vec!["しめんそか".to_string()]);
    }

    #[test]
    fn parses_multiple_defaultsort_readings() {
        let page = parse_japanese_page("古施錠古", "{{DEFAULTSORT:こしせいこ こじせいご}}");
        assert_eq!(page.default_sorts, vec!["こじせいご".to_string()]);
    }

    #[test]
    fn ignores_defaultsort_on_katakana_titles() {
        let page = parse_japanese_page("コスモス", "{{kana-DEFAULTSORT|オーストラリア}}");
        assert!(page.default_sorts.is_empty());
    }

    #[test]
    fn parses_japanese_category_sort_for_single_kanji() {
        let page = parse_japanese_page("仙", "[[Category:{{ja}}|せん]]");
        assert_eq!(page.default_sorts, vec!["せん".to_string()]);
        assert_eq!(
            wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "せん".into(),
                candidate: "仙".into(),
                suru: false,
                source: EntrySource::Idiom,
            }]
        );
    }

    #[test]
    fn parses_localized_japanese_category_sort() {
        let page = parse_japanese_page("藍", "[[カテゴリ:{{ja}} 色|あい]]");
        assert_eq!(page.default_sorts, vec!["あい".to_string()]);
    }

    #[test]
    fn ignores_category_sort_for_non_single_kanji_titles() {
        let page = parse_japanese_page("間隔", "[[Category:{{ja}}|かんかく]]");
        assert!(page.default_sorts.is_empty());
        let page = parse_japanese_page("歩く", "[[Category:{{ja}}|あるく]]");
        assert!(page.default_sorts.is_empty());
    }

    #[test]
    fn converts_idiom_page() {
        let page = parse_japanese_page(
            "四面楚歌",
            "{{DEFAULTSORT:しめんそか}}\n{{ja-idiom|しめんそか}}",
        );
        assert_eq!(
            wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "しめんそか".into(),
                candidate: "四面楚歌".into(),
                suru: false,
                source: EntrySource::Idiom,
            }]
        );
    }

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
    fn parses_kangokana_candidates() {
        let page = parse_japanese_page(
            "こうもん",
            "{{ja-kangokana}}\n*【[[公門]]】説明\n*【[[孔門]]】説明",
        );
        assert_eq!(page.kangokana_candidates, vec!["公門", "孔門"]);
        assert_eq!(
            wiktionary_entries(&page),
            vec![
                WiktionaryEntry {
                    reading: "こうもん".into(),
                    candidate: "公門".into(),
                    suru: false,
                    source: EntrySource::Kangokana,
                },
                WiktionaryEntry {
                    reading: "こうもん".into(),
                    candidate: "孔門".into(),
                    suru: false,
                    source: EntrySource::Kangokana,
                },
            ]
        );
    }

    #[test]
    fn parses_kangokana_page_template_candidate() {
        let page = parse_japanese_page("とりどく", "{{ja-kangokana|とりどく}}");
        assert!(page.kangokana_candidates.is_empty());
    }

    #[test]
    fn parses_ja_k_candidates() {
        let page = parse_japanese_page("しかく", "{{ja-kangokana}}\n{{ja-k|歯革|t=象牙}}");
        assert_eq!(page.kangokana_candidates, vec!["歯革"]);
    }

    #[test]
    fn converts_lang_ja_wagokanji_candidate() {
        let page = parse_japanese_page(
            "拐かす",
            "{{DEFAULTSORT:かとわかす かどわかす}}\n=={{ja}}==\n{{lang|ja|'''[[拐]]かす'''}}（かどわかす）",
        );
        assert_eq!(
            wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "かどわk".into(),
                candidate: "拐".into(),
                suru: false,
                source: EntrySource::WagoOkuri,
            }]
        );
    }

    #[test]
    fn converts_bold_wagokanji_with_inline_reading() {
        let page = parse_japanese_page(
            "忘れる",
            "{{DEFAULTSORT:わすれる}}\n=={{ja}}==\n'''[[忘]]れる'''（わすれる）",
        );
        assert_eq!(
            wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "わすr".into(),
                candidate: "忘".into(),
                suru: false,
                source: EntrySource::WagoOkuri,
            }]
        );
    }

    #[test]
    fn converts_repeated_kanji_wagokanji() {
        let page = parse_japanese_page(
            "若若しい",
            "{{kana-DEFAULTSORT|わかわかしい}}\n=={{ja}}==\n'''[[若]][[若]]しい'''（わかわかしい）",
        );
        assert_eq!(
            wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "わかわかs".into(),
                candidate: "若若".into(),
                suru: false,
                source: EntrySource::WagoOkuri,
            }]
        );
    }

    #[test]
    fn converts_wagokanji_title_without_verb_template() {
        let page = parse_japanese_page("歩く", "{{ja-wagokanji|あるく}}");
        assert_eq!(
            wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "あるk".into(),
                candidate: "歩".into(),
                suru: false,
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
    fn converts_adverb_title_to_okuri_entry() {
        let page = parse_japanese_page(
            "はやく",
            "{{ja-wagokanji|はやく}}{{ja-adv}}【[[早]]く、[[速]]く】",
        );
        let entries = wiktionary_entries(&page);
        assert_eq!(
            entries,
            vec![
                WiktionaryEntry {
                    reading: "はやk".into(),
                    candidate: "早".into(),
                    suru: false,
                    source: EntrySource::WagoOkuri,
                },
                WiktionaryEntry {
                    reading: "はやk".into(),
                    candidate: "速".into(),
                    suru: false,
                    source: EntrySource::WagoOkuri,
                },
            ]
        );
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
                    reading: "まz".into(),
                    candidate: "混".into(),
                    suru: false,
                    source: EntrySource::WagoOkuri,
                },
                WiktionaryEntry {
                    reading: "まz".into(),
                    candidate: "交".into(),
                    suru: false,
                    source: EntrySource::WagoOkuri,
                }
            ]
        );
        assert_eq!(
            entries.iter().map(to_entry).collect::<Vec<_>>(),
            vec![Entry::new("まz", "混"), Entry::new("まz", "交")]
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
            vec![Entry {
                reading: "いどう".into(),
                candidates: vec!["移動".into()],
                annotations: Vec::new(),
            }]
        );
    }

    #[test]
    fn converts_noun_suru_entry_without_suru_suffix() {
        let page = parse_japanese_page("咆哮", "{{ja-noun-suru|ほうこう}}");
        let entries = wiktionary_entries(&page);
        assert_eq!(
            entries.iter().map(to_entry).collect::<Vec<_>>(),
            vec![Entry::new("ほうこう", "咆哮")]
        );
    }

    #[test]
    fn keeps_verb_suru_entry_with_suru_suffix() {
        let page = parse_japanese_page("移動", "{{ja-noun-suru|いどう}}\n{{ja-verb-suru|いどう}}");
        let entries = wiktionary_entries(&page);
        assert_eq!(
            entries.iter().map(to_entry).collect::<Vec<_>>(),
            vec![Entry {
                reading: "いどう".into(),
                candidates: vec!["移動".into()],
                annotations: Vec::new(),
            }]
        );
    }

    fn mappings() -> MappingIndex {
        MappingIndex::new(&[
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
        ])
    }

    #[test]
    fn converts_kanjitab_word() {
        let page = parse_japanese_page(
            "学校",
            "{{ja-kanjitab|がく|k1=がっ|こう|yomi=o}}{{ja-noun|がっこう}}",
        );
        let (entries, jion_entries, errors) = kanji_word_entries(&page, &mappings());
        assert!(errors.is_empty());
        assert_eq!(entries, vec![Entry::new("がっこう", "学校")]);
        assert_eq!(jion_entries, vec![Entry::new("がっかう", "学校")]);
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
