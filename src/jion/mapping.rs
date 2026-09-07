use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    pub kanji: String,
    pub modern: String,
    pub historical: String,
}

pub fn parse_line(line: &str) -> Option<Mapping> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (kanji, rest) = line.split_once('\t')?;
    let (modern, historical) = rest.split_once('\t')?;
    let kanji = kanji.trim();
    let modern = modern.trim();
    let historical = historical.trim();
    if kanji.chars().count() != 1 || modern.is_empty() || historical.is_empty() {
        return None;
    }
    Some(Mapping {
        kanji: kanji.to_string(),
        modern: modern.to_string(),
        historical: historical.to_string(),
    })
}

pub fn read_mapping(path: impl AsRef<Path>) -> io::Result<Vec<Mapping>> {
    let file = File::open(path)?;
    let mut mappings = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if let Some(mapping) = parse_line(&line) {
            mappings.push(mapping);
        }
    }
    Ok(mappings)
}

pub fn write_mapping(path: impl AsRef<Path>, mappings: &[Mapping]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for mapping in mappings {
        writeln!(
            writer,
            "{}\t{}\t{}",
            mapping.kanji, mapping.modern, mapping.historical
        )?;
    }
    writer.flush()
}

pub fn to_index(mappings: &[Mapping]) -> HashMap<String, Vec<Mapping>> {
    let mut index = HashMap::new();
    for mapping in mappings {
        index
            .entry(mapping.kanji.clone())
            .or_insert_with(Vec::new)
            .push(mapping.clone());
    }
    index
}

/// Builds the mapping index once; reuse it for many pages to avoid O(pages * mappings) work.
pub struct MappingIndex(HashMap<String, Vec<Mapping>>);

impl MappingIndex {
    pub fn new(mappings: &[Mapping]) -> Self {
        Self(to_index(mappings))
    }

    pub fn get(&self, kanji: &str) -> Option<&Vec<Mapping>> {
        self.0.get(kanji)
    }
}
