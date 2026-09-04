use once_cell::sync::OnceCell;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use wana_kana::ConvertJapanese;
use xml_xtract::seikana::mapping::Mapping;
use xml_xtract::{kanji_articles, model::*};

static EXTRACT_REGEX: OnceCell<Regex> = OnceCell::new();
static ON_READING_REGEX: OnceCell<Regex> = OnceCell::new();

fn extract_on(
    mut buffer: impl Write,
    mut mapping_buffer: impl Write,
    area: &str,
    page: &Page,
) -> io::Result<()> {
    if area.contains("無し") {
        println!("no on in {}", page.title);
        return Ok(());
    }
    let mut at_least_one = false;
    for cap in EXTRACT_REGEX.get().unwrap().captures_iter(area) {
        at_least_one = true;
        let gen = cap.get(1).unwrap().as_str();
        let sei = cap.get(2).unwrap().as_str();
        let modern = gen.to_hiragana();
        let historical = sei.to_hiragana();
        writeln!(buffer, "{} /{}/", historical, page.title)?;
        writeln!(buffer, "{} /{};{}/", modern, page.title, sei)?;
        writeln!(mapping_buffer, "{}\t{}\t{}", page.title, modern, historical)?;
    }

    if !at_least_one {
        println!("no seikana in {}\n{}", page.title, area)
    }
    Ok(())
}

fn is_on_reading(reading: &str) -> bool {
    !reading.is_empty() && reading.chars().all(|c| matches!(c, 'ァ'..='ヶ'))
}

fn build_historical_inference(mappings: &[Mapping]) -> HashMap<String, String> {
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for mapping in mappings {
        *counts
            .entry((mapping.modern.clone(), mapping.historical.clone()))
            .or_insert(0) += 1;
    }

    let mut by_modern: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for ((modern, historical), count) in counts {
        by_modern
            .entry(modern)
            .or_default()
            .push((historical, count));
    }

    let mut result = HashMap::new();
    for (modern, historicals) in by_modern {
        let total: usize = historicals.iter().map(|(_, count)| count).sum();
        let Some((historical, _)) = historicals
            .into_iter()
            .max_by_key(|(historical, count)| (*count, historical.clone()))
        else {
            continue;
        };
        // Frequency-based inference is only a fallback. Avoid noisy one-off
        // Wiktionary data by requiring broad support for a modern reading.
        if total < 5 {
            continue;
        }
        result.insert(modern, historical);
    }
    result
}

fn main() -> io::Result<()> {
    let on_area_regex = Regex::new(r"(?m)^\s*\*\s*(?:\[\[)?音読(?:み)?(?:\]\])?\s*[::]\s*([^\n]+)")
        .expect("internal error: invalid regex");
    let ids_file = env::args().nth(1).expect("Usage: IDs XML");
    let xml_file = env::args().nth(2).expect("Usage: IDs XML");
    let output = File::create("tmp.seikana")?;
    let mapping_output = File::create("kanji_readings.tsv")?;
    let mut buffer = BufWriter::new(output);
    let mut mapping_buffer = BufWriter::new(mapping_output);

    let kanji_template_regex =
        Regex::new(r"\{\{ja-kanji\|([^\n]+?)\}\}").expect("internal error: invalid regex");
    ON_READING_REGEX
        .set(
            Regex::new(r"(?:呉音|漢音|唐音|宋音|慣用音)\s*=\s*([^|;}]+)")
                .expect("internal error: invalid regex"),
        )
        .unwrap();
    EXTRACT_REGEX
        .set(
            Regex::new(
                r"(?:\[\[)?([\p{Hiragana}\p{Katakana}]+)(?:\]\])?\s*(?:\(|（)(?:\[\[)?([\p{Hiragana}\p{Katakana}]+)(?:\]\])?(?:\)|）)",
            )
            .expect("internal error: invalid regex"),
        )
        .unwrap();

    let known_mappings = kanji_articles(ids_file.clone(), xml_file.clone())
        .filter_map(|page| {
            let cap = kanji_template_regex.captures(&page.revision.text)?;
            let params = cap.get(1)?.as_str();
            let mut mappings = Vec::new();
            for reading in ON_READING_REGEX.get().unwrap().captures_iter(params) {
                for pair in reading[1].split(',') {
                    let mut parts = pair.split('<');
                    let modern = parts.next().unwrap_or("").trim();
                    let historical = parts.next().unwrap_or("").trim();
                    let historical = if historical.is_empty() {
                        modern
                    } else {
                        historical
                    };
                    if is_on_reading(modern) && is_on_reading(historical) {
                        let mapping = Mapping {
                            kanji: page.title.clone(),
                            modern: modern.to_hiragana(),
                            historical: historical.to_hiragana(),
                        };
                        if !mappings.contains(&mapping) {
                            mappings.push(mapping);
                        }
                    }
                }
            }
            Some(mappings)
        })
        .flatten()
        .collect::<Vec<_>>();
    let inference = build_historical_inference(&known_mappings);

    for page in kanji_articles(ids_file, xml_file) {
        let mut processed = false;
        if let Some(cap) = kanji_template_regex.captures(&page.revision.text) {
            let params = cap.get(1).unwrap().as_str();
            let mut modern_readings = Vec::new();
            let mut mappings = Vec::new();
            let mut historical_readings = Vec::new();
            let mut annotations = Vec::new();

            for reading in ON_READING_REGEX.get().unwrap().captures_iter(params) {
                processed = true;
                for pair in reading[1].split(',') {
                    let mut parts = pair.split('<');
                    let modern = parts.next().unwrap_or("").trim();
                    let historical = parts.next().unwrap_or("").trim();
                    let historical = if historical.is_empty() {
                        modern
                    } else {
                        historical
                    };
                    if is_on_reading(modern) {
                        let modern = modern.to_hiragana();
                        if !modern_readings.contains(&modern) {
                            modern_readings.push(modern.clone());
                        }
                    }
                    if is_on_reading(historical) {
                        let historical = historical.to_hiragana();
                        if !historical_readings.contains(&historical) {
                            historical_readings.push(historical.clone());
                        }
                        if is_on_reading(modern) {
                            let mapping = Mapping {
                                kanji: page.title.clone(),
                                modern: modern.to_hiragana(),
                                historical: historical.clone(),
                            };
                            if !mappings.contains(&mapping) {
                                mappings.push(mapping);
                            }
                        }
                        let annotation = historical.to_katakana();
                        if !annotations.contains(&annotation) {
                            annotations.push(annotation);
                        }
                    } else if is_on_reading(modern) {
                        let modern_hiragana = modern.to_hiragana();
                        if let Some(historical) = inference.get(modern_hiragana.as_str()).cloned() {
                            let mapping = Mapping {
                                kanji: page.title.clone(),
                                modern: modern_hiragana,
                                historical,
                            };
                            if !mappings.contains(&mapping) {
                                mappings.push(mapping);
                            }
                        }
                    }
                }
            }

            for mapping in &mappings {
                writeln!(
                    mapping_buffer,
                    "{}\t{}\t{}",
                    mapping.kanji, mapping.modern, mapping.historical
                )?;
            }

            for historical in &historical_readings {
                writeln!(buffer, "{} /{}/", historical, page.title)?;
            }

            if !annotations.is_empty() {
                for modern in &modern_readings {
                    writeln!(
                        buffer,
                        "{} /{};{}/",
                        modern,
                        page.title,
                        annotations.join(",")
                    )?;
                }
            }
        }

        if !processed {
            for cap in on_area_regex.captures_iter(&page.revision.text) {
                processed = true;
                let area = cap.get(1).unwrap().as_str();
                extract_on(&mut buffer, &mut mapping_buffer, area, &page)?
            }
        }

        if !processed {
            println!("{}: no match", page.title);
            if ["呉音", "漢音", "音読", "訓読"]
                .iter()
                .any(|p| page.revision.text.contains(p))
            {
                println!(
                    "Hole of regex in {} {}:\n{}",
                    page.id, page.title, page.revision.text
                )
            }
        }
    }
    Ok(())
}
