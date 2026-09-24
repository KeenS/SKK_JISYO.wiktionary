use regex::Regex;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;
use xml_xtract::kanji_articles;

fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    path.with_file_name(name)
}

fn usage() {
    eprintln!("Usage: shikakugoma IDS XML [OUTPUT]");
}

fn generate(ids_file: &str, xml_file: &str, output: &Path) -> io::Result<()> {
    let line_regex = Regex::new(r"(?:\[\[)?四角号碼(?:\]\])?\s*[:：]\s*(.*)").unwrap();
    let template_regex = Regex::new(r"\{\{検字\|[^\n}]*四角\s*=\s*([^\n}]+)").unwrap();
    let number_regex = Regex::new(r"(\d{4})(?:\.(\d))?").unwrap();

    let partial = partial_path(output);
    let result = (|| {
        let file = File::create(&partial)?;
        let mut buffer = BufWriter::new(file);
        let mut pages = kanji_articles(ids_file, xml_file)?;
        for page in pages.by_ref() {
            let page = page?;
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
                writeln!(buffer, "{code} /{}/", page.title)?;
                if let Some(sub) = sub {
                    writeln!(buffer, "{code}{sub} /{}/", page.title)?;
                }
            }

            if codes.is_empty() {
                println!("{}: no match", page.title);
                if page.revision.text.contains("四角") {
                    println!("Hole of regex in {} {}", page.id, page.title)
                }
            }
        }
        if pages.skipped_pages() > 0 {
            eprintln!("shikakugoma: skipped {} pages", pages.skipped_pages());
        }
        buffer.flush()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
        return result;
    }
    fs::rename(&partial, output)?;
    Ok(())
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(ids_file) = args.next() else {
        usage();
        process::exit(2);
    };
    let Some(xml_file) = args.next() else {
        usage();
        process::exit(2);
    };
    let output = args.next().unwrap_or_else(|| "tmp.shikakugoma".to_string());
    if let Err(error) = generate(&ids_file, &xml_file, Path::new(&output)) {
        eprintln!("shikakugoma: {error}");
        process::exit(1);
    }
}
