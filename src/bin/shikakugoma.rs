use std::env;
use std::path::Path;
use std::process;

use xml_xtract::unihan;

fn main() {
    let mut args = env::args().skip(1);
    let Some(dictionary) = args.next() else {
        eprintln!("Usage: shikakugoma DICTIONARY_LIKE [OUTPUT]");
        process::exit(2);
    };
    let output = args.next().unwrap_or_else(|| "tmp.shikakugoma".to_string());
    if let Err(error) = unihan::write_four_corner(Path::new(&dictionary), Path::new(&output)) {
        eprintln!("shikakugoma: {error}");
        process::exit(1);
    }
}
