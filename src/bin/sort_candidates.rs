use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use xml_xtract::candidate_order::{
    japanese_link_frequencies, sort_candidate_annotations, LinkFrequency,
};
use xml_xtract::jion::jisyo::parse_line;

#[derive(Debug)]
struct Options {
    xml: PathBuf,
    input: PathBuf,
    output: PathBuf,
}

fn usage(code: ExitCode) -> ExitCode {
    eprintln!("Usage: sort_candidates --xml XML --input DICTIONARY --output DICTIONARY");
    code
}

fn parse_args() -> Result<Options, ExitCode> {
    parse_args_from(env::args().skip(1))
}

fn parse_args_from(mut args: impl Iterator<Item = String>) -> Result<Options, ExitCode> {
    let mut xml = None;
    let mut input = None;
    let mut output = None;

    fn required(values: &mut Option<PathBuf>, value: String) -> Result<(), ExitCode> {
        if values.is_some() {
            return Err(usage(ExitCode::FAILURE));
        }
        *values = Some(PathBuf::from(value));
        Ok(())
    }

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--xml" => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                required(&mut xml, value)?;
            }
            "--input" => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                required(&mut input, value)?;
            }
            "--output" => {
                let value = args.next().ok_or_else(|| usage(ExitCode::FAILURE))?;
                required(&mut output, value)?;
            }
            "--help" => return Err(usage(ExitCode::SUCCESS)),
            _ => return Err(usage(ExitCode::FAILURE)),
        }
    }

    Ok(Options {
        xml: xml.ok_or_else(|| usage(ExitCode::FAILURE))?,
        input: input.ok_or_else(|| usage(ExitCode::FAILURE))?,
        output: output.ok_or_else(|| usage(ExitCode::FAILURE))?,
    })
}

fn run(options: &Options) -> io::Result<()> {
    let frequencies = japanese_link_frequencies(
        options
            .xml
            .to_str()
            .ok_or_else(|| io::Error::other("XML path is not valid UTF-8"))?,
    )?;
    sort_dictionary(options, &frequencies)
}

fn sort_dictionary(options: &Options, frequencies: &LinkFrequency) -> io::Result<()> {
    let reader = BufReader::new(File::open(&options.input)?);
    let writer = BufWriter::new(File::create(&options.output)?);
    sort_stream(reader, writer, frequencies)
}

fn sort_stream(
    reader: impl BufRead,
    mut writer: impl Write,
    frequencies: &LinkFrequency,
) -> io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if let Some(entry) = parse_line(&line) {
            let candidate_annotations = entry.candidate_annotations();
            let sorted = sort_candidate_annotations(&candidate_annotations, frequencies);
            writeln!(writer, "{} /{}/", entry.reading, sorted.join("/"))?;
        } else {
            writeln!(writer, "{line}")?;
        }
    }
    writer.flush()
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(code) => return code,
    };

    if let Err(error) = run(&options) {
        eprintln!("sort_candidates: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_all_arguments() {
        assert!(parse_args_from(Vec::<String>::new().into_iter()).is_err());
        assert!(
            parse_args_from(vec!["--xml".to_string(), "dump.xml".to_string()].into_iter()).is_err()
        );
        assert!(parse_args_from(
            vec![
                "--xml".into(),
                "dump.xml".into(),
                "--input".into(),
                "input".into(),
                "--output".into(),
                "output".into(),
            ]
            .into_iter()
        )
        .is_ok());
    }

    #[test]
    fn sorts_entries_and_preserves_headers_and_annotations() {
        let mut frequencies = LinkFrequency::new();
        frequencies.insert("笑".to_string(), 42);
        frequencies.insert("咲".to_string(), 5);
        frequencies.insert("安".to_string(), 10);
        frequencies.insert("唵".to_string(), 1);
        frequencies.insert("呵".to_string(), 1);

        let input = "; header\nわらu /粲/笑/晒/咲/咥/呵/听/\nあん /唵;アム/安;アム/庵;アム/\n";
        let mut output = Vec::new();

        sort_stream(input.as_bytes(), &mut output, &frequencies).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "; header\nわらu /笑/咲/呵/粲/晒/咥/听/\nあん /安;アム/唵;アム/庵;アム/\n"
        );
    }
}
