use std::collections::{BTreeMap, HashMap};

use super::entry::Entry;
use super::exception::Exception;
use super::mapping::{to_index, Mapping};
use super::rules::{canonicalize_modern, convert, restore_modern, RuleError};
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

            let canonical_reading = canonicalize_modern(&entry.reading);
            let alternatives = segmentations(candidate, &canonical_reading, &index);
            match alternatives.as_slice() {
                [] => output.report.push(ReportRow {
                    status: "missing",
                    candidate: candidate.clone(),
                    reading: entry.reading.clone(),
                    detail: "no unique segmentation".to_string(),
                }),
                [segments] => match convert(candidate, &entry.reading, segments) {
                    Ok(conversion) => {
                        if restore_modern(&conversion).is_some() {
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
                        } else {
                            output.report.push(ReportRow {
                                status: "missing",
                                candidate: candidate.clone(),
                                reading: entry.reading.clone(),
                                detail: "restore check failed".to_string(),
                            });
                        }
                    }
                    Err(RuleError::EmptyInput) => output.report.push(ReportRow {
                        status: "missing",
                        candidate: candidate.clone(),
                        reading: entry.reading.clone(),
                        detail: "empty input".to_string(),
                    }),
                    Err(RuleError::UnrestorableSokuon) => output.report.push(ReportRow {
                        status: "missing",
                        candidate: candidate.clone(),
                        reading: entry.reading.clone(),
                        detail: "unrestorable sokuon".to_string(),
                    }),
                },
                alternatives => output.report.push(ReportRow {
                    status: "ambiguous",
                    candidate: candidate.clone(),
                    reading: entry.reading.clone(),
                    detail: ambiguity_detail(alternatives),
                }),
            }
        }
    }

    for candidates in output.entries.values_mut() {
        candidates.dedup();
    }
    output
}

fn is_kanji(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2EBEF}'
            | '\u{30000}'..='\u{3134F}'
    )
}

pub fn is_jukugo_candidate(candidate: &str) -> bool {
    candidate.chars().count() > 1 && candidate.chars().all(is_kanji)
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
