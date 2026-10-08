use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::jion::on_reading::katakana_to_hiragana;
use crate::jion::wiktionary::{okuri_romaji, split_candidate};
use crate::model::is_kanji;

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
    priority: bool,
    order: u32,
}

struct Keb {
    text: String,
    priority: bool,
}

struct Reb {
    text: String,
    restr: Vec<String>,
    priority: bool,
    nokanji: bool,
}

struct Entry {
    kebs: Vec<Keb>,
    rebs: Vec<Reb>,
    okuri: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    None,
    Keb,
    Reb,
    Restr,
    Pos,
}

/// Bare SKK lines from JMdict_e. Okuri verbs and i-adjectives use `okuri_romaji`.
pub fn parse(text: &str) -> Result<BTreeMap<String, Vec<String>>, ParseError> {
    let entities = parse_entities(text);
    let mut reader = Reader::from_str(text);
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let mut entries: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    let mut depth = 0usize;
    let mut saw_root_end = false;
    let mut entry: Option<Entry> = None;
    let mut current_keb: Option<Keb> = None;
    let mut current_reb: Option<Reb> = None;
    let mut slot = Slot::None;
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
                    "entry" => {
                        entry = Some(Entry {
                            kebs: Vec::new(),
                            rebs: Vec::new(),
                            okuri: false,
                        });
                    }
                    "k_ele" => {
                        current_keb = Some(Keb {
                            text: String::new(),
                            priority: false,
                        });
                    }
                    "r_ele" => {
                        current_reb = Some(Reb {
                            text: String::new(),
                            restr: Vec::new(),
                            priority: false,
                            nokanji: false,
                        });
                    }
                    "keb" => slot = Slot::Keb,
                    "reb" => slot = Slot::Reb,
                    "re_restr" => {
                        slot = Slot::Restr;
                        if let Some(reb) = current_reb.as_mut() {
                            reb.restr.push(String::new());
                        }
                    }
                    "ke_pri" => {
                        if let Some(keb) = current_keb.as_mut() {
                            keb.priority = true;
                        }
                    }
                    "re_pri" => {
                        if let Some(reb) = current_reb.as_mut() {
                            reb.priority = true;
                        }
                    }
                    "re_nokanji" => {
                        if let Some(reb) = current_reb.as_mut() {
                            reb.nokanji = true;
                        }
                    }
                    "pos" => slot = Slot::Pos,
                    _ => {}
                }
                text_buf.clear();
            }
            Event::Empty(start) => {
                match start.name().0 {
                    "ke_pri" => {
                        if let Some(keb) = current_keb.as_mut() {
                            keb.priority = true;
                        }
                    }
                    "re_pri" => {
                        if let Some(reb) = current_reb.as_mut() {
                            reb.priority = true;
                        }
                    }
                    "re_nokanji" => {
                        if let Some(reb) = current_reb.as_mut() {
                            reb.nokanji = true;
                        }
                    }
                    _ => {}
                }
                if depth == 0 {
                    saw_root_end = true;
                }
            }
            Event::Text(value) => {
                append_text(
                    &mut text_buf,
                    slot,
                    &value.xml_content(quick_xml::XmlVersion::Implicit1_0),
                );
            }
            Event::GeneralRef(reference) => {
                if slot == Slot::Pos {
                    if let Some(name) = entity_name(&reference) {
                        if let Some(entry) = entry.as_mut() {
                            entry.okuri |= is_okuri_pos(&name);
                        }
                    }
                } else if let Some(character) = reference.resolve_char_ref().ok().flatten() {
                    append_text(&mut text_buf, slot, &character.to_string());
                } else if let Some(name) = entity_name(&reference) {
                    if let Some(value) = entities.get(&name) {
                        append_text(&mut text_buf, slot, value);
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
                    "keb" => {
                        if let Some(keb) = current_keb.as_mut() {
                            keb.text = text_buf.trim().to_string();
                        }
                        slot = Slot::None;
                    }
                    "reb" => {
                        if let Some(reb) = current_reb.as_mut() {
                            reb.text = text_buf.trim().to_string();
                        }
                        slot = Slot::None;
                    }
                    "re_restr" => {
                        if let Some(reb) = current_reb.as_mut() {
                            if let Some(restr) = reb.restr.last_mut() {
                                *restr = text_buf.trim().to_string();
                            }
                        }
                        slot = Slot::None;
                    }
                    "pos" => {
                        if let Some(entry) = entry.as_mut() {
                            entry.okuri |= is_okuri_pos(text_buf.trim());
                        }
                        slot = Slot::None;
                    }
                    "k_ele" => {
                        if let Some(keb) = current_keb.take() {
                            if keb.text.chars().any(is_kanji) {
                                if let Some(entry) = entry.as_mut() {
                                    if let Some(existing) =
                                        entry.kebs.iter_mut().find(|item| item.text == keb.text)
                                    {
                                        existing.priority |= keb.priority;
                                    } else {
                                        entry.kebs.push(keb);
                                    }
                                }
                            }
                        }
                    }
                    "r_ele" => {
                        if let Some(reb) = current_reb.take() {
                            if !reb.text.is_empty() && !reb.nokanji {
                                if let Some(entry) = entry.as_mut() {
                                    entry.rebs.push(reb);
                                }
                            }
                        }
                    }
                    "entry" => {
                        if let Some(entry) = entry.take() {
                            add_entry(&mut entries, entry);
                        }
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
    for (midashi, mut candidates) in entries {
        candidates.sort_by_key(|candidate| (!candidate.priority, candidate.order));
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

fn add_entry(entries: &mut BTreeMap<String, Vec<Candidate>>, entry: Entry) {
    if entry.kebs.is_empty() {
        return;
    }
    for reb in &entry.rebs {
        let Some(reading) = normalize_reading(&reb.text) else {
            continue;
        };
        for keb in &entry.kebs {
            if !reb.restr.is_empty() && !reb.restr.iter().any(|restr| restr == &keb.text) {
                continue;
            }
            let priority = keb.priority || reb.priority;
            if entry.okuri {
                if let Some((midashi, stem)) = okuri_pair(&keb.text, &reading) {
                    push(entries, midashi, stem, priority);
                    continue;
                }
            }
            push(entries, reading.clone(), keb.text.clone(), priority);
        }
    }
}

fn push(
    entries: &mut BTreeMap<String, Vec<Candidate>>,
    midashi: String,
    candidate: String,
    priority: bool,
) {
    let candidates = entries.entry(midashi).or_default();
    if let Some(existing) = candidates.iter_mut().find(|item| item.text == candidate) {
        existing.priority |= priority;
        return;
    }
    let order = candidates.len() as u32;
    candidates.push(Candidate {
        text: candidate,
        priority,
        order,
    });
}

fn okuri_pair(keb: &str, reading: &str) -> Option<(String, String)> {
    let (head, tail) = split_candidate(keb);
    if head.is_empty() || tail.is_empty() {
        return None;
    }
    let tail = katakana_to_hiragana(tail);
    if tail.is_empty() {
        return None;
    }
    let stem = reading.strip_suffix(tail.as_str())?;
    if stem.is_empty() {
        return None;
    }
    let romaji = okuri_romaji(tail.chars().next()?)?;
    let mut midashi = stem.to_string();
    midashi.push(romaji);
    Some((midashi, head.to_string()))
}

fn normalize_reading(raw: &str) -> Option<String> {
    let reading = katakana_to_hiragana(raw.trim());
    if reading.is_empty() {
        return None;
    }
    if reading
        .chars()
        .all(|character| ('ぁ'..='ゖ').contains(&character) || character == 'ー')
    {
        Some(reading)
    } else {
        None
    }
}

/// `adj-i`, `adj-ix`, and verb entities. Bare `vs`, `vi`, and `vt` are not okuri tags.
fn is_okuri_pos(name: &str) -> bool {
    name == "adj-i"
        || name == "adj-ix"
        || (name.starts_with('v') && name != "vs" && name != "vi" && name != "vt")
}

fn entity_name(reference: &quick_xml::events::BytesRef<'_>) -> Option<String> {
    if reference.resolve_char_ref().ok().flatten().is_some() {
        return None;
    }
    let name = reference
        .xml_content(quick_xml::XmlVersion::Implicit1_0)
        .into_owned();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn append_text(buffer: &mut String, slot: Slot, value: &str) {
    if slot != Slot::None {
        buffer.push_str(value);
    }
}

fn parse_entities(text: &str) -> HashMap<String, String> {
    let mut entities = HashMap::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!ENTITY") {
        rest = &rest[start + "<!ENTITY".len()..];
        rest = rest.trim_start();
        let name_len = rest
            .find(|character: char| character.is_whitespace())
            .unwrap_or(rest.len());
        let name = &rest[..name_len];
        rest = rest[name_len..].trim_start();
        let Some(quote) = rest.chars().next() else {
            break;
        };
        if quote != '"' && quote != '\'' {
            continue;
        }
        rest = &rest[quote.len_utf8()..];
        let Some(end) = rest.find(quote) else {
            break;
        };
        if !name.is_empty() {
            entities.insert(name.to_string(), rest[..end].to_string());
        }
        rest = &rest[end + quote.len_utf8()..];
    }
    entities
}

fn truncated() -> ParseError {
    ParseError {
        message: "truncated JMdict".to_string(),
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
        format_entries(&parse(include_str!("../tests/fixtures/jmdict.xml")).unwrap())
    }

    fn line<'a>(text: &'a str, midashi: &str) -> Option<&'a str> {
        text.lines()
            .find(|line| line.starts_with(&format!("{midashi} /")))
    }

    #[test]
    fn builds_vocabulary() {
        let text = text();
        assert_eq!(line(&text, "たb"), Some("たb /食/"));
        assert_eq!(line(&text, "かk"), Some("かk /書/"));
        assert_eq!(line(&text, "くr"), Some("くr /来/"));
        assert_eq!(line(&text, "あu"), Some("あu /会/"));
        assert_eq!(line(&text, "たかi"), Some("たかi /高/"));
        assert_eq!(line(&text, "いk"), Some("いk /行/"));
        assert_eq!(line(&text, "にほんご"), Some("にほんご /日本語/"));
        assert_eq!(line(&text, "べんきょう"), Some("べんきょう /勉強/"));
        assert_eq!(
            line(&text, "べんきょうする"),
            Some("べんきょうする /勉強する/")
        );
        assert_eq!(line(&text, "こう"), Some("こう /甲/"));
        assert_eq!(line(&text, "おつ"), Some("おつ /甲/乙/"));
        assert_eq!(line(&text, "じゅんばん"), Some("じゅんばん /優先/普通/"));
        assert_eq!(line(&text, "かたかな"), Some("かたかな /片仮名/"));
        assert_eq!(line(&text, "ふめい"), Some("ふめい /不明/"));
        assert!(line(&text, "いt").is_none());
        assert!(line(&text, "たべる").is_none());
        assert!(line(&text, "いく").is_none());
        assert!(line(&text, "べんきょうs").is_none());
        assert!(line(&text, "かなだけ").is_none());
        assert!(line(&text, "ひらがな").is_none());
        assert!(line(&text, "なかぐろ").is_none());
        assert!(!text.contains("to eat"));
        assert!(!text.contains("usually written"));
    }

    #[test]
    fn truncated_document_leaves_no_output_file() {
        let directory = std::env::temp_dir().join(format!("jmdict-jisyo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let input = directory.join("JMdict_e.xml");
        fs::write(&input, "<JMdict><entry><k_ele><keb>食</keb>").unwrap();
        let output = directory.join("SKK-JISYO.jmdict");
        let error = write_dictionary(&input, &output).unwrap_err();
        assert!(error.to_string().contains("truncated"));
        assert!(!output.exists());
        assert!(!partial_path(&output).exists());
        let _ = fs::remove_dir_all(&directory);
    }
}
