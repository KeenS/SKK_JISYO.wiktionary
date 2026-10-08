use std::env;
use std::path::Path;
use std::process;

use xml_xtract::jmdict;

fn main() {
    let mut args = env::args().skip(1);
    let Some(dictionary) = args.next() else {
        eprintln!("Usage: jmdict_jisyo JMDICT_E [OUTPUT]");
        process::exit(2);
    };
    let output = args.next().unwrap_or_else(|| "tmp.jmdict".to_string());
    if let Err(error) = jmdict::write_dictionary(Path::new(&dictionary), Path::new(&output)) {
        eprintln!("jmdict_jisyo: {error}");
        process::exit(1);
    }
}
