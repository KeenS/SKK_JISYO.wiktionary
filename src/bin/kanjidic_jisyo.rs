use std::env;
use std::path::Path;
use std::process;

use xml_xtract::kanjidic;

fn main() {
    let mut args = env::args().skip(1);
    let Some(dictionary) = args.next() else {
        eprintln!("Usage: kanjidic_jisyo KANJIDIC2_XML [OUTPUT]");
        process::exit(2);
    };
    let output = args.next().unwrap_or_else(|| "tmp.kanjidic".to_string());
    if let Err(error) = kanjidic::write_dictionary(Path::new(&dictionary), Path::new(&output)) {
        eprintln!("kanjidic_jisyo: {error}");
        process::exit(1);
    }
}
