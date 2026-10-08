use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::jion::on_reading::katakana_to_hiragana;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid Unihan line {}: {}",
            self.line, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// Readings from `kJapanese`, keyed by hiragana midashi.
/// Characters on a line are ordered by code point.
pub fn parse(text: &str) -> Result<BTreeMap<String, BTreeSet<char>>, ParseError> {
    let mut entries: BTreeMap<String, BTreeSet<char>> = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let Some(code) = fields.next() else {
            return Err(tab_error(line_number));
        };
        let Some(property) = fields.next() else {
            return Err(tab_error(line_number));
        };
        let Some(value) = fields.next() else {
            return Err(tab_error(line_number));
        };
        if fields.next().is_some() {
            return Err(tab_error(line_number));
        }
        if property != "kJapanese" {
            continue;
        }
        let character = code_point(code).map_err(|message| ParseError {
            line: line_number,
            message,
        })?;
        for token in value.split_whitespace() {
            let reading = katakana_to_hiragana(token);
            if is_kept_reading(&reading) {
                entries.entry(reading).or_default().insert(character);
            }
        }
    }
    Ok(entries)
}

pub fn write_dictionary(readings: &Path, output: &Path) -> io::Result<()> {
    let text = fs::read_to_string(readings)?;
    let entries = parse(&text).map_err(|error| io::Error::other(error.to_string()))?;
    let partial = partial_path(output);
    let result = (|| {
        let file = File::create(&partial)?;
        let mut writer = BufWriter::new(file);
        write_entries(&mut writer, &entries)?;
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

pub fn format_entries(entries: &BTreeMap<String, BTreeSet<char>>) -> String {
    let mut buffer = Vec::new();
    write_entries(&mut buffer, entries).expect("writing to a string buffer");
    String::from_utf8(buffer).expect("dictionary lines are UTF-8")
}

fn write_entries(
    writer: &mut impl Write,
    entries: &BTreeMap<String, BTreeSet<char>>,
) -> io::Result<()> {
    for (reading, characters) in entries {
        write!(writer, "{reading} /")?;
        for character in characters {
            write!(writer, "{character}/")?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    path.with_file_name(name)
}

fn tab_error(line: usize) -> ParseError {
    ParseError {
        line,
        message: "expected three tab-separated fields".to_string(),
    }
}

fn code_point(field: &str) -> Result<char, String> {
    let Some(hex) = field.strip_prefix("U+") else {
        return Err(format!("invalid code point {field}"));
    };
    let value = u32::from_str_radix(hex, 16).map_err(|_| format!("invalid code point {field}"))?;
    char::from_u32(value).ok_or_else(|| format!("invalid code point {field}"))
}

fn is_kept_reading(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ('ぁ'..='ゖ').contains(&ch) || ch == 'ー')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        include_str!("../tests/fixtures/unihan_readings.txt")
    }

    #[test]
    fn builds_readings_from_kjapanese() {
        let text = format_entries(&parse(fixture()).unwrap());
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(
            lines,
            vec![
                "あかるい /明/",
                "あきらか /明/",
                "いち /一/",
                "いつ /一/",
                "かい /峡/",
                "きょう /峡/",
                "ぎょう /硤/",
                "こう /峡/硤/",
                "はざま /峡/",
                "ひと /一/",
                "ひとつ /一/",
            ]
        );
        assert!(!text.contains("ひとt"));
        assert!(!text.contains('二'));
    }

    #[test]
    fn keeps_a_reading_that_contains_a_long_vowel_mark() {
        let entries = parse("U+7C73\tkJapanese\tメートル\n").unwrap();
        assert_eq!(format_entries(&entries), "めーとる /米/\n");
    }

    #[test]
    fn rejects_a_line_that_is_not_three_fields() {
        let error = parse("U+4E00 kJapanese イチ\n").unwrap_err();
        assert_eq!(error.line, 1);
        assert_eq!(error.message, "expected three tab-separated fields");
    }

    #[test]
    fn bad_line_leaves_no_output_file() {
        let directory =
            std::env::temp_dir().join(format!("unihan-bad-line-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let input = directory.join("readings.txt");
        let output = directory.join("SKK-JISYO.unihan");
        fs::write(&input, "U+4E00\tkJapanese\tイチ\nshort\n").unwrap();

        let error = write_dictionary(&input, &output).unwrap_err();
        assert!(error.to_string().contains("invalid Unihan line 2"));
        assert!(!output.exists());
        assert!(!partial_path(&output).exists());

        let _ = fs::remove_dir_all(&directory);
    }
}
