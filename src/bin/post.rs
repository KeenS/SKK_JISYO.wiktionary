use std::env;
use std::fs;
use std::io::{self, Write};

use xml_xtract::gsi::post::{generate_jisyo, Row};

fn main() -> io::Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let usage = "Usage: post CSV [OUTPUT]";
    let Some(input) = args.get(1) else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let output = args.get(2).map(String::as_str).unwrap_or("SKK-JISYO.post");

    let text = fs::read_to_string(input)?;
    let rows = parse_csv(&text);
    let jisyo = generate_jisyo(&rows);
    let mut writer = io::BufWriter::new(fs::File::create(output)?);
    jisyo.write(&mut writer)?;
    writer.flush()
}

fn parse_csv(text: &str) -> Vec<Row> {
    text.lines().filter_map(parse_row).collect()
}

fn parse_row(line: &str) -> Option<Row> {
    let fields = line.split(',').collect::<Vec<_>>();
    if fields.len() < 15 {
        return None;
    }
    Some(Row {
        postal_code: unquote(fields[2])?,
        prefecture_kana: unquote(fields[3])?,
        city_kana: unquote(fields[4])?,
        town_kana: unquote(fields[5])?,
        prefecture: unquote(fields[6])?,
        city: unquote(fields[7])?,
        town: unquote(fields[8])?,
    })
}

fn unquote(value: &str) -> Option<String> {
    let value = value.strip_prefix('"')?;
    let value = value.strip_suffix('"')?;
    Some(value.to_string())
}
