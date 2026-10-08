use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::jion::on_reading::katakana_to_hiragana;
use crate::jion::wiktionary::okuri_romaji;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

struct Candidate {
    text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    None,
    Literal,
    Reading,
    Nanori,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadingKind {
    On,
    Kun,
    Nanori,
}

struct Reading {
    kind: ReadingKind,
    text: String,
}

/// Bare SKK lines for one kanji. Dotted kun and nanori use `okuri_romaji`.
pub fn parse(text: &str) -> Result<BTreeMap<String, Vec<String>>, ParseError> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let mut entries: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    let mut depth = 0usize;
    let mut saw_root_end = false;
    let mut literal: Option<String> = None;
    let mut readings: Vec<Reading> = Vec::new();
    let mut slot = Slot::None;
    let mut reading_kind: Option<ReadingKind> = None;
    let mut text_buf = String::new();

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| ParseError {
                message: error.to_string(),
            })?;
        match event {
            Event::Start(start) => {
                depth += 1;
                match start.name().0 {
                    "character" => {
                        literal = None;
                        readings.clear();
                    }
                    "literal" => slot = Slot::Literal,
                    "reading" => {
                        slot = Slot::Reading;
                        reading_kind = reading_kind_of(&start);
                    }
                    "nanori" => slot = Slot::Nanori,
                    _ => {}
                }
                text_buf.clear();
            }
            Event::Empty(start) => {
                if start.name().0 == "character" {
                    literal = None;
                    readings.clear();
                }
                if depth == 0 {
                    saw_root_end = true;
                }
            }
            Event::Text(value) => {
                if slot != Slot::None {
                    text_buf.push_str(&value.xml_content(quick_xml::XmlVersion::Implicit1_0));
                }
            }
            Event::GeneralRef(reference) => {
                if slot != Slot::None {
                    if let Some(character) = reference.resolve_char_ref().ok().flatten() {
                        text_buf.push(character);
                    }
                }
            }
            Event::End(end) => {
                if depth == 0 {
                    return Err(truncated());
                }
                depth -= 1;
                if depth == 0 {
                    saw_root_end = true;
                }
                match end.name().0 {
                    "literal" => {
                        let text = text_buf.trim().to_string();
                        if !text.is_empty() {
                            literal = Some(text);
                        }
                        slot = Slot::None;
                    }
                    "reading" => {
                        if let Some(kind) = reading_kind.take() {
                            let text = text_buf.trim().to_string();
                            if !text.is_empty() {
                                readings.push(Reading { kind, text });
                            }
                        }
                        slot = Slot::None;
                    }
                    "nanori" => {
                        let text = text_buf.trim().to_string();
                        if !text.is_empty() {
                            readings.push(Reading {
                                kind: ReadingKind::Nanori,
                                text,
                            });
                        }
                        slot = Slot::None;
                    }
                    "character" => {
                        if let Some(literal) = literal.take() {
                            add_character(&mut entries, &literal, &readings);
                        }
                        readings.clear();
                    }
                    _ => {}
                }
                text_buf.clear();
            }
            Event::Eof => {
                if !saw_root_end || depth != 0 {
                    return Err(truncated());
                }
                break;
            }
            _ => {}
        }
        buffer.clear();
    }

    let mut lines = BTreeMap::new();
    for (midashi, candidates) in entries {
        lines.insert(
            midashi,
            candidates
                .into_iter()
                .map(|candidate| candidate.text)
                .collect(),
        );
    }
    Ok(lines)
}

pub fn write_dictionary(input: &Path, output: &Path) -> io::Result<()> {
    let text = fs::read_to_string(input)?;
    let entries = parse(&text).map_err(|error| io::Error::other(error.to_string()))?;
    publish(output, |writer| write_entries(writer, &entries))
}

pub fn format_entries(entries: &BTreeMap<String, Vec<String>>) -> String {
    let mut buffer = Vec::new();
    write_entries(&mut buffer, entries).expect("writing to a string buffer");
    String::from_utf8(buffer).expect("dictionary lines are UTF-8")
}

fn reading_kind_of(start: &BytesStart<'_>) -> Option<ReadingKind> {
    let attribute = start.try_get_attribute("r_type").ok().flatten()?;
    let value = attribute
        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
        .ok()?;
    match value.as_ref() {
        "ja_on" => Some(ReadingKind::On),
        "ja_kun" => Some(ReadingKind::Kun),
        _ => None,
    }
}

fn add_character(
    entries: &mut BTreeMap<String, Vec<Candidate>>,
    literal: &str,
    readings: &[Reading],
) {
    for reading in readings {
        let Some(midashi) = midashi_for(reading) else {
            continue;
        };
        push(entries, midashi, literal.to_string());
    }
}

fn midashi_for(reading: &Reading) -> Option<String> {
    let text = katakana_to_hiragana(reading.text.trim());
    match reading.kind {
        ReadingKind::On => kana_reading(&text),
        ReadingKind::Kun | ReadingKind::Nanori => kun_midashi(&text),
    }
}

fn kun_midashi(reading: &str) -> Option<String> {
    let Some((head, tail)) = reading.split_once('.') else {
        return kana_reading(reading);
    };
    if head.is_empty() {
        return kana_reading(tail);
    }
    let joined = format!("{head}{tail}");
    kana_reading(&joined)?;
    let romaji = okuri_romaji(tail.chars().next()?)?;
    let mut midashi = head.to_string();
    midashi.push(romaji);
    Some(midashi)
}

fn kana_reading(reading: &str) -> Option<String> {
    if reading.is_empty() {
        return None;
    }
    if reading
        .chars()
        .all(|character| ('ぁ'..='ゖ').contains(&character) || character == 'ー')
    {
        Some(reading.to_string())
    } else {
        None
    }
}

fn push(entries: &mut BTreeMap<String, Vec<Candidate>>, midashi: String, literal: String) {
    let candidates = entries.entry(midashi).or_default();
    if candidates.iter().any(|candidate| candidate.text == literal) {
        return;
    }
    candidates.push(Candidate { text: literal });
}

fn truncated() -> ParseError {
    ParseError {
        message: "truncated KANJIDIC2".to_string(),
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

    fn text() -> String {
        format_entries(&parse(include_str!("../tests/fixtures/kanjidic.xml")).unwrap())
    }

    fn line<'a>(text: &'a str, midashi: &str) -> Option<&'a str> {
        text.lines()
            .find(|line| line.starts_with(&format!("{midashi} /")))
    }

    #[test]
    fn builds_single_kanji_readings() {
        let text = text();
        assert_eq!(line(&text, "こう"), Some("こう /硤/甲/"));
        assert_eq!(line(&text, "ぎょう"), Some("ぎょう /硤/"));
        assert_eq!(line(&text, "たb"), Some("たb /食/"));
        assert_eq!(line(&text, "ひと"), Some("ひと /一/"));
        assert_eq!(line(&text, "ひとt"), Some("ひとt /一/"));
        assert_eq!(line(&text, "はじめ"), Some("はじめ /一/"));
        assert_eq!(line(&text, "かず"), Some("かず /一/"));
        assert_eq!(line(&text, "かぶと"), Some("かぶと /甲/"));
        assert_eq!(line(&text, "こうe"), Some("こうe /甲/"));
        assert!(line(&text, "そ").is_none());
        assert!(line(&text, "たべる").is_none());
        assert!(line(&text, "はじめかず").is_none());
        assert!(line(&text, "うわ").is_none());
        assert!(!text.contains("xia2"));
        assert!(!text.contains("1463"));
        assert!(!text.contains("gorge"));
        assert!(!text
            .chars()
            .any(|character| { ('ァ'..='ヶ').contains(&character) || character == '・' }));
    }

    #[test]
    fn truncated_document_leaves_no_output_file() {
        let directory = std::env::temp_dir().join(format!("kanjidic-jisyo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let input = directory.join("kanjidic2.xml");
        fs::write(&input, "<kanjidic2><character><literal>食</literal>").unwrap();
        let output = directory.join("SKK-JISYO.kanjidic");
        let error = write_dictionary(&input, &output).unwrap_err();
        assert!(error.to_string().contains("truncated"));
        assert!(!output.exists());
        assert!(!partial_path(&output).exists());
        let _ = fs::remove_dir_all(&directory);
    }
}
