use std::fs;
use std::process::Command;

#[test]
fn converts_wiktionary_fixture_end_to_end() {
    let output = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-output");
    let report = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-report");

    let status = Command::new(env!("CARGO_BIN_EXE_wiktionary_jisyo"))
        .arg("--xml")
        .arg("tests/fixtures/wiktionary_jisyo.xml")
        .arg("--mapping")
        .arg("tests/fixtures/wiktionary_jisyo_mapping.tsv")
        .arg("--exceptions")
        .arg("tests/fixtures/wiktionary_jisyo_exceptions.tsv")
        .arg("--output")
        .arg(&output)
        .arg("--report")
        .arg(&report)
        .status()
        .unwrap();
    assert!(status.success());

    let output_text = fs::read_to_string(&output).unwrap();
    assert!(output_text.contains(";; okuri-ari entries."));
    assert!(output_text.contains(";; okuri-nasi entries."));
    assert!(output_text.contains("がくかう /学校/"));
    assert!(output_text.contains("いどう /移動/する/"));
    assert!(!output_text.contains("あるk /歩/"));

    let report_text = fs::read_to_string(&report).unwrap();
    assert!(report_text.contains("metric\tcount"));
    assert!(report_text.contains("pages\t3"));
    assert!(report_text.contains("invalid_pages\t1"));
    assert!(report_text.contains("excluded\t1"));

    fs::remove_file(output).unwrap();
    fs::remove_file(report).unwrap();
}

#[test]
fn dry_run_does_not_write_wiktionary_dictionary() {
    let output = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-dry-run-output");
    let report = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-dry-run-report");
    let _ = fs::remove_file(&output);

    let status = Command::new(env!("CARGO_BIN_EXE_wiktionary_jisyo"))
        .arg("--xml")
        .arg("tests/fixtures/wiktionary_jisyo.xml")
        .arg("--mapping")
        .arg("tests/fixtures/wiktionary_jisyo_mapping.tsv")
        .arg("--output")
        .arg(&output)
        .arg("--report")
        .arg(&report)
        .arg("--dry-run")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!output.exists());

    let report_text = fs::read_to_string(&report).unwrap();
    assert!(report_text.contains("entries\t2"));

    fs::remove_file(report).unwrap();
}
