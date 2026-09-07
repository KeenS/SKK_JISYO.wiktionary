use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use xml_xtract::jion::mapping::Mapping;
use xml_xtract::jion::on_reading::{
    build_historical_inference, kanji_template_params, parse_on_readings, resolve_readings,
    OnReading,
};
use xml_xtract::{kanji_articles, model::Page};

struct Outputs {
    dictionary: BufWriter<File>,
    mappings: BufWriter<File>,
}

impl Outputs {
    fn create(dictionary: impl AsRef<Path>, mappings: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            dictionary: BufWriter::new(File::create(dictionary)?),
            mappings: BufWriter::new(File::create(mappings)?),
        })
    }

    fn write_kanji_page(
        &mut self,
        page: &Page,
        readings: &[OnReading],
        inference: &HashMap<String, String>,
    ) -> io::Result<()> {
        let mappings = resolve_readings(readings, inference);
        for mapping in &mappings {
            for historical in mapping.historical_entries() {
                writeln!(self.dictionary, "{historical} /{}/", page.title)?;
            }
        }
        for mapping in &mappings {
            let annotations = mapping.annotations();
            if annotations.is_empty() {
                continue;
            }
            writeln!(
                self.dictionary,
                "{} /{};{}/",
                mapping.modern,
                page.title,
                annotations.join(";")
            )?;
        }
        for mapping in &mappings {
            for historical in &mapping.historicals {
                writeln!(
                    self.mappings,
                    "{}\t{}\t{}",
                    page.title, mapping.modern, historical
                )?;
            }
        }
        Ok(())
    }
}

fn parse_kanji_page(
    page: &Page,
    inference: &HashMap<String, String>,
    outputs: &mut Outputs,
) -> io::Result<()> {
    let Some(params) = kanji_template_params(&page.revision.text) else {
        return Ok(());
    };
    let readings = parse_on_readings(params);
    outputs.write_kanji_page(page, &readings, inference)
}

fn main() -> io::Result<()> {
    let ids_file = env::args().nth(1).expect("Usage: IDs XML");
    let xml_file = env::args().nth(2).expect("Usage: IDs XML");
    let mut outputs = Outputs::create("tmp.jion", "kanji_readings.tsv")?;

    let explicit_mappings = kanji_articles(&ids_file, &xml_file)
        .filter_map(|page| {
            let params = kanji_template_params(&page.revision.text)?;
            Some(parse_on_readings(params))
        })
        .flatten()
        .filter_map(|reading| {
            reading
                .historical
                .filter(|historical| *historical != reading.modern)
                .map(|historical| Mapping {
                    kanji: String::new(),
                    modern: reading.modern,
                    historical,
                })
        })
        .collect::<Vec<_>>();
    let inference = build_historical_inference(&explicit_mappings);

    for page in kanji_articles(&ids_file, &xml_file) {
        parse_kanji_page(&page, &inference, &mut outputs)?;
    }
    outputs.dictionary.flush()?;
    outputs.mappings.flush()?;
    Ok(())
}
