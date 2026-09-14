use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use regex::Regex;

use super::entry::Entry;
use super::mapping::MappingIndex;
use super::on_reading::{
    kanji_template_params, parse_common_readings, parse_dictionary_on_readings,
};
use super::rules::{canonicalize_modern, normalize_historical, render_segment};
use super::segmentation::Segment;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JapanesePage {
    pub title: String,
    pub furigana_readings: Vec<String>,
    pub default_sorts: Vec<String>,
    pub kanji_template_readings: Vec<String>,
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
    pub old_japanese_conjugations: Vec<OldJapaneseConjugation>,
    pub kangokana_candidates: Vec<String>,
    pub new_style_variants: Vec<String>,
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
pub struct OldJapaneseConjugation {
    pub stem: String,
    pub suffix: String,
    pub conjugation: OldJapaneseConjugationType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldJapaneseConjugationType {
    KamiIchidan,
    KamiNidan,
    ShimoIchidan,
    ShimoNidan,
    Shodan,
    Irregular,
    Nari,
    Tari,
    Ku,
    Shiku,
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
    Symbol,
    KanjiWord,
    KanjiReading,
    Idiom,
    Noun,
    Redirect,
    WagoOkuri,
    Suru,
    SuruNoun,
    Kangokana,
    OldJapanese,
}

/// Extract the destination of a MediaWiki redirect.
///
/// The Japanese Wiktionary uses both ASCII and full-width `#`, and the
/// keyword is written in Latin characters or as `転送`.
pub fn redirect_target(text: &str) -> Option<&str> {
    let mut rest = text.trim_start();
    if rest.starts_with('＃') {
        rest = &rest['＃'.len_utf8()..];
    } else if rest.starts_with('#') {
        rest = &rest['#'.len_utf8()..];
    } else {
        return None;
    }

    let keyword_end = rest
        .find(|ch: char| ch == '[' || ch.is_whitespace())
        .unwrap_or(rest.len());
    let keyword = rest[..keyword_end].trim_end();
    if !eq_ignore_case(keyword, "redirect") && keyword != "転送" {
        return None;
    }

    let start = rest.find("[[")?;
    let start = start + 2;
    let end = rest[start..].find("]]")? + start;
    let target = &rest[start..end];
    let target = target.split('#').next().unwrap_or(target);
    let target = target.trim();
    if target.is_empty() {
        None
    } else {
        Some(target)
    }
}

fn eq_ignore_case(value: &str, expected: &str) -> bool {
    value.len() == expected.len()
        && value
            .chars()
            .zip(expected.chars())
            .all(|(a, b)| a.eq_ignore_ascii_case(&b))
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

fn template_bodies_after<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let marker = format!("{{{{{name}}}}}");
    text.match_indices(&marker)
        .map(|(index, _)| index + marker.len())
        .map(|start| &text[start..])
        .collect()
}

fn split_params(body: &str) -> Vec<&str> {
    body.split('|').map(str::trim).collect()
}

fn furigana_pair(body: &str) -> Option<(String, String)> {
    let mut params = split_params(body);
    let reading = params.pop()?;
    let reading = clean_wikitext(reading);
    if !hiragana_only(&reading) {
        return None;
    }
    let word = params.last().map(|word| clean_wikitext(word))?;
    Some((word, reading))
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

pub fn is_symbol(title: &str) -> bool {
    title
        .chars()
        .all(|ch| !ch.is_alphanumeric() && !is_kanji(ch))
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

fn kanji_template_readings(text: &str) -> Vec<String> {
    let Some(params) = kanji_template_params(text) else {
        return Vec::new();
    };
    let mut readings = parse_common_readings(params);
    for reading in parse_dictionary_on_readings(params) {
        if !readings.contains(&reading) {
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

    for body in template_bodies(text, "ふりがな") {
        if let Some((_, reading)) = furigana_pair(body) {
            if hiragana_only(&reading) && !page.furigana_readings.contains(&reading) {
                page.furigana_readings.push(reading);
            }
        }
    }

    for body in default_sort_bodies(text) {
        for reading in default_sort_readings(title, &body) {
            if !page.default_sorts.contains(&reading) {
                page.default_sorts.push(reading);
            }
        }
    }
    for reading in kanji_template_readings(text) {
        if !page.kanji_template_readings.contains(&reading) {
            page.kanji_template_readings.push(reading);
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

    for body in template_bodies(text, "ja-verb")
        .into_iter()
        .chain(template_bodies_after(text, "ja-verb"))
    {
        for title in extract_bracket_titles(body) {
            if !page.verb_titles.contains(&title) {
                page.verb_titles.push(title);
            }
        }
    }
    for reading in &page.wagokanji_readings {
        if reading.ends_with(['る', 'つ', 'む', 'ぬ', 'ぶ', 'く', 'ぐ', 'す'])
            && !page.verb_titles.contains(reading)
            && !page.adjective_titles.contains(reading)
            && !page.adverb_titles.contains(reading)
        {
            page.verb_titles.push(reading.clone());
        }
    }
    if template_bodies(text, "ja-verb").is_empty()
        && template_bodies_after(text, "ja-verb").is_empty()
    {
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

    if template_bodies(text, "ja-adj").is_empty()
        && head_candidate_bodies(text, "形容動詞").is_empty()
    {
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

    for body in template_bodies_after(text, "ja-adv") {
        for title in extract_bracket_titles(body) {
            if !page.adverb_titles.contains(&title) {
                page.adverb_titles.push(title);
            }
        }
    }

    if !head_candidate_bodies(text, "形容動詞").is_empty() {
        for reading in matched_readings(&page) {
            if !page.noun_readings.contains(&reading) {
                page.noun_readings.push(reading);
            }
        }
    }
    for body in head_candidate_bodies(text, "形容動詞") {
        for title in extract_bracket_titles(body) {
            if !page.adjective_titles.contains(&title) {
                page.adjective_titles.push(title);
            }
        }
    }
    for body in head_candidate_bodies(text, "adverb") {
        for title in extract_bracket_titles(body) {
            if !page.adverb_titles.contains(&title) {
                page.adverb_titles.push(title);
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
            for param in split_params(body) {
                let param = param.split('#').next().unwrap_or(param);
                let param = clean_wikitext(param);
                if param.contains(is_kanji) && !page.old_japanese_titles.contains(&param) {
                    page.old_japanese_titles.push(param);
                }
            }
        }
        for title in extract_bracket_titles(text) {
            if !page.old_japanese_titles.contains(&title) {
                page.old_japanese_titles.push(title);
            }
        }
    }

    for (name, conjugation) in [
        (
            "古典日本語上一段活用",
            OldJapaneseConjugationType::KamiIchidan,
        ),
        (
            "古典日本語上二段活用",
            OldJapaneseConjugationType::KamiNidan,
        ),
        (
            "古典日本語下一段活用",
            OldJapaneseConjugationType::ShimoIchidan,
        ),
        (
            "古典日本語下二段活用",
            OldJapaneseConjugationType::ShimoNidan,
        ),
        ("古典日本語四段活用", OldJapaneseConjugationType::Shodan),
        ("古典日本語変格活用", OldJapaneseConjugationType::Irregular),
        ("古典日本語ナリ活用", OldJapaneseConjugationType::Nari),
        ("古典日本語タリ活用", OldJapaneseConjugationType::Tari),
        ("古典日本語ク活用", OldJapaneseConjugationType::Ku),
        ("古典日本語シク活用", OldJapaneseConjugationType::Shiku),
    ] {
        for body in template_bodies(text, name) {
            let params = split_params(body);
            if params.is_empty() {
                continue;
            }
            let stem = clean_wikitext(params[0]);
            let suffix = params
                .get(1)
                .map(|value| clean_wikitext(value))
                .unwrap_or_default();
            if !hiragana_only(&stem) || (!suffix.is_empty() && !hiragana_only(&suffix)) {
                continue;
            }
            let conjugation = OldJapaneseConjugation {
                stem,
                suffix,
                conjugation,
            };
            if !page.old_japanese_conjugations.contains(&conjugation) {
                page.old_japanese_conjugations.push(conjugation);
            }
        }
    }

    page.kangokana_candidates = kangokana_candidates(text);
    page.new_style_variants = new_style_variants(text);

    page
}

fn head_candidate_bodies<'a>(text: &'a str, part_of_speech: &str) -> Vec<&'a str> {
    let mut bodies = Vec::new();
    let marker = "{{head|ja|";
    let mut cursor = 0;
    while let Some(start) = text[cursor..].find(marker) {
        let content_start = cursor + start + marker.len();
        let Some(relative_end) = text[content_start..].find("}}") else {
            break;
        };
        let end = content_start + relative_end;
        let Some(candidate_end) = text[end..].find("】") else {
            cursor = end + "}}".len();
            continue;
        };
        let candidate_end = end + candidate_end + "】".len();
        let body = &text[content_start..candidate_end];
        if body.contains(part_of_speech) {
            bodies.push(body);
        }
        cursor = candidate_end;
    }
    bodies
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

fn new_style_variant_bodies(text: &str) -> Vec<&str> {
    let mut bodies = Vec::new();
    let marker = "{{kanji variants|";
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

fn new_style_variants(text: &str) -> Vec<String> {
    let mut variants = Vec::new();
    for body in new_style_variant_bodies(text) {
        for param in split_params(body) {
            let Some((candidate, labels)) = param.split_once('=') else {
                continue;
            };
            if labels.contains("新字体") {
                let candidate = clean_wikitext(candidate);
                if candidate.chars().all(is_kanji) && !variants.contains(&candidate) {
                    variants.push(candidate);
                }
            }
        }
    }
    variants
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
    if is_symbol(&page.title) {
        for reading in &page.furigana_readings {
            entries.push(WiktionaryEntry {
                reading: reading.clone(),
                candidate: page.title.clone(),
                suru: false,
                source: EntrySource::Symbol,
            });
        }
    }
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
                    .filter(|source| source.reading == **reading)
                    .filter_map(wago_source_entry)
                    .any(|entry| starts_with_okuri(&entry.reading, reading))
        }) {
            entries.push(WiktionaryEntry {
                reading: reading.clone(),
                candidate: page.title.clone(),
                suru: false,
                source: EntrySource::Idiom,
            });
        }
        for reading in &page.kanji_template_readings {
            entries.push(WiktionaryEntry {
                reading: reading.clone(),
                candidate: page.title.clone(),
                suru: false,
                source: EntrySource::KanjiReading,
            });
        }
    }

    if kana_only(&page.title) {
        let adjective_kanji_words: Vec<_> = page
            .adjective_titles
            .iter()
            .filter(|candidate| candidate.chars().all(is_kanji))
            .cloned()
            .collect();
        let mut candidates = page.noun_candidates.clone();
        candidates.extend(adjective_kanji_words);
        for candidate in candidates {
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
        let is_non_verb_okuri = page
            .adjective_titles
            .iter()
            .chain(page.adverb_titles.iter())
            .any(|title| title.as_str() == candidate);
        let is_kana_reading = candidate.as_str() == page.title || !candidate.contains(is_kanji);
        let generate_onbin = !is_non_verb_okuri
            && (candidate.contains(is_kanji)
                || page
                    .verb_titles
                    .iter()
                    .all(|title| !title.contains(is_kanji)))
            || is_kana_reading && page.verb_titles.len() == 1 && !page.title.chars().any(is_kanji);
        entries.extend(okuri_candidates(page, candidate, generate_onbin));
    }
    for candidate in &page.adjective_titles {
        if page.verb_titles.contains(candidate) {
            continue;
        }
        entries.extend(okuri_candidates(page, candidate, false));
    }
    for candidate in &page.adverb_titles {
        if page.verb_titles.contains(candidate) || page.adjective_titles.contains(candidate) {
            continue;
        }
        if let Some(entry) = okuri_candidate(page, candidate) {
            entries.push(entry);
        }
    }
    entries.extend(okuri_candidates(page, page.title.as_str(), true));
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

pub fn old_japanese_entries(page: &JapanesePage) -> Vec<WiktionaryEntry> {
    let mut entries = Vec::new();
    for conjugation in &page.old_japanese_conjugations {
        let dictionary_reading = format!("{}{}", conjugation.stem, conjugation.suffix);
        let key = match conjugation.conjugation {
            OldJapaneseConjugationType::Nari
            | OldJapaneseConjugationType::Tari
            | OldJapaneseConjugationType::Ku
            | OldJapaneseConjugationType::Shiku => dictionary_reading,
            _ => {
                let Some(key) = old_japanese_okuri_key(conjugation) else {
                    continue;
                };
                key
            }
        };
        for candidate in old_japanese_candidates(page) {
            entries.push(WiktionaryEntry {
                reading: key.clone(),
                candidate,
                suru: false,
                source: EntrySource::OldJapanese,
            });
        }
    }
    entries
}

fn old_japanese_okuri_key(conjugation: &OldJapaneseConjugation) -> Option<String> {
    let final_kana = conjugation.suffix.chars().last()?;
    let romaji = okuri_romaji(final_kana)?;
    let mut key = conjugation.stem.clone();
    key.push(romaji);
    Some(key)
}

fn old_japanese_candidates(page: &JapanesePage) -> Vec<String> {
    let mut candidates = Vec::new();
    for title in &page.old_japanese_titles {
        if !title.contains(is_kanji) {
            continue;
        }
        let candidate = if let Some((stem, _)) = old_japanese_split_title(title) {
            stem.to_string()
        } else {
            title.to_string()
        };
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    if candidates.is_empty() && page.title.contains(is_kanji) {
        let title = page.title.clone();
        if let Some((stem, _)) = old_japanese_split_title(&title) {
            candidates.push(stem.to_string());
        } else {
            candidates.push(title);
        }
    }
    candidates
}

fn old_japanese_split_title(title: &str) -> Option<(&str, &str)> {
    let boundary = title
        .char_indices()
        .rev()
        .find(|(_, ch)| is_kanji(*ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(title.len());
    let (stem, suffix) = title.split_at(boundary);
    if suffix.is_empty() {
        None
    } else {
        Some((stem, suffix))
    }
}

fn okuri_candidates(
    page: &JapanesePage,
    candidate: &str,
    generate_onbin: bool,
) -> Vec<WiktionaryEntry> {
    let mut entries = Vec::new();
    if let Some(entry) = okuri_candidate(page, candidate) {
        if generate_onbin {
            entries.extend(okuri_onbin_entries(&entry));
        }
        entries.push(entry);
    }
    entries
}

fn okuri_onbin_entries(dictionary_entry: &WiktionaryEntry) -> Vec<WiktionaryEntry> {
    let Some((stem, dictionary_key_consonant)) = split_onbin_stem(&dictionary_entry.reading) else {
        return Vec::new();
    };
    let suffix = match dictionary_key_consonant {
        'r' | 'u' => "t",
        't' => "tt",
        'm' | 'n' | 'b' => "n",
        'k' | 'g' => {
            if is_iku_verb(&dictionary_entry.candidate) {
                "t"
            } else {
                "i"
            }
        }
        _ => return Vec::new(),
    };
    let mut key = String::with_capacity(stem.len() + suffix.len());
    key.push_str(stem);
    key.push_str(suffix);
    if key == dictionary_entry.reading {
        return Vec::new();
    }
    vec![WiktionaryEntry {
        reading: key,
        candidate: dictionary_entry.candidate.clone(),
        suru: false,
        source: EntrySource::WagoOkuri,
    }]
}

fn split_onbin_stem(reading: &str) -> Option<(&str, char)> {
    let mut chars = reading.char_indices().rev();
    let (_, consonant) = chars.next()?;
    if !consonant.is_ascii_lowercase() {
        return None;
    }
    let (previous_index, previous) = chars.next()?;
    Some((
        reading.get(..previous_index + previous.len_utf8())?,
        consonant,
    ))
}

fn is_iku_verb(candidate: &str) -> bool {
    candidate.ends_with(['行', '逝', '往']) && candidate.ends_with("く")
}

fn okuri_candidate(page: &JapanesePage, candidate: &str) -> Option<WiktionaryEntry> {
    if !candidate.contains(is_kanji) {
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
        .filter(|reading| reading.chars().count() >= stem.chars().count())
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

fn starts_with_okuri(okuri_reading: &str, dictionary_reading: &str) -> bool {
    okuri_reading
        .strip_suffix(|ch: char| ch.is_ascii_alphabetic())
        .is_some_and(|stem| dictionary_reading.starts_with(stem))
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

    fn collect_wiktionary_entries(page: &JapanesePage) -> Vec<WiktionaryEntry> {
        wiktionary_entries(page)
    }

    fn entry_strings(entries: &[WiktionaryEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| format!("{} /{}/", entry.reading, entry.candidate))
            .collect()
    }

    #[test]
    fn parses_redirect_targets() {
        assert_eq!(redirect_target("#REDIRECT [[別別]]"), Some("別別"));
        assert_eq!(redirect_target("#REDIRECT[[cœur]]"), Some("cœur"));
        assert_eq!(
            redirect_target("#redirect[[お邪魔します]]"),
            Some("お邪魔します")
        );
        assert_eq!(redirect_target("#転送 [[散散]]"), Some("散散"));
        assert_eq!(redirect_target("＃転送[[云云]]"), Some("云云"));
        assert_eq!(
            redirect_target("#REDIRECT [[アメリカ合衆国#語源]]"),
            Some("アメリカ合衆国")
        );
        assert_eq!(redirect_target("#REDIRECT"), None);
        assert_eq!(redirect_target("=={{ja}}=="), None);
    }

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
            collect_wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "せん".into(),
                candidate: "仙".into(),
                suru: false,
                source: EntrySource::Idiom,
            }]
        );
    }

    #[test]
    fn converts_kanji_template_readings_for_single_kanji() {
        let page = parse_japanese_page(
            "料",
            "=={{L|ja}}==\n{{ja-kanji|常用=リョウ|施策=教育:4|呉音=リョウ<レウ|漢音=リョウ<レウ}}",
        );
        assert_eq!(page.kanji_template_readings, vec!["りょう".to_string()]);
        assert_eq!(
            collect_wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "りょう".into(),
                candidate: "料".into(),
                suru: false,
                source: EntrySource::KanjiReading,
            }]
        );
    }

    #[test]
    fn merges_common_and_on_readings_from_kanji_template() {
        let page = parse_japanese_page(
            "青",
            "{{kana-DEFAULTSORT|せい}}\n=={{L|ja}}==\n{{ja-kanji|常用=セイ|呉音=ショウ<シャウ|漢音=セイ|唐音=チン,シイ}}",
        );
        assert_eq!(
            page.kanji_template_readings,
            vec![
                "せい".to_string(),
                "しょう".to_string(),
                "ちん".to_string(),
                "しい".to_string()
            ]
        );
        let entries = wiktionary_entries(&page);
        assert!(entries.iter().any(|entry| entry.reading == "しょう"
            && entry.candidate == "青"
            && entry.source == EntrySource::KanjiReading));
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
            collect_wiktionary_entries(&page),
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
            collect_wiktionary_entries(&page),
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
            collect_wiktionary_entries(&page),
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
            collect_wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "わすr".into(),
                candidate: "忘".into(),
                suru: false,
                source: EntrySource::WagoOkuri,
            }]
        );
    }

    #[test]
    fn keeps_noun_reading_when_bold_form_has_no_okuri() {
        let page = parse_japanese_page(
            "周知",
            "{{kana-DEFAULTSORT|しゅうち}}\n=={{ja}}==\n'''[[周]] [[知]]'''（[[しゅうち]]）",
        );
        assert_eq!(
            collect_wiktionary_entries(&page),
            vec![WiktionaryEntry {
                reading: "しゅうち".into(),
                candidate: "周知".into(),
                suru: false,
                source: EntrySource::Idiom,
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
            collect_wiktionary_entries(&page),
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
        assert_eq!(page.verb_titles, vec!["あるく".to_string()]);
        assert_eq!(
            entry_strings(&okuri_candidates(&page, "歩く", true)),
            vec!["あるi /歩/".to_string(), "あるk /歩/".to_string()]
        );
        assert_eq!(
            entry_strings(&wiktionary_entries(&page)),
            vec!["あるi /歩/".to_string(), "あるk /歩/".to_string()]
        );
    }

    #[test]
    fn generates_onbin_entry_for_ru_ending_verb() {
        let page = parse_japanese_page(
            "焦る",
            "{{ja-wagokanji|あせる|いる}}{{ja-verb}}【[[焦]]る】",
        );
        assert_eq!(
            page.wagokanji_readings,
            vec!["あせる".to_string(), "いる".to_string()]
        );
        assert_eq!(page.adverb_titles, Vec::<String>::new());
        assert_eq!(
            page.verb_titles,
            vec!["焦る".to_string(), "あせる".to_string(), "いる".to_string()]
        );
        let base_entry = okuri_candidate(&page, "焦る").unwrap();
        assert_eq!(base_entry.reading, "あせr");
        let onbin_entry = okuri_onbin_entries(&base_entry);
        assert_eq!(onbin_entry[0].reading, "あせt");
        assert_eq!(
            entry_strings(&okuri_candidates(&page, "焦る", true)),
            vec!["あせt /焦/".to_string(), "あせr /焦/".to_string()]
        );
        assert_eq!(page.adjective_titles, vec!["焦る".to_string()]);
        assert_eq!(
            entry_strings(&okuri_candidates(&page, "焦る", true)),
            vec!["あせt /焦/".to_string(), "あせr /焦/".to_string()]
        );
        assert_eq!(
            entry_strings(&wiktionary_entries(&page)),
            vec!["あせr /焦/".to_string(), "あせt /焦/".to_string()]
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
    fn generates_no_onbin_entry_for_adverbial_ku() {
        let page = parse_japanese_page(
            "はやく",
            "{{ja-wagokanji|はやく}}{{ja-adv}}【[[早]]く、[[速]]く】",
        );
        let entries = wiktionary_entries(&page);
        assert_eq!(
            page.verb_titles,
            vec!["はやく".to_string(), "早く".to_string(), "速く".to_string()]
        );
        assert_eq!(
            page.adverb_titles,
            vec!["早く".to_string(), "速く".to_string()]
        );
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
    fn parses_head_template_candidate_titles() {
        let page = parse_japanese_page(
            "たくさん",
            "{{head|ja|形容動詞}}【[[沢]][[山]]】\n{{head|ja|adverb}}【[[卓]][[散]]】",
        );
        assert_eq!(page.adjective_titles, vec!["沢山".to_string()]);
        assert_eq!(page.adverb_titles, vec!["卓散".to_string()]);
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
    #[test]
    fn parses_old_japanese_bracket_titles() {
        let page = parse_japanese_page("あるく", "{{ojp-verb}}【[[歩]]く】");
        assert_eq!(page.old_japanese_titles, vec!["歩く".to_string()]);
    }

    #[test]
    fn parses_old_japanese_bare_titles() {
        let page = parse_japanese_page("歩く", "{{ojp-verb|歩く}}");
        assert_eq!(page.old_japanese_titles, vec!["歩く".to_string()]);
    }

    #[test]
    fn parses_old_japanese_conjugations() {
        let page = parse_japanese_page(
            "あるく",
            "{{ojp-verb}}【[[歩]]く】\n{{古典日本語四段活用|ある|く}}",
        );
        assert_eq!(
            page.old_japanese_conjugations,
            vec![OldJapaneseConjugation {
                stem: "ある".into(),
                suffix: "く".into(),
                conjugation: OldJapaneseConjugationType::Shodan,
            }]
        );
    }

    #[test]
    fn generates_old_japanese_okuri_entry() {
        let page = parse_japanese_page(
            "あるく",
            "{{ojp-verb}}【[[歩]]く】\n{{古典日本語四段活用|ある|く}}",
        );
        assert_eq!(
            old_japanese_entries(&page),
            vec![WiktionaryEntry {
                reading: "あるk".into(),
                candidate: "歩".into(),
                suru: false,
                source: EntrySource::OldJapanese,
            }]
        );
    }

    #[test]
    fn generates_old_japanese_adjective_entry() {
        let page = parse_japanese_page(
            "とし",
            "{{ojp-adj}}【[[疾]]し・[[敏]]し】\n{{古典日本語ク活用|と}}",
        );
        assert_eq!(
            old_japanese_entries(&page),
            vec![
                WiktionaryEntry {
                    reading: "と".into(),
                    candidate: "疾".into(),
                    suru: false,
                    source: EntrySource::OldJapanese,
                },
                WiktionaryEntry {
                    reading: "と".into(),
                    candidate: "敏".into(),
                    suru: false,
                    source: EntrySource::OldJapanese,
                },
            ]
        );
    }
}
