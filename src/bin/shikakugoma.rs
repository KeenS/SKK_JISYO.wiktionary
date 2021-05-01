use quick_xml::de::from_reader;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use xml_xtract::model::*;

fn main() {
    let line_regex = Regex::new(r"(?:\[\[)?四角号碼(?:\]\])?\s*[:：]\s*(.*)")
        .expect("internal error: invalid regex");
    let number_regex =
        Regex::new(r"(?:(\d{4})(?:\s*<sub>(\d)</sub>)?)").expect("internal error: invalid regex");

    let id_file = env::args().nth(1).expect("Usage: IDs XML");
    let xml_file = env::args().nth(2).expect("Usage: IDs XML");
    let ids = File::open(id_file).expect("failed to open file");
    let xml = File::open(xml_file).expect("failed to read file");
    let output = File::create("SKK_JISYO.shikakugoma").expect("failed to create output file");
    let wiki = from_reader::<_, Mediawiki>(BufReader::new(xml)).expect("failed to decode xml");
    let mut buffer = BufWriter::new(output);

    writeln!(
        buffer,
        "; この辞書はWikitonary[^1]を元に生成されておりCC BY-SA 3.0[^2]の下提供されます\n; [^1]: https://ja.wiktionary.org/\n; [^2]: https://creativecommons.org/licenses/by-sa/3.0/deed.ja"
    )
    .expect("failed to write header");

    let ids = BufReader::new(ids)
        .lines()
        .map(|l| l.expect("line error").parse::<u64>().expect("parse error"))
        .collect::<Vec<_>>();

    let pages = wiki
        .page
        .into_iter()
        .filter(|page| ids.contains(&page.id) && page.title.chars().count() == 1);

    for page in pages {
        match line_regex.captures(&page.revision.text) {
            None => {
                println!("{}: no match", page.title);
                if page.revision.text.contains("四角号碼") {
                    println!("Hole of regex in {} {}", page.id, page.title)
                }
            }
            Some(cap) => match cap.get(1) {
                None => {
                    println!("empty shikakugoma in {}: {}", page.id, page.title);
                    // println!("{}", page.revision.text);
                }
                Some(line) => {
                    for shikakugoma in number_regex.captures_iter(line.as_str()) {
                        let code = shikakugoma.get(1).unwrap().as_str();
                        writeln!(buffer, "{} /{}/", code, page.title)
                            .expect("failed to write to output");
                        if let Some(sub) = shikakugoma.get(2) {
                            let sub_code = sub.as_str();
                            writeln!(buffer, "{}{} /{}/", code, sub_code, page.title)
                                .expect("failed to write sub to output");
                        }
                    }
                }
            },
        }
    }
}
