use std::env;
use std::fs;
use std::io::{self, Write};

use xml_xtract::gsi::pdf_text::generate_jisyo;

fn main() -> io::Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let usage = "Usage: gsi PDF_TEXT [OUTPUT]";
    let input = args.get(1).unwrap_or_else(|| {
        eprintln!("{usage}");
        std::process::exit(2);
    });
    let output = args.get(2).map(String::as_str).unwrap_or("SKK-JISYO.gsi");

    let pdf_text = fs::read_to_string(input)?;
    let jisyo = generate_jisyo(&pdf_text);
    let mut writer = io::BufWriter::new(fs::File::create(output)?);
    jisyo.write(&mut writer)?;
    writer.flush()
}
