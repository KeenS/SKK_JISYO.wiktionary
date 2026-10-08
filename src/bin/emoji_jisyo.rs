use std::env;
use std::path::Path;
use std::process;

use xml_xtract::emoji::{self, Inputs};

fn main() {
    let mut args = env::args().skip(1);
    let Some(emoji_test) = args.next() else {
        usage();
    };
    let Some(ja) = args.next() else { usage() };
    let Some(en) = args.next() else { usage() };
    let Some(ja_derived) = args.next() else {
        usage()
    };
    let Some(en_derived) = args.next() else {
        usage()
    };
    let Some(unicode_data) = args.next() else {
        usage()
    };
    let Some(names_list) = args.next() else {
        usage()
    };
    let output = args.next().unwrap_or_else(|| "tmp.emoji".to_string());
    let inputs = Inputs {
        emoji_test: Path::new(&emoji_test),
        ja: Path::new(&ja),
        en: Path::new(&en),
        ja_derived: Path::new(&ja_derived),
        en_derived: Path::new(&en_derived),
        unicode_data: Path::new(&unicode_data),
        names_list: Path::new(&names_list),
    };
    if let Err(error) = emoji::write_dictionary(&inputs, Path::new(&output)) {
        eprintln!("emoji_jisyo: {error}");
        process::exit(1);
    }
}

fn usage() -> ! {
    eprintln!(
        "Usage: emoji_jisyo EMOJI_TEST JA_XML EN_XML JA_DERIVED EN_DERIVED UNICODE_DATA NAMES_LIST [OUTPUT]"
    );
    process::exit(2);
}
