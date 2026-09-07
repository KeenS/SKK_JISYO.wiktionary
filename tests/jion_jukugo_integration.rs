use std::fs;
use std::process::Command;

#[test]
fn converts_fixture_dictionary_end_to_end() {
    let output = std::env::temp_dir().join("xml-xtract-jukugo-integration-output");
    let report = std::env::temp_dir().join("xml-xtract-jukugo-integration-report");

    let status = Command::new(env!("CARGO_BIN_EXE_jion_jukugo"))
        .arg("--input")
        .arg("tests/fixtures/jion_jukugo_input.tsv")
        .arg("--mapping")
        .arg("tests/fixtures/jion_jukugo_mapping.tsv")
        .arg("--exceptions")
        .arg("tests/fixtures/jion_jukugo_exceptions.tsv")
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
    assert!(output_text.contains("はうかう /方向/"));
    assert!(output_text.contains("がっかう /学校/"));

    let report_text = fs::read_to_string(&report).unwrap();
    assert!(report_text.contains("status\tcandidate\treading\tdetail"));
    assert!(report_text.contains("converted\t方向\tほうこう\tはうかう"));
    assert!(report_text.contains("overridden\t学校\tがっこう\tがっかう"));

    fs::remove_file(output).unwrap();
    fs::remove_file(report).unwrap();
}

#[test]
fn converts_yoon_jukugo_from_all_kanji_fixture() {
    let output = std::env::temp_dir().join("xml-xtract-jukugo-yoon-output");
    let report = std::env::temp_dir().join("xml-xtract-jukugo-yoon-report");

    let status = Command::new(env!("CARGO_BIN_EXE_jion_jukugo"))
        .arg("--input")
        .arg("tests/fixtures/jion_jukugo_all_kanji_input.tsv")
        .arg("--mapping")
        .arg("tests/fixtures/jion_jukugo_mapping.tsv")
        .arg("--output")
        .arg(&output)
        .arg("--report")
        .arg(&report)
        .status()
        .unwrap();
    assert!(status.success());

    let output_text = fs::read_to_string(&output).unwrap();
    assert!(output_text.contains("しやくわい /社会/"));

    let report_text = fs::read_to_string(&report).unwrap();
    assert!(report_text.contains("converted\t社会\tしゃかい\tしやくわい"));

    fs::remove_file(output).unwrap();
    fs::remove_file(report).unwrap();
}

#[test]
fn dry_run_does_not_write_dictionary() {
    let output = std::env::temp_dir().join("xml-xtract-jukugo-dry-run-output");
    let report = std::env::temp_dir().join("xml-xtract-jukugo-dry-run-report");
    let _ = fs::remove_file(&output);

    let status = Command::new(env!("CARGO_BIN_EXE_jion_jukugo"))
        .arg("--input")
        .arg("tests/fixtures/jion_jukugo_input.tsv")
        .arg("--mapping")
        .arg("tests/fixtures/jion_jukugo_mapping.tsv")
        .arg("--output")
        .arg(&output)
        .arg("--report")
        .arg(&report)
        .arg("--dry-run")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!output.exists());

    fs::remove_file(report).unwrap();
}
