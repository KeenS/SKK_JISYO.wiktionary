use std::collections::{BTreeMap, HashMap};

use super::entry::Entry;
use super::exception::Exception;
use super::mapping::{to_index, Mapping};
use super::rules::{convert, restore_modern, sokuon_expansions, RuleError};
use super::segmentation::{segmentations, Segment};

#[derive(Debug, Clone)]
pub struct ReportRow {
    pub status: &'static str,
    pub candidate: String,
    pub reading: String,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct ConvertedEntries {
    pub entries: BTreeMap<String, Vec<String>>,
    pub report: Vec<ReportRow>,
}

pub fn convert_entries(
    entries: &[Entry],
    mappings: &[Mapping],
    exceptions: &[Exception],
) -> ConvertedEntries {
    let index = to_index(mappings);
    let exceptions = exception_index(exceptions);
    let mut output = ConvertedEntries::default();

    for entry in entries {
        for candidate in &entry.candidates {
            if !is_jukugo_candidate(candidate) {
                continue;
            }

            if let Some(exception) = exceptions
                .get(candidate.as_str())
                .and_then(|by_reading| by_reading.get(entry.reading.as_str()))
            {
                match exception {
                    Exception::Override { historical, .. } => {
                        output
                            .entries
                            .entry(historical.clone())
                            .or_default()
                            .push(candidate.clone());
                        output.report.push(ReportRow {
                            status: "overridden",
                            candidate: candidate.clone(),
                            reading: entry.reading.clone(),
                            detail: historical.clone(),
                        });
                    }
                    Exception::Excluded { .. } => output.report.push(ReportRow {
                        status: "excluded",
                        candidate: candidate.clone(),
                        reading: entry.reading.clone(),
                        detail: "excluded by exception".to_string(),
                    }),
                }
                continue;
            }

            match segment_reading(candidate, &entry.reading, &index) {
                SegmentOutcome::Missing(detail) => output.report.push(ReportRow {
                    status: "missing",
                    candidate: candidate.clone(),
                    reading: entry.reading.clone(),
                    detail: detail.to_string(),
                }),
                SegmentOutcome::Ambiguous(alternatives) => output.report.push(ReportRow {
                    status: "ambiguous",
                    candidate: candidate.clone(),
                    reading: entry.reading.clone(),
                    detail: ambiguity_detail(&alternatives),
                }),
                SegmentOutcome::Converted(conversion) => {
                    output
                        .entries
                        .entry(conversion.historical.clone())
                        .or_default()
                        .push(candidate.clone());
                    output.report.push(ReportRow {
                        status: "converted",
                        candidate: candidate.clone(),
                        reading: entry.reading.clone(),
                        detail: conversion.historical.clone(),
                    });
                }
            }
        }
    }

    for candidates in output.entries.values_mut() {
        candidates.dedup();
    }
    output
}

pub fn is_jukugo_candidate(candidate: &str) -> bool {
    candidate.chars().count() > 1 && candidate.chars().all(crate::model::is_kanji)
}

enum SegmentOutcome {
    Missing(&'static str),
    Ambiguous(Vec<Vec<Segment>>),
    Converted(super::rules::Conversion),
}

fn same_modern_sequence(left: &[Segment], right: &[Segment]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| left.kanji == right.kanji && left.modern == right.modern)
}

/// Segment every 促音 expansion. Keep one result when the successful
/// expansions agree on the kanji and modern reading. Disagreeing splits stay
/// ambiguous.
fn segment_reading(
    candidate: &str,
    reading: &str,
    index: &HashMap<String, Vec<Mapping>>,
) -> SegmentOutcome {
    if reading.is_empty() {
        return SegmentOutcome::Missing("empty input");
    }
    let mut successes: Vec<Vec<Segment>> = Vec::new();
    let mut restore_failed = false;
    for expansion in sokuon_expansions(reading) {
        for segments in segmentations(candidate, &expansion, index) {
            match convert(candidate, reading, &segments) {
                Ok(conversion) if restore_modern(&conversion).is_some() => {
                    if !successes
                        .iter()
                        .any(|existing| same_modern_sequence(existing, &segments))
                    {
                        successes.push(segments);
                    }
                }
                Ok(_) | Err(RuleError::UnrestorableSokuon) => restore_failed = true,
                Err(RuleError::EmptyInput) => {}
            }
        }
    }
    match successes.len() {
        0 if restore_failed => SegmentOutcome::Missing("restore check failed"),
        0 => SegmentOutcome::Missing("no unique segmentation"),
        1 => match convert(candidate, reading, &successes[0]) {
            Ok(conversion) => SegmentOutcome::Converted(conversion),
            Err(RuleError::EmptyInput) => SegmentOutcome::Missing("empty input"),
            Err(RuleError::UnrestorableSokuon) => SegmentOutcome::Missing("unrestorable sokuon"),
        },
        _ => SegmentOutcome::Ambiguous(successes),
    }
}

fn exception_index(exceptions: &[Exception]) -> HashMap<&str, HashMap<&str, &Exception>> {
    let mut index: HashMap<&str, HashMap<&str, &Exception>> = HashMap::new();
    for exception in exceptions {
        index
            .entry(exception.candidate())
            .or_default()
            .insert(exception.modern(), exception);
    }
    index
}

fn ambiguity_detail(alternatives: &[Vec<Segment>]) -> String {
    alternatives
        .iter()
        .map(|segments| {
            segments
                .iter()
                .map(|segment| segment.modern.as_str())
                .collect::<Vec<_>>()
                .join("+")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping(kanji: &str, modern: &str, historical: &str) -> Mapping {
        Mapping {
            kanji: kanji.into(),
            modern: modern.into(),
            historical: historical.into(),
        }
    }

    fn converted(candidate: &str, reading: &str, mappings: &[Mapping]) -> ConvertedEntries {
        let entry = Entry::new(reading, candidate);
        convert_entries(&[entry], mappings, &[])
    }

    #[test]
    fn segments_sokuon_from_ku_tsu_and_chi() {
        let school = converted(
            "学校",
            "がっこう",
            &[mapping("学", "がく", "がく"), mapping("校", "こう", "かう")],
        );
        assert_eq!(school.entries["がっかう"], vec!["学校".to_string()]);

        let diary = converted(
            "日記",
            "にっき",
            &[mapping("日", "にち", "にち"), mapping("記", "き", "き")],
        );
        assert_eq!(diary.entries["にっき"], vec!["日記".to_string()]);

        let marriage = converted(
            "結婚",
            "けっこん",
            &[mapping("結", "けつ", "けつ"), mapping("婚", "こん", "こん")],
        );
        assert_eq!(marriage.entries["けっこん"], vec!["結婚".to_string()]);

        let experiment = converted(
            "実験",
            "じっけん",
            &[mapping("実", "じつ", "じつ"), mapping("験", "けん", "けん")],
        );
        assert_eq!(experiment.entries["じっけん"], vec!["実験".to_string()]);
    }

    #[test]
    fn leaves_disagreeing_sokuon_expansions_ambiguous() {
        let result = converted(
            "日記",
            "にっき",
            &[
                mapping("日", "にち", "にち"),
                mapping("日", "にく", "にく"),
                mapping("記", "き", "き"),
            ],
        );
        assert!(result.entries.is_empty());
        assert_eq!(result.report[0].status, "ambiguous");
    }

    #[test]
    fn treats_extension_kanji_as_jukugo() {
        assert!(crate::model::is_kanji('\u{F900}'));
        assert!(crate::model::is_kanji('\u{20000}'));
        assert!(is_jukugo_candidate("\u{F900}\u{20000}"));
        assert!(!crate::jion::wiktionary::is_symbol("\u{F900}"));
        assert!(!crate::jion::wiktionary::is_symbol("\u{20000}"));
    }
}
