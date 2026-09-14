use std::collections::BTreeMap;

use super::entry::Entry;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Jisyo {
    entries: BTreeMap<String, Entry>,
}

impl Jisyo {
    pub fn add_entry(&mut self, reading: impl Into<String>, candidate: impl Into<String>) {
        let reading = reading.into();
        self.entries
            .entry(reading.clone())
            .or_insert(Entry {
                reading,
                candidates: BTreeMap::new(),
            })
            .add_candidate(candidate);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, reading: &str) -> Option<&Entry> {
        self.entries.get(reading)
    }

    pub fn render(&self) -> String {
        let mut sorted = self.entries.values().collect::<Vec<_>>();
        sorted.sort_by_cached_key(|entry| sort_key(&entry.reading));
        sorted
            .into_iter()
            .map(Entry::to_line)
            .collect::<Vec<_>>()
            .concat()
    }

    pub fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writer.write_all(self.render().as_bytes())
    }
}

fn sort_key(reading: &str) -> Vec<(u8, Option<u32>)> {
    reading
        .chars()
        .map(|ch| {
            let weight = if ch.is_ascii() {
                0
            } else if matches!(ch, 'ー' | '〜' | '・') {
                1
            } else {
                2
            };
            (weight, Some(ch as u32))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_entries_by_reading_and_preserves_candidates() {
        let mut jisyo = Jisyo::default();
        jisyo.add_entry("なかつ", "中津");
        jisyo.add_entry("なかつし", "中津市");
        jisyo.add_entry("なかつ", "中津市");
        assert_eq!(
            jisyo.get("なかつ").unwrap().to_line(),
            "なかつ /中津/中津市/\n"
        );
        assert_eq!(jisyo.render(), "なかつ /中津/中津市/\nなかつし /中津市/\n");
    }
}
