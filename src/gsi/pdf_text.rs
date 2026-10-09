use once_cell::sync::Lazy;
use regex::Regex;

use crate::jion::on_reading::katakana_to_hiragana;

use super::jisyo::Jisyo;

static ROW_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^\s*(\d+)\s+(\d*)[ \t]+([^ ]*)[ \t]+([^ ]+)[ \t]+(.*?)[ \t]+(-?\d+)°(\d+)'\s+(-?\d+)°(\d+)'\s+([A-Za-z ]+)$",
    )
    .expect("internal error: invalid gazetteer row regex")
});

/// Generate dictionary entries from text extracted from the Gazetteer of
/// Japan PDF by `pdftotext -layout`.
pub fn generate_jisyo(pdf_text: &str) -> Jisyo {
    let mut jisyo = Jisyo::default();
    for row in pdf_text.lines().filter_map(parse_row) {
        let Some(kanji) = fallback_kanji(&row) else {
            continue;
        };
        let official_kana = katakana_to_hiragana(strip_parenthetical(&row.kana));
        let official_romanized = strip_parenthetical(&row.romanized);
        for reading in official_and_alternate_readings(&row.kana) {
            jisyo.add_entry(&reading, kanji.clone());

            if row.classification == "Municipality" {
                if let Some(base) = base_entry(&kanji, &official_kana, official_romanized) {
                    if let Some(base_reading) = reading.strip_suffix(base.kana_suffix) {
                        if !base_reading.is_empty() {
                            jisyo.add_entry(base_reading, base.candidate);
                        }
                    }
                }
            }
        }
    }
    jisyo
}

fn fallback_kanji(row: &Row) -> Option<String> {
    let is_kana = row
        .kanji
        .chars()
        .all(|ch| matches!(ch, 'ぁ'..='ゖ' | 'ァ'..='ー'));
    if row.kanji.is_empty() || is_kana {
        None
    } else {
        Some(row.kanji.clone())
    }
}

fn official_and_alternate_readings(kana: &str) -> Vec<String> {
    let mut readings = vec![katakana_to_hiragana(strip_alternates(kana))];
    if let Some((_, alternates)) = kana.split_once('（') {
        for alternate in alternates.trim_end_matches('）').split('，') {
            let alternate = katakana_to_hiragana(alternate.trim());
            if !alternate.is_empty() {
                readings.push(alternate);
            }
        }
    }
    readings
}

fn strip_alternates(kana: &str) -> &str {
    strip_parenthetical(kana)
}

fn strip_parenthetical(value: &str) -> &str {
    if let Some((head, _)) = value.split_once('（') {
        return head.trim();
    }
    if let Some((head, _)) = value.split_once(" (") {
        return head.trim();
    }
    value.trim()
}

fn base_entry<'a>(kanji: &'a str, kana: &str, romanized: &str) -> Option<BaseEntry<'a>> {
    let suffixes = [
        ("市", "し", " Shi"),
        ("町", "ちょう", " Cho"),
        ("町", "まち", " Machi"),
        ("村", "むら", " Mura"),
        ("村", "そん", " Son"),
        ("郡", "ぐん", " Gun"),
        ("区", "く", " Ku"),
    ];

    for (kanji_suffix, kana_suffix, romanized_suffix) in suffixes {
        if let Some(candidate) = kanji.strip_suffix(kanji_suffix) {
            if kana.strip_suffix(kana_suffix).is_some()
                && romanized.strip_suffix(romanized_suffix).is_some()
            {
                return Some(BaseEntry {
                    candidate,
                    kana_suffix,
                });
            }
        }
    }
    None
}

struct BaseEntry<'a> {
    candidate: &'a str,
    kana_suffix: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
struct Row {
    #[allow(dead_code)]
    number: u32,
    #[allow(dead_code)]
    grid: Option<u32>,
    kanji: String,
    kana: String,
    romanized: String,
    #[allow(dead_code)]
    latitude: String,
    #[allow(dead_code)]
    longitude: String,
    classification: String,
}

/// Parse one PDF table row. The grid code is absent for undersea features.
/// The kanji field is also absent for a small number of rows in the PDF text
/// layer; those rows are parsed as incomplete and skipped by the generator.
fn parse_row(line: &str) -> Option<Row> {
    let captures = ROW_REGEX.captures(line)?;
    let grid = if captures[2].is_empty() {
        None
    } else {
        Some(captures[2].parse().ok()?)
    };

    Some(Row {
        number: captures[1].parse().ok()?,
        grid,
        kanji: captures[3].to_string(),
        kana: captures[4].to_string(),
        romanized: captures[5].to_string(),
        latitude: format!("{}°{}'", &captures[6], &captures[7]),
        longitude: format!("{}°{}'", &captures[8], &captures[9]),
        classification: captures[10].to_string(),
    })
}

/// Exposed for tests and the binary to inspect the generated base candidate.
pub fn municipality_base_entry(
    kanji: &str,
    kana: &str,
    romanized: &str,
) -> Option<(String, String, String)> {
    base_entry(kanji, kana, romanized).map(|base| {
        (
            base.candidate.to_string(),
            base.kana_suffix.to_string(),
            String::new(),
        )
    })
}
