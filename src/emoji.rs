use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::jion::on_reading::katakana_to_hiragana;
use crate::model;

const GREEK_LETTERS: &[&str] = &[
    "ALPHA", "BETA", "GAMMA", "DELTA", "EPSILON", "ZETA", "ETA", "THETA", "IOTA", "KAPPA", "LAMDA",
    "MU", "NU", "XI", "OMICRON", "PI", "RHO", "SIGMA", "TAU", "UPSILON", "PHI", "CHI", "PSI",
    "OMEGA",
];

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "{}", self.message)
        } else {
            write!(formatter, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Inputs<'a> {
    pub emoji_test: &'a Path,
    pub ja: &'a Path,
    pub en: &'a Path,
    pub ja_derived: &'a Path,
    pub en_derived: &'a Path,
    pub unicode_data: &'a Path,
    pub names_list: &'a Path,
    pub readings: &'a [&'a Path],
}

pub struct Sources<'a> {
    pub emoji_test: &'a str,
    pub ja: &'a str,
    pub en: &'a str,
    pub ja_derived: &'a str,
    pub en_derived: &'a str,
    pub unicode_data: &'a str,
    pub names_list: &'a str,
    /// Okuri-nasi SKK text, usually `SKK-JISYO.wiktionary` and `SKK-JISYO.jmdict`.
    /// Kanji symbol names take their readings from this text.
    pub readings: &'a str,
}

/// Okuri-nasi lines for emoji, modern Greek letters, and kana-named symbols.
/// A candidate list keeps insertion order, except a final form or symbol follows its letter.
pub fn parse(sources: &Sources<'_>) -> Result<BTreeMap<String, Vec<String>>, ParseError> {
    let emoji = parse_emoji_test(sources.emoji_test)?;
    let emoji_keys = emoji_keys(&emoji);
    let ja = parse_annotations(sources.ja)?;
    let en = parse_annotations(sources.en)?;
    // Derived Japanese names are not midashi. Parsing still rejects a truncated file.
    let _ja_derived = parse_annotations(sources.ja_derived)?;
    let en_derived = parse_annotations(sources.en_derived)?;
    let readings = reading_index(sources.readings);
    let mut entries: BTreeMap<String, Vec<(u8, String)>> = BTreeMap::new();

    for item in &emoji {
        if is_flag(&item.raw) {
            add_english_tts(&mut entries, &en_derived, &item.key, &item.raw);
            continue;
        }
        if item.family || has_skin_tone(&item.raw) {
            continue;
        }
        add_japanese_annotation(&mut entries, &ja, &item.key, &item.raw);
        add_english_tts(&mut entries, &en, &item.key, &item.raw);
    }

    for annotation in &ja {
        let Some(character) = symbol_character(&annotation.sequence) else {
            continue;
        };
        if emoji_keys.contains(&annotation.sequence) || has_skin_tone(&annotation.sequence) {
            continue;
        }
        let candidate = character.to_string();
        if annotation.tts {
            add_symbol_name(&mut entries, annotation.text.trim(), &candidate, &readings);
        } else {
            for token in annotation.text.split('|') {
                add_symbol_name(&mut entries, token.trim(), &candidate, &readings);
            }
        }
    }

    add_greek(&mut entries, sources.unicode_data, sources.names_list)?;

    let mut lines = BTreeMap::new();
    for (midashi, mut candidates) in entries {
        candidates.sort_by_key(|(rank, _)| *rank);
        let texts = candidates.into_iter().map(|(_, text)| text).collect();
        lines.insert(midashi, texts);
    }
    Ok(lines)
}

pub fn write_dictionary(inputs: &Inputs<'_>, output: &Path) -> io::Result<()> {
    let sources = Sources {
        emoji_test: &fs::read_to_string(inputs.emoji_test)?,
        ja: &fs::read_to_string(inputs.ja)?,
        en: &fs::read_to_string(inputs.en)?,
        ja_derived: &fs::read_to_string(inputs.ja_derived)?,
        en_derived: &fs::read_to_string(inputs.en_derived)?,
        unicode_data: &fs::read_to_string(inputs.unicode_data)?,
        names_list: &fs::read_to_string(inputs.names_list)?,
        readings: &read_readings(inputs.readings)?,
    };
    let entries = parse(&sources).map_err(|error| io::Error::other(error.to_string()))?;
    publish(output, |writer| write_entries(writer, &entries))
}

pub fn format_entries(entries: &BTreeMap<String, Vec<String>>) -> String {
    let mut buffer = Vec::new();
    write_entries(&mut buffer, entries).expect("writing to a string buffer");
    String::from_utf8(buffer).expect("dictionary lines are UTF-8")
}

struct QualifiedEmoji {
    raw: String,
    key: String,
    family: bool,
}

struct Annotation {
    sequence: String,
    tts: bool,
    text: String,
}

fn parse_emoji_test(text: &str) -> Result<Vec<QualifiedEmoji>, ParseError> {
    let mut emoji = Vec::new();
    let mut family = false;
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            if let Some(name) = comment.trim().strip_prefix("subgroup:") {
                family = name.trim() == "family";
            }
            continue;
        }
        let Some((codes, rest)) = line.split_once(';') else {
            return Err(invalid_emoji_line(line_number));
        };
        let status = rest.split('#').next().unwrap_or("").trim();
        if !matches!(
            status,
            "fully-qualified" | "unqualified" | "minimally-qualified" | "component"
        ) {
            return Err(invalid_emoji_line(line_number));
        }
        if status != "fully-qualified" {
            continue;
        }
        let raw = decode_codes(codes).map_err(|message| ParseError {
            line: line_number,
            message,
        })?;
        emoji.push(QualifiedEmoji {
            key: lookup_key(&raw),
            raw,
            family,
        });
    }
    Ok(emoji)
}

fn invalid_emoji_line(line: usize) -> ParseError {
    ParseError {
        line,
        message: "invalid emoji-test line".to_string(),
    }
}

fn decode_codes(field: &str) -> Result<String, String> {
    let mut raw = String::new();
    for hex in field.split_whitespace() {
        if hex.is_empty() {
            continue;
        }
        let value =
            u32::from_str_radix(hex, 16).map_err(|_| format!("invalid code point {hex}"))?;
        let character = char::from_u32(value).ok_or_else(|| format!("invalid code point {hex}"))?;
        raw.push(character);
    }
    if raw.is_empty() {
        return Err("invalid code point".to_string());
    }
    Ok(raw)
}

fn emoji_keys(emoji: &[QualifiedEmoji]) -> HashSet<String> {
    emoji.iter().map(|item| item.key.clone()).collect()
}

fn parse_annotations(text: &str) -> Result<Vec<Annotation>, ParseError> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let mut annotations = Vec::new();
    let mut current: Option<AnnotationBuilder> = None;
    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| ParseError {
                line: 0,
                message: error.to_string(),
            })?;
        match event {
            Event::Start(start) if start.name().0 == "annotation" => {
                if current.is_some() {
                    return Err(truncated_annotation());
                }
                current = Some(annotation_builder(&start)?);
            }
            Event::Empty(start) if start.name().0 == "annotation" => {
                let builder = annotation_builder(&start)?;
                annotations.push(builder.finish());
            }
            Event::Text(text) => {
                if let Some(builder) = current.as_mut() {
                    builder
                        .text
                        .push_str(&text.xml_content(quick_xml::XmlVersion::Implicit1_0));
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(builder) = current.as_mut() {
                    if let Some(character) = reference.resolve_char_ref().ok().flatten() {
                        builder.text.push(character);
                    }
                }
            }
            Event::End(end) if end.name().0 == "annotation" => {
                let Some(builder) = current.take() else {
                    return Err(truncated_annotation());
                };
                annotations.push(builder.finish());
            }
            Event::Eof => {
                if current.is_some() {
                    return Err(truncated_annotation());
                }
                break;
            }
            _ => {}
        }
        buffer.clear();
    }
    Ok(annotations)
}

struct AnnotationBuilder {
    sequence: String,
    tts: bool,
    text: String,
}

impl AnnotationBuilder {
    fn finish(self) -> Annotation {
        Annotation {
            sequence: self.sequence,
            tts: self.tts,
            text: self.text,
        }
    }
}

fn annotation_builder(
    start: &quick_xml::events::BytesStart<'_>,
) -> Result<AnnotationBuilder, ParseError> {
    let cp = start
        .try_get_attribute("cp")
        .map_err(|error| ParseError {
            line: 0,
            message: error.to_string(),
        })?
        .ok_or_else(|| ParseError {
            line: 0,
            message: "annotation without cp".to_string(),
        })?;
    let sequence = cp
        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
        .map_err(|error| ParseError {
            line: 0,
            message: error.to_string(),
        })?
        .into_owned();
    if sequence.is_empty() {
        return Err(ParseError {
            line: 0,
            message: "annotation without cp".to_string(),
        });
    }
    let tts = start
        .try_get_attribute("type")
        .map_err(|error| ParseError {
            line: 0,
            message: error.to_string(),
        })?
        .is_some_and(|attribute| attribute.value.as_ref() == "tts");
    Ok(AnnotationBuilder {
        sequence: lookup_key(&sequence),
        tts,
        text: String::new(),
    })
}

fn truncated_annotation() -> ParseError {
    ParseError {
        line: 0,
        message: "truncated annotation".to_string(),
    }
}

fn add_japanese_annotation(
    entries: &mut BTreeMap<String, Vec<(u8, String)>>,
    annotations: &[Annotation],
    key: &str,
    candidate: &str,
) {
    for annotation in annotations
        .iter()
        .filter(|annotation| annotation.sequence == key)
    {
        if annotation.tts {
            add_japanese_token(entries, annotation.text.trim(), candidate, 0);
        } else {
            for token in annotation.text.split('|') {
                add_japanese_token(entries, token.trim(), candidate, 0);
            }
        }
    }
}

fn add_english_tts(
    entries: &mut BTreeMap<String, Vec<(u8, String)>>,
    annotations: &[Annotation],
    key: &str,
    candidate: &str,
) {
    for annotation in annotations
        .iter()
        .filter(|annotation| annotation.tts && annotation.sequence == key)
    {
        if let Some(midashi) = alphabet_midashi(annotation.text.trim()) {
            push(entries, midashi, candidate, 0);
        }
    }
}

fn add_japanese_token(
    entries: &mut BTreeMap<String, Vec<(u8, String)>>,
    token: &str,
    candidate: &str,
    rank: u8,
) {
    if token.is_empty() {
        return;
    }
    if let Some(midashi) = kana_midashi(token) {
        push(entries, midashi, candidate, rank);
    } else if let Some(midashi) = japanese_alphabet_midashi(token) {
        push(entries, midashi, candidate, rank);
    }
}

fn add_symbol_name(
    entries: &mut BTreeMap<String, Vec<(u8, String)>>,
    token: &str,
    candidate: &str,
    readings: &HashMap<String, Vec<String>>,
) {
    if let Some(midashi) = kana_midashi(token) {
        push(entries, midashi, candidate, 0);
        return;
    }
    if let Some(midashi) = japanese_alphabet_midashi(token) {
        push(entries, midashi, candidate, 0);
        return;
    }
    let Some(midashi_list) = readings.get(token) else {
        return;
    };
    for midashi in midashi_list {
        push(entries, midashi.clone(), candidate, 0);
    }
}

/// Readings of kanji words from okuri-nasi entries.
/// A word needs two or more characters. 上 and 丸 have too many readings to use.
/// Okuri-ari midashi such as `うえむk` are not readings of the noun.
fn reading_index(text: &str) -> HashMap<String, Vec<String>> {
    let mut index: HashMap<String, Vec<String>> = HashMap::new();
    let mut okuri_nasi = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with(";;") && line.contains("okuri-ari entries") {
            okuri_nasi = false;
            continue;
        }
        if line.starts_with(";;") && line.contains("okuri-nasi entries") {
            okuri_nasi = true;
            continue;
        }
        if !okuri_nasi || line.is_empty() || line.starts_with(';') {
            continue;
        }
        let Some((midashi, rest)) = line.split_once(' ') else {
            continue;
        };
        let Some(midashi) = kana_midashi(midashi) else {
            continue;
        };
        if !rest.starts_with('/') || !rest.ends_with('/') {
            continue;
        }
        for candidate in rest[1..rest.len() - 1].split('/') {
            let word = candidate.split(';').next().unwrap_or("").trim();
            if word.chars().count() < 2 || !word.chars().any(model::is_kanji) {
                continue;
            }
            let readings = index.entry(word.to_string()).or_default();
            if !readings.contains(&midashi) {
                readings.push(midashi.clone());
            }
        }
    }
    index
}

fn read_readings(paths: &[&Path]) -> io::Result<String> {
    let mut text = String::new();
    for path in paths {
        let chunk = fs::read_to_string(path)?;
        text.push_str(&chunk);
        if !chunk.ends_with('\n') {
            text.push('\n');
        }
    }
    Ok(text)
}

fn add_greek(
    entries: &mut BTreeMap<String, Vec<(u8, String)>>,
    unicode_data: &str,
    names_list: &str,
) -> Result<(), ParseError> {
    let aliases = parse_name_aliases(names_list);
    for (index, line) in unicode_data.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split(';');
        let Some(code) = fields.next() else {
            return Err(invalid_unicode_data(line_number));
        };
        let Some(name) = fields.next() else {
            return Err(invalid_unicode_data(line_number));
        };
        if name.is_empty() {
            return Err(invalid_unicode_data(line_number));
        }
        let Some((midashi, rank)) = greek_reading(name) else {
            continue;
        };
        let Some(character) = parse_hex_code(code) else {
            return Err(invalid_unicode_data(line_number));
        };
        let candidate = character.to_string();
        push(entries, midashi.clone(), &candidate, rank);
        for alias in aliases.get(&character).into_iter().flatten() {
            let Some(alias_midashi) = single_word_alias(alias) else {
                continue;
            };
            if alias_midashi.eq_ignore_ascii_case(&midashi) {
                continue;
            }
            push(entries, alias_midashi, &candidate, 0);
        }
    }
    Ok(())
}

fn invalid_unicode_data(line: usize) -> ParseError {
    ParseError {
        line,
        message: "invalid UnicodeData line".to_string(),
    }
}

fn greek_reading(name: &str) -> Option<(String, u8)> {
    if let Some(rest) = name.strip_prefix("GREEK SMALL LETTER ") {
        if rest == "FINAL SIGMA" {
            return Some(("sigma".to_string(), 1));
        }
        if GREEK_LETTERS.contains(&rest) {
            return Some((rest.to_ascii_lowercase(), 0));
        }
        return None;
    }
    if let Some(rest) = name.strip_prefix("GREEK CAPITAL LETTER ") {
        if GREEK_LETTERS.contains(&rest) {
            return Some((capitalize_word(&rest.to_ascii_lowercase()), 0));
        }
        return None;
    }
    match name {
        "GREEK KAPPA SYMBOL" => Some(("kappa".to_string(), 1)),
        "GREEK PHI SYMBOL" => Some(("phi".to_string(), 1)),
        "GREEK THETA SYMBOL" => Some(("theta".to_string(), 1)),
        "GREEK PI SYMBOL" => Some(("pi".to_string(), 1)),
        "GREEK RHO SYMBOL" => Some(("rho".to_string(), 1)),
        "GREEK LUNATE EPSILON SYMBOL" => Some(("epsilon".to_string(), 1)),
        "MICRO SIGN" => Some(("mu".to_string(), 1)),
        _ => None,
    }
}

fn capitalize_word(lower: &str) -> String {
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut midashi = first.to_uppercase().collect::<String>();
    midashi.push_str(chars.as_str());
    midashi
}

fn parse_name_aliases(text: &str) -> HashMap<char, Vec<String>> {
    let mut aliases: HashMap<char, Vec<String>> = HashMap::new();
    let mut current = None;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        if line.starts_with(|ch: char| ch.is_whitespace()) {
            let trimmed = line.trim_start();
            let Some(alias) = trimmed.strip_prefix('=') else {
                continue;
            };
            let Some(character) = current else {
                continue;
            };
            for piece in alias.split(',') {
                let piece = piece.trim();
                if !piece.is_empty() {
                    aliases
                        .entry(character)
                        .or_default()
                        .push(piece.to_string());
                }
            }
            continue;
        }
        let Some((hex, _)) = line.split_once('\t') else {
            current = None;
            continue;
        };
        current = parse_hex_code(hex);
    }
    aliases
}

/// A one-word formal alias, with a trailing parenthetical removed.
/// `lambda` stays. `stigma (the Modern Greek name...)` becomes `stigma`.
/// `script theta` is left out.
fn single_word_alias(alias: &str) -> Option<String> {
    let word = alias.split('(').next().unwrap_or(alias).trim();
    if word.is_empty() || !word.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }
    Some(word.to_ascii_lowercase())
}

fn parse_hex_code(hex: &str) -> Option<char> {
    if !(4..=6).contains(&hex.len()) || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    char::from_u32(u32::from_str_radix(hex, 16).ok()?)
}

fn push(
    entries: &mut BTreeMap<String, Vec<(u8, String)>>,
    midashi: String,
    candidate: &str,
    rank: u8,
) {
    let list = entries.entry(midashi).or_default();
    if list.iter().any(|(_, existing)| existing == candidate) {
        return;
    }
    list.push((rank, candidate.to_string()));
}

fn kana_midashi(token: &str) -> Option<String> {
    if token.is_empty() || !token.chars().all(is_kana) {
        return None;
    }
    Some(katakana_to_hiragana(token))
}

fn japanese_alphabet_midashi(token: &str) -> Option<String> {
    if token.chars().any(|ch| is_kana(ch) || model::is_kanji(ch)) {
        return None;
    }
    if !token.chars().any(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }
    alphabet_midashi(token)
}

fn alphabet_midashi(token: &str) -> Option<String> {
    let mut midashi = String::new();
    for ch in token.chars() {
        if ch.is_ascii_alphanumeric() {
            midashi.push(ch.to_ascii_lowercase());
        }
    }
    if midashi.is_empty() || midashi.chars().all(|ch| ch.is_ascii_digit()) {
        None
    } else {
        Some(midashi)
    }
}

fn is_kana(ch: char) -> bool {
    ('ぁ'..='ゖ').contains(&ch) || ('ァ'..='ヶ').contains(&ch) || ch == 'ー'
}

fn lookup_key(sequence: &str) -> String {
    sequence.chars().filter(|ch| *ch != '\u{FE0F}').collect()
}

fn is_flag(sequence: &str) -> bool {
    let mut chars = sequence.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(first), Some(second), None) => is_regional(first) && is_regional(second),
        _ => false,
    }
}

fn is_regional(ch: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&ch)
}

fn has_skin_tone(sequence: &str) -> bool {
    sequence
        .chars()
        .any(|ch| ('\u{1F3FB}'..='\u{1F3FF}').contains(&ch))
}

fn symbol_character(sequence: &str) -> Option<char> {
    let mut chars = sequence.chars();
    match (chars.next(), chars.next()) {
        (Some(character), None)
            if !('\u{0020}'..='\u{007E}').contains(&character) && !has_skin_tone(sequence) =>
        {
            Some(character)
        }
        _ => None,
    }
}

fn write_entries(
    writer: &mut dyn Write,
    entries: &BTreeMap<String, Vec<String>>,
) -> io::Result<()> {
    for (midashi, candidates) in entries {
        write!(writer, "{midashi} /")?;
        for candidate in candidates {
            write!(writer, "{candidate}/")?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn publish(
    output: &Path,
    write_body: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    let partial = partial_path(output);
    let result = (|| {
        let file = File::create(&partial)?;
        let mut writer = BufWriter::new(file);
        write_body(&mut writer)?;
        writer.flush()?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    if let Err(error) = fs::rename(&partial, output) {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    Ok(())
}

fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> Sources<'static> {
        Sources {
            emoji_test: include_str!("../tests/fixtures/emoji/emoji-test.txt"),
            ja: include_str!("../tests/fixtures/emoji/ja.xml"),
            en: include_str!("../tests/fixtures/emoji/en.xml"),
            ja_derived: include_str!("../tests/fixtures/emoji/ja-derived.xml"),
            en_derived: include_str!("../tests/fixtures/emoji/en-derived.xml"),
            unicode_data: include_str!("../tests/fixtures/emoji/UnicodeData.txt"),
            names_list: include_str!("../tests/fixtures/emoji/NamesList.txt"),
            readings: "",
        }
    }

    fn line<'a>(text: &'a str, midashi: &str) -> Option<&'a str> {
        text.lines()
            .find(|line| line.starts_with(&format!("{midashi} /")))
    }

    #[test]
    fn builds_emoji_greek_letters_and_symbols() {
        let text = format_entries(&parse(&sources()).unwrap());
        assert_eq!(line(&text, "にっこり"), Some("にっこり /😀/"));
        assert_eq!(line(&text, "すまいる"), Some("すまいる /😀/🙂/"));
        assert_eq!(line(&text, "grinningface"), Some("grinningface /😀/"));
        assert_eq!(line(&text, "いいね"), Some("いいね /👍/"));
        assert_eq!(line(&text, "さむずあっぷ"), Some("さむずあっぷ /👍/"));
        assert_eq!(line(&text, "ok"), Some("ok /👍/"));
        assert_eq!(line(&text, "thumbsup"), Some("thumbsup /👍/"));
        assert_eq!(line(&text, "はーと"), Some("はーと /\u{2764}\u{FE0F}/"));
        assert_eq!(line(&text, "redheart"), Some("redheart /\u{2764}\u{FE0F}/"));
        assert_eq!(line(&text, "flagjapan"), Some("flagjapan /🇯🇵/"));
        assert_eq!(line(&text, "いんふぃにてぃ"), Some("いんふぃにてぃ /∞/"));
        assert_eq!(line(&text, "ゆーろ"), Some("ゆーろ /€/"));
        assert_eq!(line(&text, "eur"), Some("eur /€/"));
        assert_eq!(line(&text, "こぴーらいと"), Some("こぴーらいと /©/"));
        assert_eq!(line(&text, "とれーどまーく"), Some("とれーどまーく /™/"));
        assert_eq!(line(&text, "きゃれっと"), Some("きゃれっと /‸/"));
        assert_eq!(line(&text, "kappa"), Some("kappa /κ/ϰ/"));
        assert_eq!(line(&text, "Kappa"), Some("Kappa /Κ/"));
        assert_eq!(line(&text, "lamda"), Some("lamda /λ/"));
        assert_eq!(line(&text, "lambda"), Some("lambda /λ/"));
        assert_eq!(line(&text, "Lamda"), Some("Lamda /Λ/"));
        assert_eq!(line(&text, "mu"), Some("mu /μ/µ/"));
        assert_eq!(line(&text, "sigma"), Some("sigma /σ/ς/"));
        assert_eq!(line(&text, "stigma"), Some("stigma /ς/"));
        assert_eq!(line(&text, "phi"), Some("phi /φ/ϕ/"));
        for absent in [
            "にっこりわらう",
            "えがお",
            "face",
            "あかい",
            "にほん",
            "はた",
            "infinity",
            "かっこ",
            "はいふん",
            "びっくり",
            "digamma",
            "familymanwomangirl",
            "thumbsuplightskintone",
            "かっぱ",
        ] {
            assert_eq!(line(&text, absent), None, "{absent}");
        }
        assert!(!text.contains('→'));
        assert!(!text.contains('※'));
        assert!(!text.contains('♪'));
        assert!(!text.contains("👍🏻"));
        assert!(!text.contains('{'));
    }

    #[test]
    fn reads_kanji_symbol_names_from_okuri_nasi() {
        let mut input = sources();
        input.ja = r#"
            <annotation cp="▲">三角 | 上 | 上向黒三角</annotation>
            <annotation cp="▲" type="tts">上向黒三角</annotation>
            <annotation cp="▼">三角</annotation>
            <annotation cp="▼" type="tts">下向黒三角</annotation>
            <annotation cp="△">上向き白三角</annotation>
            <annotation cp="△" type="tts">上向き白三角</annotation>
            <annotation cp="→">右 | 矢印</annotation>
            <annotation cp="→" type="tts">右向矢印</annotation>
            <annotation cp="※">米印</annotation>
            <annotation cp="※" type="tts">米印</annotation>
            <annotation cp="●">黒丸</annotation>
            <annotation cp="●" type="tts">黒丸</annotation>
            <annotation cp="😀">笑顔</annotation>
            <annotation cp="😀" type="tts">笑顔</annotation>
        "#;
        input.readings = "\
;; okuri-ari entries.
うえむk /上向/
;; okuri-nasi entries.
さんかく /三角;幾何/参画/
サンカク /三角/
やじるし /矢印/
こめじるし /米印/
くろまる /黒丸/
うえ /上/
えがお /笑顔/
";
        let text = format_entries(&parse(&input).unwrap());
        assert_eq!(line(&text, "さんかく"), Some("さんかく /▲/▼/"));
        assert_eq!(line(&text, "やじるし"), Some("やじるし /→/"));
        assert_eq!(line(&text, "こめじるし"), Some("こめじるし /※/"));
        assert_eq!(line(&text, "くろまる"), Some("くろまる /●/"));
        for absent in ["うえ", "うえむk", "えがお", "あ"] {
            assert_eq!(line(&text, absent), None, "{absent}");
        }
        assert!(!text.contains('△'));
    }

    #[test]
    fn rejects_a_bad_emoji_test_line() {
        let mut input = sources();
        input.emoji_test = "not a code\n";
        let error = parse(&input).unwrap_err();
        assert_eq!(error.line, 1);
        assert!(error.message.contains("invalid emoji-test line"));
    }

    #[test]
    fn truncated_annotation_leaves_no_output_file() {
        let directory = std::env::temp_dir().join(format!("emoji-jisyo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let ja = directory.join("ja.xml");
        fs::write(&ja, "<annotation cp=\"😀\">にっこり").unwrap();
        for name in [
            "emoji-test.txt",
            "en.xml",
            "ja-derived.xml",
            "en-derived.xml",
            "UnicodeData.txt",
            "NamesList.txt",
        ] {
            fs::write(directory.join(name), "").unwrap();
        }
        fs::write(directory.join("emoji-test.txt"), "# empty\n").unwrap();
        let output = directory.join("SKK-JISYO.emoji");
        let inputs = Inputs {
            emoji_test: &directory.join("emoji-test.txt"),
            ja: &ja,
            en: &directory.join("en.xml"),
            ja_derived: &directory.join("ja-derived.xml"),
            en_derived: &directory.join("en-derived.xml"),
            unicode_data: &directory.join("UnicodeData.txt"),
            names_list: &directory.join("NamesList.txt"),
            readings: &[],
        };
        let error = write_dictionary(&inputs, &output).unwrap_err();
        assert!(error.to_string().contains("truncated annotation"));
        assert!(!output.exists());
        assert!(!partial_path(&output).exists());
        let _ = fs::remove_dir_all(&directory);
    }
}
