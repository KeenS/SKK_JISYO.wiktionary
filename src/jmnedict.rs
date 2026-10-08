use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::jion::on_reading::katakana_to_hiragana;

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

#[derive(Clone, Copy, Default)]
struct Labels {
    surname: bool,
    given: bool,
    person: bool,
    place: bool,
    station: bool,
    company: bool,
    organization: bool,
    product: bool,
    work: bool,
}

impl Labels {
    fn is_empty(self) -> bool {
        !self.surname
            && !self.given
            && !self.person
            && !self.place
            && !self.station
            && !self.company
            && !self.organization
            && !self.product
            && !self.work
    }

    fn union(&mut self, other: Self) {
        self.surname |= other.surname;
        self.given |= other.given;
        self.person |= other.person;
        self.place |= other.place;
        self.station |= other.station;
        self.company |= other.company;
        self.organization |= other.organization;
        self.product |= other.product;
        self.work |= other.work;
    }

    fn annotation(self) -> String {
        let mut parts = Vec::new();
        if self.surname {
            parts.push("姓");
        }
        if self.given {
            parts.push("名");
        }
        if self.person {
            parts.push("人名");
        }
        if self.place {
            parts.push("地名");
        }
        if self.station {
            parts.push("駅");
        }
        if self.company {
            parts.push("会社");
        }
        if self.organization {
            parts.push("組織");
        }
        if self.product {
            parts.push("製品");
        }
        if self.work {
            parts.push("作品");
        }
        parts.join(";")
    }
}

struct Candidate {
    keb: String,
    labels: Labels,
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
}

struct Entry {
    kebs: Vec<Keb>,
    rebs: Vec<Reb>,
    labels: Labels,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    None,
    Keb,
    Reb,
    Restr,
    NameType,
}

/// Okuri-nasi lines for JMnedict names. A candidate is `表記;姓` and the other labels in `Labels::annotation`.
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
                            labels: Labels::default(),
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
                    "name_type" => slot = Slot::NameType,
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
                if let Some(character) = reference.resolve_char_ref().ok().flatten() {
                    append_text(&mut text_buf, slot, &character.to_string());
                } else {
                    let name = reference
                        .xml_content(quick_xml::XmlVersion::Implicit1_0)
                        .into_owned();
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
                    "name_type" => {
                        if let Some(entry) = entry.as_mut() {
                            entry.labels.union(label_for(text_buf.trim()));
                        }
                        slot = Slot::None;
                    }
                    "k_ele" => {
                        if let Some(keb) = current_keb.take() {
                            if !keb.text.is_empty() {
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
                            if !reb.text.is_empty() {
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
                .map(|candidate| format!("{};{}", candidate.keb, candidate.labels.annotation()))
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
    if entry.kebs.is_empty() || entry.labels.is_empty() {
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
            if spells_leading_letters(&reading, &keb.text) {
                continue;
            }
            push(
                entries,
                reading.clone(),
                keb.text.clone(),
                entry.labels,
                keb.priority || reb.priority,
            );
        }
    }
    for keb in &entry.kebs {
        if let Some(midashi) = ascii_token(&keb.text) {
            push(
                entries,
                midashi,
                keb.text.clone(),
                entry.labels,
                keb.priority,
            );
        }
    }
}

fn push(
    entries: &mut BTreeMap<String, Vec<Candidate>>,
    reading: String,
    keb: String,
    labels: Labels,
    priority: bool,
) {
    let candidates = entries.entry(reading).or_default();
    if let Some(existing) = candidates.iter_mut().find(|candidate| candidate.keb == keb) {
        existing.labels.union(labels);
        existing.priority |= priority;
        return;
    }
    let order = candidates.len() as u32;
    candidates.push(Candidate {
        keb,
        labels,
        priority,
        order,
    });
}

fn normalize_reading(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(token) = ascii_token(trimmed) {
        return Some(token);
    }
    let reading = katakana_to_hiragana(trimmed);
    if reading.chars().all(|character| {
        ('ぁ'..='ゖ').contains(&character) || character == 'ー' || character == '・'
    }) {
        Some(reading)
    } else {
        None
    }
}

fn fold_fullwidth(character: char) -> char {
    match character {
        '\u{FF01}'..='\u{FF5E}' => char::from_u32(character as u32 - 0xFEE0).unwrap_or(character),
        _ => character,
    }
}

/// Letters and digits, folded from fullwidth. Digits alone are not a midashi.
fn ascii_token(text: &str) -> Option<String> {
    let mut token = String::new();
    let mut saw_letter = false;
    for character in text.chars() {
        let folded = fold_fullwidth(character);
        if !folded.is_ascii_alphanumeric() {
            return None;
        }
        if folded.is_ascii_alphabetic() {
            saw_letter = true;
        }
        token.push(folded);
    }
    saw_letter.then_some(token)
}

fn letter_names(letter: char) -> &'static [&'static str] {
    match letter {
        'A' => &["えー"],
        'B' => &["びー"],
        'C' => &["しー"],
        'D' => &["でぃー"],
        'E' => &["いー"],
        'F' => &["えふ"],
        'G' => &["じー"],
        'H' => &["えいち"],
        'I' => &["あい"],
        'J' => &["じぇー", "じぇい"],
        'K' => &["けー"],
        'L' => &["える"],
        'M' => &["えむ"],
        'N' => &["えぬ"],
        'O' => &["おー"],
        'P' => &["ぴー"],
        'Q' => &["きゅー"],
        'R' => &["あーる"],
        'S' => &["えす"],
        'T' => &["てぃー"],
        'U' => &["ゆー"],
        'V' => &["ぶい", "ゔぃー", "ゔぃ"],
        'W' => &["だぶりゅー", "だぶりゅ"],
        'X' => &["えっくす"],
        'Y' => &["わい"],
        'Z' => &["ぜっと", "ずぃー", "ぜっど"],
        _ => &[],
    }
}

fn spells_letters(reading: &str, letters: &[char]) -> bool {
    fn rec(reading: &str, letters: &[char]) -> bool {
        if letters.is_empty() {
            return true;
        }
        letter_names(letters[0]).iter().any(|name| {
            reading
                .strip_prefix(name)
                .is_some_and(|rest| rec(rest, &letters[1..]))
        })
    }
    !letters.is_empty() && rec(reading, letters)
}

/// True when `reading` begins with the kana spelling of the Latin letters that open `keb`.
fn spells_leading_letters(reading: &str, keb: &str) -> bool {
    let letters = keb
        .chars()
        .map(fold_fullwidth)
        .take_while(|character| character.is_ascii_alphabetic())
        .map(|character| character.to_ascii_uppercase())
        .collect::<Vec<_>>();
    spells_letters(reading, &letters)
}

fn label_for(name_type: &str) -> Labels {
    let mut labels = Labels::default();
    match name_type {
        "surname" | "family or surname" => labels.surname = true,
        "given name or forename, gender not specified"
        | "female given name or forename"
        | "male given name or forename" => labels.given = true,
        "unclassified name" | "full name of a particular person" => labels.person = true,
        "place name" => labels.place = true,
        "railway station" => labels.station = true,
        "company name" => labels.company = true,
        "organization name" => labels.organization = true,
        "product name" => labels.product = true,
        "work of art, literature, music, etc. name" => labels.work = true,
        _ => {}
    }
    labels
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
        message: "truncated JMnedict".to_string(),
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
        format_entries(&parse(include_str!("../tests/fixtures/jmnedict.xml")).unwrap())
    }

    fn line<'a>(text: &'a str, midashi: &str) -> Option<&'a str> {
        text.lines()
            .find(|line| line.starts_with(&format!("{midashi} /")))
    }

    #[test]
    fn builds_personal_names() {
        let text = text();
        assert_eq!(line(&text, "やまだ"), Some("やまだ /山田;姓/"));
        assert_eq!(line(&text, "はなこ"), Some("はなこ /花子;名/"));
        assert_eq!(line(&text, "たろう"), Some("たろう /太郎;名/"));
        assert_eq!(line(&text, "じろう"), Some("じろう /次郎;名/"));
        assert_eq!(line(&text, "こうふ"), Some("こうふ /甲府;姓;地名/"));
        assert_eq!(line(&text, "こう"), Some("こう /甲;姓/"));
        assert_eq!(line(&text, "おつ"), Some("おつ /甲;姓/乙;姓/"));
        assert_eq!(line(&text, "まりー"), Some("まりー /マリー;名/"));
        assert_eq!(
            line(&text, "やまだ・はなこ"),
            Some("やまだ・はなこ /山田花子;人名/")
        );
        assert_eq!(
            line(&text, "じゅんばん"),
            Some("じゅんばん /優先;名/普通;名/")
        );
        assert_eq!(line(&text, "さとう"), Some("さとう /佐藤;姓;名;人名/"));
        assert_eq!(line(&text, "ぼう"), Some("ぼう /某;人名/"));
        assert_eq!(line(&text, "とうきょう"), Some("とうきょう /東京;地名/"));
        assert_eq!(line(&text, "とよた"), Some("とよた /トヨタ;会社/"));
        assert_eq!(
            line(&text, "しんじゅくえき"),
            Some("しんじゅくえき /新宿駅;駅/")
        );
        assert_eq!(line(&text, "こくれん"), Some("こくれん /国連;組織/"));
        assert_eq!(line(&text, "えんぴつ"), Some("えんぴつ /鉛筆;製品/"));
        assert_eq!(line(&text, "ゆきぐに"), Some("ゆきぐに /雪国;作品/"));
        assert_eq!(
            line(&text, "すねーくがわ"),
            Some("すねーくがわ /スネーク川;地名/")
        );
        assert_eq!(
            line(&text, "しゃらんきゅー"),
            Some("しゃらんきゅー /シャ乱Ｑ;作品/")
        );
        assert_eq!(line(&text, "ANC"), Some("ANC /ＡＮＣ;組織/"));
        assert_eq!(line(&text, "HUGO"), Some("HUGO /ＨＵＧＯ;人名/"));
        assert_eq!(line(&text, "ひゅーご"), Some("ひゅーご /ＨＵＧＯ;人名/"));
        assert_eq!(line(&text, "3M"), Some("3M /３Ｍ;会社/"));
        assert_eq!(line(&text, "すりーえむ"), Some("すりーえむ /３Ｍ;会社/"));
        assert_eq!(line(&text, "ISO"), Some("ISO /ＩＳＯ;組織/"));
        assert_eq!(line(&text, "あいそ"), Some("あいそ /ＩＳＯ;組織/"));
        assert!(line(&text, "かなだけ").is_none());
        assert!(line(&text, "ふめい").is_none());
        assert!(line(&text, "じぇーえーひろしまびょういんまええき").is_none());
        assert!(line(&text, "ぴーしーえんじん").is_none());
        assert!(line(&text, "えーえぬしー").is_none());
        assert!(line(&text, "あいえすおー").is_none());
        assert!(line(&text, "きゅーたろう").is_none());
        assert!(!text.contains("Yamada"));
        assert!(text.lines().all(|line| {
            let midashi = line
                .split_once(' ')
                .map(|(midashi, _)| midashi)
                .unwrap_or(line);
            let hiragana = midashi.chars().all(|character| {
                ('ぁ'..='ゖ').contains(&character) || character == 'ー' || character == '・'
            });
            let ascii = midashi
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
                && midashi
                    .chars()
                    .any(|character| character.is_ascii_alphabetic());
            hiragana || ascii
        }));
    }

    #[test]
    fn truncated_document_leaves_no_output_file() {
        let directory = std::env::temp_dir().join(format!("jmnedict-jisyo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let input = directory.join("JMnedict.xml");
        fs::write(&input, "<JMnedict><entry><k_ele><keb>山田</keb>").unwrap();
        let output = directory.join("SKK-JISYO.jmnedict");
        let error = write_dictionary(&input, &output).unwrap_err();
        assert!(error.to_string().contains("truncated"));
        assert!(!output.exists());
        assert!(!partial_path(&output).exists());
        let _ = fs::remove_dir_all(&directory);
    }
}
