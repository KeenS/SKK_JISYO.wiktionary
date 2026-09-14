use super::entry::Entry;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

pub fn read_jisyo(path: impl AsRef<Path>) -> io::Result<Vec<Entry>> {
    let file = File::open(path)?;
    let mut entries = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if let Some(entry) = parse_line(&line) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

pub fn parse_line(line: &str) -> Option<Entry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') {
        return None;
    }
    let (reading, candidates) = line.split_once(" /")?;
    let candidates = candidates.strip_suffix('/')?;
    let candidate_annotations = candidates
        .split('/')
        .filter(|candidate| !candidate.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if reading.is_empty() || candidates.is_empty() {
        return None;
    }
    Some(Entry::from_candidate_annotations(
        reading,
        candidate_annotations,
    ))
}

pub fn write_jisyo(path: impl AsRef<Path>, entries: &[Entry]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for entry in entries {
        write!(writer, "{}", entry.to_line())?;
    }
    writer.flush()
}
