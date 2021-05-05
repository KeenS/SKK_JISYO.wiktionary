use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use xml_xtract::kanji_articles;

fn main() {
    let line_regex = Regex::new(r"(?:\[\[)?四角号碼(?:\]\])?\s*[:：]\s*(.*)")
        .expect("internal error: invalid regex");
    let number_regex =
        Regex::new(r"(?:(\d{4})(?:\s*<sub>(\d)</sub>)?)").expect("internal error: invalid regex");

    let ids_file = env::args().nth(1).expect("Usage: IDs XML");
    let xml_file = env::args().nth(2).expect("Usage: IDs XML");
    let output = File::create("tmp.shikakugoma").expect("failed to create output file");

    let mut buffer = BufWriter::new(output);

    for page in kanji_articles(ids_file, xml_file) {
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
