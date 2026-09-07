use super::mapping::{to_index, Mapping};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub kanji: String,
    pub modern: String,
    pub historical: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segmentation {
    Unique(Vec<Segment>),
    Ambiguous(Vec<Vec<Segment>>),
    Missing,
}

/// Segmentations are unique by their modern reading sequence. Different historical
/// spellings for the same modern sequence are not ambiguity at this stage.
pub fn segmentations(
    candidate: &str,
    reading: &str,
    index: &HashMap<String, Vec<Mapping>>,
) -> Vec<Vec<Segment>> {
    let kanji: Vec<char> = candidate.chars().collect();
    if kanji.is_empty() || reading.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut current = Vec::new();
    search(&kanji, reading, index, &mut current, &mut result);
    result.sort_by_key(|segments| {
        (
            segments.len(),
            segments
                .iter()
                .map(|segment| segment.modern.clone())
                .collect::<Vec<_>>(),
        )
    });
    result.dedup_by(|a, b| {
        a.len() == b.len()
            && a.iter()
                .zip(b.iter())
                .all(|(left, right)| left.kanji == right.kanji && left.modern == right.modern)
    });
    result
}

fn search(
    kanji: &[char],
    reading: &str,
    index: &HashMap<String, Vec<Mapping>>,
    current: &mut Vec<Segment>,
    result: &mut Vec<Vec<Segment>>,
) {
    let Some((&first, rest_kanji)) = kanji.split_first() else {
        if reading.is_empty() {
            result.push(current.clone());
        }
        return;
    };

    let Some(mappings) = index.get(&first.to_string()) else {
        return;
    };
    for mapping in mappings {
        let Some(rest_reading) = reading.strip_prefix(mapping.modern.as_str()) else {
            continue;
        };
        current.push(Segment {
            kanji: mapping.kanji.clone(),
            modern: mapping.modern.clone(),
            historical: mapping.historical.clone(),
        });
        search(rest_kanji, rest_reading, index, current, result);
        current.pop();
    }
}

pub fn segment(candidate: &str, reading: &str, mappings: &[Mapping]) -> Segmentation {
    let index = to_index(mappings);
    let mut segmentations = segmentations(candidate, reading, &index);
    match segmentations.len() {
        0 => Segmentation::Missing,
        1 => Segmentation::Unique(segmentations.remove(0)),
        _ => Segmentation::Ambiguous(segmentations),
    }
}
