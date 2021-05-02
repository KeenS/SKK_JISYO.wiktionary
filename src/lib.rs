use quick_xml::de::from_reader;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub mod model;

pub fn kanji_articles(
    ids_file: impl AsRef<Path>,
    xml_file: impl AsRef<Path>,
) -> impl Iterator<Item = model::Page> {
    let ids = File::open(ids_file).expect("failed to open file");
    let xml = File::open(xml_file).expect("failed to read file");
    let wiki =
        from_reader::<_, model::Mediawiki>(BufReader::new(xml)).expect("failed to decode xml");
    let ids = BufReader::new(ids)
        .lines()
        .map(|l| l.expect("line error").parse::<u64>().expect("parse error"))
        .collect::<Vec<_>>();

    wiki.page
        .into_iter()
        .filter(move |page| ids.contains(&page.id) && page.title.chars().count() == 1)
}
