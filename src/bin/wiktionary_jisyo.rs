use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use wana_kana::ConvertJapanese;
use xml_xtract::articles;
use xml_xtract::jion::entry::Entry;
use xml_xtract::jion::mapping::{read_mapping, MappingIndex};
use xml_xtract::jion::on_reading::has_on_reading;
use xml_xtract::jion::on_reading::katakana_to_hiragana;
use xml_xtract::jion::wiktionary::{
    is_kanji, kanji_word_entries, old_japanese_entries, parse_japanese_page, redirect_target,
    to_entry, wiktionary_entries, EntrySource, JapanesePage, WiktionaryEntry,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use xml_xtract::jion::mapping::Mapping;

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
            jion_output: None,
            ojp_output: None,
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
            kanji_reading_entries: 0,
            noun_entries: 0,
            wago_entries: 1,
            idiom_entries: 0,
            kangokana_entries: 0,
            redirect_entries: 0,
            symbol_entries: 1,
            jion_entries: 3,
            ojp_entries: 0,
            shared_entries: 1,
            invalid_pages: 0,
            kanjitabs: 0,
            noun_readings: 0,
        };
        write_report(path.clone(), &report).unwrap();
        let contents = fs::read_to_string(path).unwrap();
        assert!(contents.contains("entries\t2"));
        assert!(contents.contains("jion_entries\t3"));
        assert!(contents.contains("shared_entries\t1"));
    }

    #[test]
    fn converts_kanji_pages() {
        use xml_xtract::model::{Page, Revision};

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
        let raw_page = Page {
            ns: 0,
            id: 1,
            title: "学校".into(),
            revision: Revision {
                id: 1,
                comment: None,
                text: String::new(),
            },
        };
        let (_, converted, jion) =
            page_entries(&page, &mappings, Source::All, &raw_page, &HashMap::new());
        assert_eq!(converted, vec![Entry::new("がっこう", "学校")]);
        assert_eq!(jion, vec![Entry::new("がくかう", "学校")]);
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
    jion_output: Option<PathBuf>,
    ojp_output: Option<PathBuf>,
    dry_run: bool,
}

#[derive(Default)]
struct Report {
    pages: usize,
    entries: usize,
    kanji_entries: usize,
    kanji_reading_entries: usize,
    noun_entries: usize,
    wago_entries: usize,
    idiom_entries: usize,
    kangokana_entries: usize,
    redirect_entries: usize,
    symbol_entries: usize,
    jion_entries: usize,
    ojp_entries: usize,
    shared_entries: usize,
    invalid_pages: usize,
    kanjitabs: usize,
    noun_readings: usize,
}

impl Report {
    fn add_entries(&mut self, entries: &[WiktionaryEntry]) {
        for entry in entries {
            match entry.source {
                EntrySource::KanjiWord => self.kanji_entries += 1,
                EntrySource::KanjiReading => self.kanji_reading_entries += 1,
                EntrySource::Noun => self.noun_entries += 1,
                EntrySource::Idiom => self.idiom_entries += 1,
                EntrySource::WagoOkuri | EntrySource::Suru | EntrySource::SuruNoun => {
                    self.wago_entries += 1
                }
                EntrySource::Kangokana => self.kangokana_entries += 1,
                EntrySource::OldJapanese => self.ojp_entries += 1,
                EntrySource::Redirect => self.redirect_entries += 1,
                EntrySource::Symbol => self.symbol_entries += 1,
            }
        }
    }
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
         [--jion-output SEIKANA_OUTPUT] [--ojp-output OJP_OUTPUT] [--dry-run]"
    );
    code
}

fn parse_args() -> Result<Options, ExitCode> {
    let mut args = env::args().skip(1);
    let mut values = HashMap::new();
    let mut source = Source::All;
    let mut dry_run = false;
    let mut jion_output = None;
    let mut ojp_output = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--jion-output" => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                jion_output = Some(PathBuf::from(value));
            }
            "--ojp-output" => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                ojp_output = Some(PathBuf::from(value));
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
        jion_output,
        ojp_output,
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
    raw_page: &xml_xtract::model::Page,
    redirects: &HashMap<String, Vec<String>>,
) -> (Vec<WiktionaryEntry>, Vec<Entry>, Vec<Entry>) {
    let mut words = Vec::new();
    let mut jion_entries = Vec::new();
    if source != Source::Wago {
        let (entries, page_jion_entries, _errors) = kanji_word_entries(page, mappings);
        jion_entries.extend(page_jion_entries);
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
        if source == Source::All {
            words.extend(new_style_variant_entries(page, raw_page));
            if let Some(titles) = redirects.get(page.title.as_str()) {
                let reading = redirect_reading(page).map(|reading| katakana_to_hiragana(&reading));
                words.extend(redirect_word_entries(reading, page.title.as_str(), titles));
            }
        }
    }
    let entries = words.iter().map(to_entry).collect();
    (words, entries, jion_entries)
}

fn redirect_word_entries(
    reading: Option<String>,
    canonical_title: &str,
    titles: &[String],
) -> Vec<WiktionaryEntry> {
    let Some(reading) = reading else {
        return Vec::new();
    };
    let mut entries = vec![WiktionaryEntry {
        reading: reading.clone(),
        candidate: canonical_title.to_string(),
        suru: false,
        source: EntrySource::Redirect,
    }];
    entries.extend(
        titles
            .iter()
            .filter(|title| title.chars().any(is_kanji))
            .map(|title| WiktionaryEntry {
                reading: reading.clone(),
                candidate: title.clone(),
                suru: false,
                source: EntrySource::Redirect,
            }),
    );
    entries
}

fn new_style_variant_entries(
    page: &JapanesePage,
    raw_page: &xml_xtract::model::Page,
) -> Vec<WiktionaryEntry> {
    if page.new_style_variants.is_empty() {
        return Vec::new();
    }
    let mut entries = Vec::new();
    for reading in &page.default_sorts {
        let reading = reading.to_hiragana();
        if has_on_reading(&raw_page.revision.text, &reading) {
            for candidate in &page.new_style_variants {
                entries.push(WiktionaryEntry {
                    reading: reading.clone(),
                    candidate: candidate.clone(),
                    suru: false,
                    source: EntrySource::Idiom,
                });
            }
        }
    }
    entries
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
    let redirects = redirect_index(&options.xml);
    let mut output_entries = BTreeMap::<EntryKey, Entry>::new();
    let mut jion_entries = BTreeMap::<EntryKey, Entry>::new();
    let mut ojp_entries = BTreeMap::<EntryKey, Entry>::new();
    let mut report = Report::default();

    for page in articles(&options.xml) {
        if let Some(text) = page.symbol_text() {
            let parsed = parse_japanese_page(page.title.as_str(), text);
            let entries = wiktionary_entries(&parsed);
            report.add_entries(&entries);
            for entry in entries.iter().map(to_entry) {
                output_entries.insert(entry_key(&entry), entry);
            }
            continue;
        }

        if let Some(text) = page.old_japanese_text_with_default_sort() {
            let parsed = parse_japanese_page(page.title.as_str(), &text);
            let entries = old_japanese_entries(&parsed);
            report.add_entries(&entries);
            for entry in entries.iter().map(to_entry) {
                ojp_entries.insert(entry_key(&entry), entry);
            }
        }

        let Some(text) = page.japanese_text_with_default_sort() else {
            report.invalid_pages += 1;
            continue;
        };
        report.pages += 1;
        let parsed = parse_japanese_page(page.title.as_str(), &text);
        report.kanjitabs += parsed.kanjitabs.len();
        report.noun_readings += parsed.noun_readings.len();
        let (entries, converted, page_jion_entries) =
            page_entries(&parsed, &mappings, options.source, &page, &redirects);
        for entry in page_jion_entries {
            jion_entries.insert(entry_key(&entry), entry);
        }
        report.add_entries(&entries);
        for entry in converted {
            output_entries.insert(entry_key(&entry), entry);
        }
    }

    report.entries = output_entries.len();
    let dictionary = Dictionary::from_entries(output_entries.values());
    if !options.dry_run {
        dictionary.write_to(&options.output)?;
    }

    report.jion_entries = jion_entries.len();
    report.shared_entries = jion_entries
        .keys()
        .filter(|key| output_entries.contains_key(*key))
        .count();
    jion_entries.retain(|key, _| !output_entries.contains_key(key));
    let jion_dictionary = Dictionary::from_entries(jion_entries.values());
    if let Some(path) = options.jion_output {
        jion_dictionary.write_to(&path)?;
    }

    let ojp_dictionary = Dictionary::from_entries(ojp_entries.values());
    if let Some(path) = options.ojp_output {
        if !options.dry_run {
            ojp_dictionary.write_to(&path)?;
        }
    }

    if let Some(report_path) = options.report {
        write_report(report_path, &report)?;
    }

    Ok(())
}

fn redirect_index(xml: &PathBuf) -> HashMap<String, Vec<String>> {
    let mut redirects: HashMap<String, Vec<String>> = HashMap::new();
    for page in articles(xml) {
        if page.ns != 0 {
            continue;
        }
        if let Some(target) = redirect_target(&page.revision.text) {
            redirects
                .entry(target.to_string())
                .or_default()
                .push(page.title.clone());
        }
    }
    redirects
}

fn redirect_reading(page: &JapanesePage) -> Option<String> {
    page.noun_readings
        .iter()
        .chain(page.pron_readings.iter())
        .chain(page.noun_suru_readings.iter())
        .find(|reading| valid_dictionary_reading(reading))
        .cloned()
        .or_else(|| {
            page.default_sorts
                .iter()
                .find(|reading| valid_dictionary_reading(reading))
                .cloned()
        })
}

fn valid_dictionary_reading(reading: &str) -> bool {
    !reading.is_empty()
        && reading.chars().count() <= 32
        && reading.chars().all(|ch| ('ぁ'..='ゖ').contains(&ch))
}

fn write_report(path: PathBuf, report: &Report) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "metric\tcount")?;
    writeln!(writer, "pages\t{}", report.pages)?;
    writeln!(writer, "entries\t{}", report.entries)?;
    writeln!(writer, "kanji_entries\t{}", report.kanji_entries)?;
    writeln!(
        writer,
        "kanji_reading_entries\t{}",
        report.kanji_reading_entries
    )?;
    writeln!(writer, "noun_entries\t{}", report.noun_entries)?;
    writeln!(writer, "wago_entries\t{}", report.wago_entries)?;
    writeln!(writer, "idiom_entries\t{}", report.idiom_entries)?;
    writeln!(writer, "kangokana_entries\t{}", report.kangokana_entries)?;
    writeln!(writer, "redirect_entries\t{}", report.redirect_entries)?;
    writeln!(writer, "symbol_entries\t{}", report.symbol_entries)?;
    writeln!(writer, "jion_entries\t{}", report.jion_entries)?;
    writeln!(writer, "ojp_entries\t{}", report.ojp_entries)?;
    writeln!(writer, "shared_entries\t{}", report.shared_entries)?;
    writeln!(writer, "invalid_pages\t{}", report.invalid_pages)?;
    writeln!(writer, "kanjitabs\t{}", report.kanjitabs)?;
    writeln!(writer, "noun_readings\t{}", report.noun_readings)?;
    writer.flush()
}
