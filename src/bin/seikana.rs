use once_cell::sync::OnceCell;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use wana_kana::ConvertJapanese;
use xml_xtract::{kanji_articles, model::*};

static EXTRACT_REGEX: OnceCell<Regex> = OnceCell::new();
static ON_READING_REGEX: OnceCell<Regex> = OnceCell::new();

fn extract_on(mut buffer: impl Write, area: &str, page: &Page) -> io::Result<()> {
    if area.contains("無し") {
        println!("no on in {}", page.title);
        return Ok(());
    }
    let mut at_least_one = false;
    for cap in EXTRACT_REGEX.get().unwrap().captures_iter(area) {
        at_least_one = true;
        let gen = cap.get(1).unwrap().as_str();
        let sei = cap.get(2).unwrap().as_str();
        writeln!(buffer, "{} /{}/", sei.to_hiragana(), page.title)?;
        writeln!(buffer, "{} /{};{}/", gen.to_hiragana(), page.title, sei)?;
    }

    if !at_least_one {
        println!("no seikana in {}\n{}", page.title, area)
    }
    Ok(())
}

fn is_on_reading(reading: &str) -> bool {
    !reading.is_empty() && reading.chars().all(|c| matches!(c, 'ァ'..='ヶ'))
}

fn main() -> io::Result<()> {
    let on_area_regex = Regex::new(r"(?m)^\s*\*\s*(?:\[\[)?音読(?:み)?(?:\]\])?\s*[::]\s*([^\n]+)")
        .expect("internal error: invalid regex");
    let ids_file = env::args().nth(1).expect("Usage: IDs XML");
    let xml_file = env::args().nth(2).expect("Usage: IDs XML");
    let output = File::create("tmp.seikana")?;
    let mut buffer = BufWriter::new(output);

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

    for page in kanji_articles(ids_file, xml_file) {
        let mut processed = false;
        if let Some(cap) = kanji_template_regex.captures(&page.revision.text) {
            let params = cap.get(1).unwrap().as_str();
            let mut modern_readings = Vec::new();
            let mut historical_readings = Vec::new();
            let mut annotations = Vec::new();

            for reading in ON_READING_REGEX.get().unwrap().captures_iter(params) {
                processed = true;
                for pair in reading[1].split(',') {
                    let mut parts = pair.split('<');
                    let modern = parts.next().unwrap_or("").trim();
                    let historical = parts.next().unwrap_or("").trim();
                    if is_on_reading(modern) {
                        let modern = modern.to_hiragana();
                        if !modern_readings.contains(&modern) {
                            modern_readings.push(modern);
                        }
                    }
                    if is_on_reading(historical) {
                        let historical = historical.to_hiragana();
                        if !historical_readings.contains(&historical) {
                            historical_readings.push(historical.clone());
                        }
                        let annotation = historical.to_katakana();
                        if !annotations.contains(&annotation) {
                            annotations.push(annotation);
                        }
                    }
                }
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
                extract_on(&mut buffer, area, &page)?
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
