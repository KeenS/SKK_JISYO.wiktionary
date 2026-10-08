use std::env;
use std::path::Path;
use std::process;

use xml_xtract::unihan;

fn main() {
    let mut args = env::args().skip(1);
    let Some(readings) = args.next() else {
        eprintln!("Usage: unihan_jisyo READINGS [OUTPUT]");
        process::exit(2);
    };
    let output = args.next().unwrap_or_else(|| "tmp.unihan".to_string());
    if let Err(error) = unihan::write_dictionary(Path::new(&readings), Path::new(&output)) {
        eprintln!("unihan_jisyo: {error}");
        process::exit(1);
    }
}
