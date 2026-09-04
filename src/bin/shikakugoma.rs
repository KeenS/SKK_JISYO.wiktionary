use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use xml_xtract::kanji_articles;

fn main() {
    let line_regex = Regex::new(r"(?:\[\[)?四角号碼(?:\]\])?\s*[:：]\s*(.*)")
        .expect("internal error: invalid regex");
    let template_regex = Regex::new(r"\{\{検字\|[^\n}]*四角\s*=\s*([^\n}]+)")
        .expect("internal error: invalid regex");
    let number_regex = Regex::new(r"(\d{4})(?:\.(\d))?").expect("internal error: invalid regex");

    let ids_file = env::args().nth(1).expect("Usage: IDs XML");
    let xml_file = env::args().nth(2).expect("Usage: IDs XML");
    let output = File::create("tmp.shikakugoma").expect("failed to create output file");

    let mut buffer = BufWriter::new(output);

    for page in kanji_articles(ids_file, xml_file) {
        let mut codes = Vec::new();

        if let Some(cap) = template_regex.captures(&page.revision.text) {
            let line = cap.get(1).unwrap().as_str();
            for shikakugoma in number_regex.captures_iter(line) {
                let code = shikakugoma[1].to_string();
                let sub = shikakugoma.get(2).map(|sub| sub.as_str().to_string());
                if !codes.contains(&(code.clone(), sub.clone())) {
                    codes.push((code, sub));
                }
            }
        }

        if codes.is_empty() {
            if let Some(cap) = line_regex.captures(&page.revision.text) {
                if let Some(line) = cap.get(1) {
                    for shikakugoma in number_regex.captures_iter(line.as_str()) {
                        let code = shikakugoma[1].to_string();
                        let sub = shikakugoma.get(2).map(|sub| sub.as_str().to_string());
                        if !codes.contains(&(code.clone(), sub.clone())) {
                            codes.push((code, sub));
                        }
                    }
                }
            }
        }

        for (code, sub) in &codes {
            writeln!(buffer, "{code} /{}/", page.title).expect("failed to write to output");
            if let Some(sub) = sub {
                writeln!(buffer, "{code}{sub} /{}/", page.title)
                    .expect("failed to write sub to output");
            }
        }

        if codes.is_empty() {
            println!("{}: no match", page.title);
            if page.revision.text.contains("四角") {
                println!("Hole of regex in {} {}", page.id, page.title)
            }
        }
    }
}
