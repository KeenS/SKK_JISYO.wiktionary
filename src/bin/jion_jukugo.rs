use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process;

use xml_xtract::jion::exception::read_exceptions;
use xml_xtract::jion::jisyo::read_jisyo;
use xml_xtract::jion::jukugo::{convert_entries, ReportRow};
use xml_xtract::jion::mapping::read_mapping;

struct Args {
    input: PathBuf,
    mapping: PathBuf,
    output: PathBuf,
    exceptions: Option<PathBuf>,
    report: Option<PathBuf>,
    dry_run: bool,
}

fn usage() {
    println!(
        "Usage: jion_jukugo --input JISYO --mapping MAPPING --output OUTPUT \
         [--exceptions EXCEPTIONS] [--report REPORT] [--dry-run]"
    );
}

fn parse_args() -> Result<Args, String> {
    let mut input = None;
    let mut mapping = None;
    let mut output = None;
    let mut exceptions = None;
    let mut report = None;
    let mut dry_run = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = Some(next_value(&mut args, "--input")?),
            "--mapping" => mapping = Some(next_value(&mut args, "--mapping")?),
            "--output" => output = Some(next_value(&mut args, "--output")?),
            "--exceptions" => exceptions = Some(next_value(&mut args, "--exceptions")?),
            "--report" => report = Some(next_value(&mut args, "--report")?),
            "--dry-run" => dry_run = true,
            "--help" => {
                usage();
                process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    Ok(Args {
        input: input.ok_or("--input is required")?,
        mapping: mapping.ok_or("--mapping is required")?,
        output: output.ok_or("--output is required")?,
        exceptions,
        report,
        dry_run,
    })
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn write_output(path: &PathBuf, entries: &BTreeMap<String, Vec<String>>) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, ";; okuri-ari entries.")?;
    writeln!(writer, ";; okuri-nasi entries.")?;
    for (reading, candidates) in entries {
        writeln!(writer, "{reading} /{}/", candidates.join("/"))?;
    }
    writer.flush()
}

#[cfg(test)]
mod e2e_tests {
    use super::*;
    use std::fs;

    #[test]
    fn converts_fixture_dictionary() {
        let output = env::temp_dir().join("xml-xtract-jukugo-e2e-output");
        let report = env::temp_dir().join("xml-xtract-jukugo-e2e-report");
        let args = Args {
            input: PathBuf::from("tests/fixtures/jion_jukugo_input.tsv"),
            mapping: PathBuf::from("tests/fixtures/jion_jukugo_mapping.tsv"),
            output: output.clone(),
            exceptions: Some(PathBuf::from("tests/fixtures/jion_jukugo_exceptions.tsv")),
            report: Some(report.clone()),
            dry_run: false,
        };

        let entries = read_jisyo(&args.input).unwrap();
        let mappings = read_mapping(&args.mapping).unwrap();
        let exceptions = read_exceptions(args.exceptions.as_ref().unwrap()).unwrap();
        let converted = convert_entries(&entries, &mappings, &exceptions);
        write_output(&args.output, &converted.entries).unwrap();
        write_report(&report, &converted.report).unwrap();

        let output_text = fs::read_to_string(&output).unwrap();
        assert!(output_text.contains(";; okuri-ari entries."));
        assert!(output_text.contains(";; okuri-nasi entries."));
        assert!(output_text.contains("はうかう /方向/"));
        assert!(output_text.contains("がっかう /学校/"));

        let report_text = fs::read_to_string(&report).unwrap();
        assert!(report_text.contains("converted\t方向\tほうこう\tはうかう"));
        assert!(report_text.contains("overridden\t学校\tがっこう\tがっかう"));

        fs::remove_file(output).unwrap();
        fs::remove_file(report).unwrap();
    }
}

fn write_report(path: &PathBuf, report: &[ReportRow]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "status\tcandidate\treading\tdetail")?;
    for row in report {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}",
            row.status, row.candidate, row.reading, row.detail
        )?;
    }
    writer.flush()
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    let entries = read_jisyo(&args.input)?;
    let mappings = read_mapping(&args.mapping)?;
    let exceptions = match &args.exceptions {
        Some(path) => read_exceptions(path)?,
        None => Vec::new(),
    };

    let converted = convert_entries(&entries, &mappings, &exceptions);
    if let Some(report_path) = &args.report {
        write_report(report_path, &converted.report)?;
    }
    if !args.dry_run {
        write_output(&args.output, &converted.entries)?;
    }

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use xml_xtract::jion::jukugo::is_jukugo_candidate;

    #[test]
    fn detects_jukugo_candidates() {
        assert!(is_jukugo_candidate("方向"));
        assert!(!is_jukugo_candidate("方"));
        assert!(!is_jukugo_candidate("向く"));
    }
}
