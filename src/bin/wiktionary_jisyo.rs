use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use xml_xtract::articles;
use xml_xtract::seikana::entry::Entry;
use xml_xtract::seikana::exception::{read_exceptions as read_exception_file, Exception};
use xml_xtract::seikana::mapping::{read_mapping, Mapping};
use xml_xtract::seikana::wiktionary::{
    kanji_word_entries, parse_japanese_page, to_entry, wiktionary_entries, EntrySource,
    JapanesePage, WiktionaryEntry,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_temp(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("xml-xtract-{name}"));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn parses_all_required_options() {
        let options = Options {
            xml: "dump.xml".into(),
            mapping: "mapping.tsv".into(),
            output: "out".into(),
            exceptions: Some("exceptions.tsv".into()),
            report: Some("report.tsv".into()),
            source: Source::Kanji,
            seikana_output: None,
            dry_run: true,
        };
        assert_eq!(options.source, Source::Kanji);
        assert!(options.dry_run);
    }

    #[test]
    fn parses_source_names() {
        assert_eq!(Source::from_name("all"), Some(Source::All));
        assert_eq!(Source::from_name("kanji"), Some(Source::Kanji));
        assert_eq!(Source::from_name("wago"), Some(Source::Wago));
        assert_eq!(Source::from_name("unknown"), None);
    }

    #[test]
    fn separates_okuri_sections() {
        let entries = vec![Entry::new("あるk", "歩"), Entry::new("がくかう", "学校")];
        let dictionary = Dictionary::from_entries(&entries);
        assert_eq!(dictionary.okuri_ari, vec![Entry::new("あるk", "歩")]);
        assert_eq!(dictionary.okuri_nasi, vec![Entry::new("がくかう", "学校")]);
    }

    #[test]
    fn reads_optional_exceptions() {
        let path = write_temp("no-exceptions", "");
        assert!(read_exceptions(&None).unwrap().is_empty());
        assert!(read_exceptions(&Some(path)).unwrap().is_empty());
    }

    #[test]
    fn writes_report() {
        let path = write_temp("report", "");
        let report = Report {
            pages: 1,
            entries: 2,
            kanji_entries: 1,
            wago_entries: 1,
            excluded: 0,
            invalid_pages: 0,
            kanjitabs: 0,
            noun_readings: 0,
        };
        write_report(path.clone(), &report).unwrap();
        let contents = fs::read_to_string(path).unwrap();
        assert!(contents.contains("entries\t2"));
    }

    #[test]
    fn converts_kanji_pages() {
        let page = parse_japanese_page(
            "学校",
            "=={{ja}}==\n{{ja-kanjitab|がく|こう}}\n{{ja-noun|がっこう}}",
        );
        let mappings = vec![
            Mapping {
                kanji: "学".into(),
                modern: "がく".into(),
                historical: "がく".into(),
            },
            Mapping {
                kanji: "校".into(),
                modern: "こう".into(),
                historical: "かう".into(),
            },
        ];
        let (_, converted, seikana) = page_entries(&page, &mappings, Source::All);
        assert_eq!(converted, vec![Entry::new("がくかう", "学校")]);
        assert_eq!(seikana, converted);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    All,
    Kanji,
    Wago,
}

impl Source {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "all" => Some(Self::All),
            "kanji" => Some(Self::Kanji),
            "wago" => Some(Self::Wago),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Options {
    xml: PathBuf,
    mapping: PathBuf,
    output: PathBuf,
    exceptions: Option<PathBuf>,
    report: Option<PathBuf>,
    source: Source,
    seikana_output: Option<PathBuf>,
    dry_run: bool,
}

#[derive(Default)]
struct Report {
    pages: usize,
    entries: usize,
    kanji_entries: usize,
    wago_entries: usize,
    excluded: usize,
    invalid_pages: usize,
    kanjitabs: usize,
    noun_readings: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Dictionary {
    okuri_ari: Vec<Entry>,
    okuri_nasi: Vec<Entry>,
}

impl Dictionary {
    fn from_entries(entries: &[Entry]) -> Self {
        let mut dictionary = Self::default();
        for entry in entries {
            if entry
                .reading
                .chars()
                .last()
                .is_some_and(|ch| ch.is_ascii_alphabetic())
            {
                dictionary.okuri_ari.push(entry.clone());
            } else {
                dictionary.okuri_nasi.push(entry.clone());
            }
        }
        dictionary
    }

    fn write_to(&self, path: &PathBuf) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, ";; okuri-ari entries.")?;
        for entry in &self.okuri_ari {
            write!(writer, "{}", entry.to_line())?;
        }
        writeln!(writer, ";; okuri-nasi entries.")?;
        for entry in &self.okuri_nasi {
            write!(writer, "{}", entry.to_line())?;
        }
        writer.flush()
    }
}

fn usage(code: ExitCode) -> ExitCode {
    eprintln!(
        "Usage: wiktionary_jisyo --xml XML --mapping MAPPING --output OUTPUT \
         [--exceptions EXCEPTIONS] [--report REPORT] [--source all|kanji|wago] \
         [--dry-run]"
    );
    code
}

fn parse_args() -> Result<Options, ExitCode> {
    let mut args = env::args().skip(1);
    let mut values = HashMap::new();
    let mut source = Source::All;
    let mut dry_run = false;
    let mut seikana_output = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--seikana-output" => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                seikana_output = Some(PathBuf::from(value));
            }
            "--source" => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                source = Source::from_name(&value).ok_or_else(|| usage(ExitCode::FAILURE))?;
            }
            option if option.starts_with("--") => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                values.insert(option.to_string(), value);
            }
            _ => return Err(usage(ExitCode::FAILURE)),
        }
    }

    let required = |name: &str| {
        values
            .get(name)
            .cloned()
            .map(PathBuf::from)
            .ok_or_else(|| usage(ExitCode::FAILURE))
    };
    Ok(Options {
        xml: required("--xml")?,
        mapping: required("--mapping")?,
        output: required("--output")?,
        exceptions: values.get("--exceptions").map(PathBuf::from),
        report: values.get("--report").map(PathBuf::from),
        source,
        seikana_output,
        dry_run,
    })
}

fn read_exceptions(path: &Option<PathBuf>) -> io::Result<Vec<Exception>> {
    match path {
        Some(path) => read_exception_file(path),
        None => Ok(Vec::new()),
    }
}

fn exception_matches(exception: &Exception, page: &JapanesePage, modern: &str) -> bool {
    exception.candidate() == page.title && exception.modern() == modern
}

fn page_entries(
    page: &JapanesePage,
    mappings: &[Mapping],
    source: Source,
) -> (Vec<WiktionaryEntry>, Vec<Entry>, Vec<Entry>) {
    let mut words = Vec::new();
    let mut seikana_entries = Vec::new();
    if source != Source::Wago {
        for entry in kanji_word_entries(page, mappings).0 {
            words.push(WiktionaryEntry {
                reading: entry.reading.clone(),
                candidate: entry.candidates.join("/"),
                source: EntrySource::KanjiWord,
            });
            seikana_entries.push(entry);
        }
    }
    if source != Source::Kanji {
        words.extend(wiktionary_entries(page));
    }
    let entries = words.iter().map(to_entry).collect();
    (words, entries, seikana_entries)
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(code) => return code,
    };

    if let Err(error) = run(options) {
        eprintln!("wiktionary_jisyo: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(options: Options) -> io::Result<()> {
    let mappings = read_mapping(&options.mapping)?;
    let exceptions = read_exceptions(&options.exceptions)?;
    let mut output_entries = Vec::new();
    let mut seikana_entries = Vec::new();
    let mut report = Report::default();

    for page in articles(&options.xml) {
        let Some(text) = page.japanese_text() else {
            report.invalid_pages += 1;
            continue;
        };
        report.pages += 1;
        let page = parse_japanese_page(page.title.as_str(), text);
        report.kanjitabs += page.kanjitabs.len();
        report.noun_readings += page.noun_readings.len();
        let (entries, converted, page_seikana_entries) =
            page_entries(&page, &mappings, options.source);
        let page_seikana_entries = page_seikana_entries
            .into_iter()
            .filter(|entry| {
                !exceptions
                    .iter()
                    .any(|exception| exception_matches(exception, &page, &entry.reading))
            })
            .collect::<Vec<_>>();
        seikana_entries.extend(page_seikana_entries);
        for entry in converted {
            if exceptions
                .iter()
                .any(|exception| exception_matches(exception, &page, &entry.reading))
            {
                report.excluded += 1;
                continue;
            }
            output_entries.push(entry);
        }
        report.kanji_entries += entries
            .iter()
            .filter(|entry| entry.source == EntrySource::KanjiWord)
            .count();
        report.wago_entries += entries
            .iter()
            .filter(|entry| {
                entry.source == EntrySource::WagoOkuri || entry.source == EntrySource::Suru
            })
            .count();
    }

    output_entries.sort_by(|left, right| {
        left.reading
            .cmp(&right.reading)
            .then_with(|| left.candidates.join("/").cmp(&right.candidates.join("/")))
    });
    output_entries.dedup();
    report.entries = output_entries.len();
    let dictionary = Dictionary::from_entries(&output_entries);
    if !options.dry_run {
        dictionary.write_to(&options.output)?;
    }

    seikana_entries.sort_by(|left, right| {
        left.reading
            .cmp(&right.reading)
            .then_with(|| left.candidates.join("/").cmp(&right.candidates.join("/")))
    });
    seikana_entries.dedup();
    let seikana_dictionary = Dictionary::from_entries(&seikana_entries);
    if let Some(path) = options.seikana_output {
        seikana_dictionary.write_to(&path)?;
    }

    if let Some(report_path) = options.report {
        write_report(report_path, &report)?;
    }

    Ok(())
}

fn write_report(path: PathBuf, report: &Report) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "metric\tcount")?;
    writeln!(writer, "pages\t{}", report.pages)?;
    writeln!(writer, "entries\t{}", report.entries)?;
    writeln!(writer, "kanji_entries\t{}", report.kanji_entries)?;
    writeln!(writer, "wago_entries\t{}", report.wago_entries)?;
    writeln!(writer, "excluded\t{}", report.excluded)?;
    writeln!(writer, "invalid_pages\t{}", report.invalid_pages)?;
    writeln!(writer, "kanjitabs\t{}", report.kanjitabs)?;
    writeln!(writer, "noun_readings\t{}", report.noun_readings)?;
    writer.flush()
}
