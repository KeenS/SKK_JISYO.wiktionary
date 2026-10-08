use std::env;
use std::path::Path;
use std::process;

use xml_xtract::jmnedict;

fn main() {
    let mut args = env::args().skip(1);
    let Some(dictionary) = args.next() else {
        eprintln!("Usage: jmnedict_jisyo JMNEDICT_XML [OUTPUT]");
        process::exit(2);
    };
    let output = args.next().unwrap_or_else(|| "tmp.jmnedict".to_string());
    if let Err(error) = jmnedict::write_dictionary(Path::new(&dictionary), Path::new(&output)) {
        eprintln!("jmnedict_jisyo: {error}");
        process::exit(1);
    }
}
