use once_cell::sync::OnceCell;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use xml_xtract::{kanji_articles, model::*};

static EXTRACT_REGEX: OnceCell<Regex> = OnceCell::new();

fn extract_on(mut buffer: impl Write, area: &str, page: &Page) -> io::Result<()> {
    if area.contains("無し") {
        println!("no on in {}", page.title);
        return Ok(());
    }
    let mut at_least_one = false;
    for cap in EXTRACT_REGEX.get().unwrap().captures_iter(area) {
        at_least_one = true;
        let gen = cap.get(1).unwrap().as_str();
        let sei = cap.get(2).unwrap().as_str();
        writeln!(buffer, "{} /{}/", sei, page.title)?;
        writeln!(buffer, "{} /{};{}/", gen, page.title, sei)?;
    }

    if !at_least_one {
        println!("no seikana in {}\n{}", page.title, area)
    }
    Ok(())
}

fn extract_kun(mut buffer: impl Write, area: &str, page: &Page) -> io::Result<()> {
    if area.contains("無し") || page.revision.text.contains("原則として音読み") {
        println!("no on in {}", page.title);
        return Ok(());
    }
    let mut at_least_one = false;
    for cap in EXTRACT_REGEX.get().unwrap().captures_iter(area) {
        at_least_one = true;
        let gen = cap.get(1).unwrap().as_str();
        let sei = cap.get(2).unwrap().as_str();
        writeln!(buffer, "{} /{}/", sei, page.title)?;
        writeln!(buffer, "{} /{};{}/", gen, page.title, sei)?;
    }

    if !at_least_one {
        println!("no seikana in {}\n{}", page.title, area)
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let on_area_regex = Regex::new(r"(?m)\*\s*(\[\[)?音読(み)?(\]\])?.*(\n^(\*\*|\*:).*$)*")
        .expect("internal error: invalid regex");
    let kun_area_regex = Regex::new(r"(?m)\*\s*(\[\[)?訓読(み)?(\]\])?.*(\n^\*\*.*$)*")
        .expect("internal error: invalid regex");
    EXTRACT_REGEX
        .set(
            Regex::new(
                r"(?:\[\[)?([\p{Hiragana}\p{Katakana}]+)(?:\]\])?\s*[(（](?:\[\[)?([\p{Hiragana}\p{Katakana}]+)(?:\]\])?[)）]",
            )
            .expect("internal error: invalid regex"),
        )
        .unwrap();

    let ids_file = env::args().nth(1).expect("Usage: IDs XML");
    let xml_file = env::args().nth(2).expect("Usage: IDs XML");
    let output = File::create("SKK_JISYO.seikana")?;
    let mut buffer = BufWriter::new(output);

    writeln!(
        buffer,
        "; この辞書はWikitonary[^1]を元に生成されておりCC BY-SA 3.0[^2]の下提供されます\n; [^1]: https://ja.wiktionary.org/\n; [^2]: https://creativecommons.org/licenses/by-sa/3.0/deed.ja"
    )?;

    for page in kanji_articles(ids_file, xml_file) {
        let mut processed = false;
        if let Some(m) = on_area_regex.find(&page.revision.text) {
            processed = true;
            extract_on(&mut buffer, m.as_str(), &page)?
        }
        if let Some(m) = kun_area_regex.find(&page.revision.text) {
            processed = true;
            extract_kun(&mut buffer, m.as_str(), &page)?
        }

        if !processed {
            println!("{}: no match", page.title);
            if ["呉音", "漢音", "音読", "訓読"]
                .iter()
                .any(|p| page.revision.text.contains(p))
            {
                println!(
                    "Hole of regex in {} {}:\n{}",
                    page.id, page.title, page.revision.text
                )
            }
        }
    }
    Ok(())
}
