use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;
use xml_xtract::jion::mapping::Mapping;
use xml_xtract::jion::on_reading::{
    build_historical_inference, kanji_template_params, parse_on_readings, resolve_readings,
    OnReading,
};
use xml_xtract::kanji_articles;

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

    fn write_kanji(
        &mut self,
        title: &str,
        readings: &[OnReading],
        inference: &HashMap<String, String>,
    ) -> io::Result<()> {
        let mappings = resolve_readings(readings, inference);
        for mapping in &mappings {
            for historical in mapping.historical_entries() {
                writeln!(self.dictionary, "{historical} /{title}/")?;
            }
        }
        for mapping in &mappings {
            let annotations = mapping.annotations();
            if annotations.is_empty() {
                continue;
            }
            writeln!(
                self.dictionary,
                "{} /{title};{}/",
                mapping.modern,
                annotations.join(";")
            )?;
        }
        for mapping in &mappings {
            for historical in &mapping.historicals {
                writeln!(self.mappings, "{title}\t{}\t{}", mapping.modern, historical)?;
            }
        }
        Ok(())
    }
}

fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    path.with_file_name(name)
}

fn usage() -> io::Error {
    eprintln!("Usage: jion IDS XML [DICTIONARY [MAPPING]]");
    io::Error::new(io::ErrorKind::InvalidInput, "missing arguments")
}

fn generate(ids_file: &str, xml_file: &str, dictionary: &Path, mapping: &Path) -> io::Result<()> {
    let dictionary_partial = partial_path(dictionary);
    let mapping_partial = partial_path(mapping);
    let result = (|| {
        let mut pages = kanji_articles(ids_file, xml_file)?;
        let mut collected = Vec::new();
        for page in pages.by_ref() {
            let page = page?;
            if let Some(params) = kanji_template_params(&page.revision.text) {
                collected.push((page.title, params.to_string()));
            }
        }
        if pages.skipped_pages() > 0 {
            eprintln!("jion: skipped {} pages", pages.skipped_pages());
        }

        let explicit_mappings = collected
            .iter()
            .flat_map(|(_, params)| parse_on_readings(params))
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
        let mut outputs = Outputs::create(&dictionary_partial, &mapping_partial)?;
        for (title, params) in &collected {
            let readings = parse_on_readings(params);
            outputs.write_kanji(title, &readings, &inference)?;
        }
        outputs.dictionary.flush()?;
        outputs.mappings.flush()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&dictionary_partial);
        let _ = fs::remove_file(&mapping_partial);
        return result;
    }
    fs::rename(&dictionary_partial, dictionary)?;
    fs::rename(&mapping_partial, mapping)?;
    Ok(())
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(ids_file) = args.next() else {
        let _ = usage();
        process::exit(2);
    };
    let Some(xml_file) = args.next() else {
        let _ = usage();
        process::exit(2);
    };
    let dictionary = args.next().unwrap_or_else(|| "tmp.jion".to_string());
    let mapping = args
        .next()
        .unwrap_or_else(|| "kanji_readings.tsv".to_string());
    if let Err(error) = generate(
        &ids_file,
        &xml_file,
        Path::new(&dictionary),
        Path::new(&mapping),
    ) {
        eprintln!("jion: {error}");
        process::exit(1);
    }
}
