use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use xml_xtract::articles;
use xml_xtract::seikana::entry::Entry;
use xml_xtract::seikana::mapping::{read_mapping, MappingIndex};
use xml_xtract::seikana::wiktionary::{
    kanji_word_entries, parse_japanese_page, to_entry, wiktionary_entries, EntrySource,
    JapanesePage, WiktionaryEntry,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use xml_xtract::seikana::mapping::Mapping;

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
    fn writes_report() {
        let path = write_temp("report", "");
        let report = Report {
            pages: 1,
            entries: 2,
            kanji_entries: 1,
            noun_entries: 0,
            wago_entries: 1,
            seikana_entries: 3,
            shared_entries: 1,
            invalid_pages: 0,
            kanjitabs: 0,
            noun_readings: 0,
        };
        write_report(path.clone(), &report).unwrap();
        let contents = fs::read_to_string(path).unwrap();
        assert!(contents.contains("entries\t2"));
        assert!(contents.contains("seikana_entries\t3"));
        assert!(contents.contains("shared_entries\t1"));
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
        let mappings = MappingIndex::new(&mappings);
        let (_, converted, seikana) = page_entries(&page, &mappings, Source::All);
        assert_eq!(converted, vec![Entry::new("がっこう", "学校")]);
        assert_eq!(seikana, vec![Entry::new("がくかう", "学校")]);
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
    noun_entries: usize,
    wago_entries: usize,
    seikana_entries: usize,
    shared_entries: usize,
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
    fn from_entries<'a>(entries: impl IntoIterator<Item = &'a Entry>) -> Self {
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
         [--report REPORT] [--source all|kanji|wago] \
         [--seikana-output SEIKANA_OUTPUT] [--dry-run]"
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
        report: values.get("--report").map(PathBuf::from),
        source,
        seikana_output,
        dry_run,
    })
}

type EntryKey = (String, Vec<String>);

fn entry_key(entry: &Entry) -> EntryKey {
    (entry.reading.clone(), entry.candidates.clone())
}

fn page_entries(
    page: &JapanesePage,
    mappings: &MappingIndex,
    source: Source,
) -> (Vec<WiktionaryEntry>, Vec<Entry>, Vec<Entry>) {
    let mut words = Vec::new();
    let mut seikana_entries = Vec::new();
    if source != Source::Wago {
        let (entries, page_seikana_entries, _errors) = kanji_word_entries(page, mappings);
        seikana_entries.extend(page_seikana_entries);
        for entry in entries {
            words.push(WiktionaryEntry {
                reading: entry.reading.clone(),
                candidate: entry.candidates.join("/"),
                suru: false,
                source: EntrySource::KanjiWord,
            });
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
    let mappings = MappingIndex::new(&read_mapping(&options.mapping)?);
    let mut output_entries = BTreeMap::<EntryKey, Entry>::new();
    let mut seikana_entries = BTreeMap::<EntryKey, Entry>::new();
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
        for entry in page_seikana_entries {
            seikana_entries.insert(entry_key(&entry), entry);
        }
        report.kanji_entries += entries
            .iter()
            .filter(|entry| entry.source == EntrySource::KanjiWord)
            .count();
        report.noun_entries += entries
            .iter()
            .filter(|entry| entry.source == EntrySource::Noun)
            .count();
        report.wago_entries += entries
            .iter()
            .filter(|entry| {
                entry.source == EntrySource::WagoOkuri
                    || entry.source == EntrySource::Suru
                    || entry.source == EntrySource::SuruNoun
            })
            .count();
        for entry in converted {
            output_entries.insert(entry_key(&entry), entry);
        }
    }

    report.entries = output_entries.len();
    let dictionary = Dictionary::from_entries(output_entries.values());
    if !options.dry_run {
        dictionary.write_to(&options.output)?;
    }

    report.seikana_entries = seikana_entries.len();
    report.shared_entries = seikana_entries
        .keys()
        .filter(|key| output_entries.contains_key(*key))
        .count();
    seikana_entries.retain(|key, _| !output_entries.contains_key(key));
    let seikana_dictionary = Dictionary::from_entries(seikana_entries.values());
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
    writeln!(writer, "noun_entries\t{}", report.noun_entries)?;
    writeln!(writer, "wago_entries\t{}", report.wago_entries)?;
    writeln!(writer, "seikana_entries\t{}", report.seikana_entries)?;
    writeln!(writer, "shared_entries\t{}", report.shared_entries)?;
    writeln!(writer, "invalid_pages\t{}", report.invalid_pages)?;
    writeln!(writer, "kanjitabs\t{}", report.kanjitabs)?;
    writeln!(writer, "noun_readings\t{}", report.noun_readings)?;
    writer.flush()
}
