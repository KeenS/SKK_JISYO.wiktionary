use std::fs;
use std::process::Command;

#[test]
fn parser_decodes_xml_entities() {
    let xml_path = std::env::temp_dir().join("xml-xtract-escaped-text.xml");
    fs::write(
        &xml_path,
        r#"<mediawiki>
  <page>
    <title>学</title>
    <ns>0</ns>
    <id>1</id>
    <revision>
      <id>10</id>
      <text>漢音=コウ&lt;カウ</text>
    </revision>
  </page>
</mediawiki>"#,
    )
    .unwrap();

    let pages: Vec<_> = xml_xtract::articles(&xml_path).collect();
    fs::remove_file(xml_path).unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].revision.text, "漢音=コウ<カウ");
}

#[test]
fn parser_continues_after_empty_text_element() {
    let xml_path = std::env::temp_dir().join("xml-xtract-empty-text.xml");
    fs::write(
        &xml_path,
        r#"<mediawiki>
  <page>
    <title>空</title>
    <ns>0</ns>
    <id>1</id>
    <revision>
      <id>10</id>
      <text bytes="0" />
      <sha1>x</sha1>
    </revision>
  </page>
  <page>
    <title>次</title>
    <ns>0</ns>
    <id>2</id>
    <revision>
      <id>20</id>
      <text>=={{ja}}==</text>
    </revision>
  </page>
</mediawiki>"#,
    )
    .unwrap();

    let pages: Vec<_> = xml_xtract::articles(&xml_path).collect();
    fs::remove_file(xml_path).unwrap();

    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].title, "空");
    assert_eq!(pages[0].revision.text, "");
    assert_eq!(pages[1].title, "次");
    assert_eq!(pages[1].revision.text, "=={{ja}}==");
}

#[test]
fn converts_wiktionary_fixture_end_to_end() {
    let output = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-output");
    let jion_output = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-jion-output");
    let ojp_output = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-ojp-output");
    let report = std::env::temp_dir().join("xml-xtract-wiktionary-jisyo-report");

    let status = Command::new(env!("CARGO_BIN_EXE_wiktionary_jisyo"))
        .arg("--xml")
        .arg("tests/fixtures/wiktionary_jisyo.xml")
        .arg("--mapping")
        .arg("tests/fixtures/wiktionary_jisyo_mapping.tsv")
        .arg("--jion-output")
        .arg(&jion_output)
        .arg("--ojp-output")
        .arg(&ojp_output)
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
    assert!(output_text.contains("がっこう /学校/"));
    assert!(output_text.contains("いどう /移動/"));
    assert!(!output_text.contains(" /移動/する/"));
    assert!(output_text.contains("あるk /歩/"));
    assert!(!output_text.contains("かんする /緘する/"));
    assert!(output_text.contains("えん /円/"));
    assert!(output_text.contains("たくさん /沢山/"));
    assert!(output_text.contains("べつべつ /別別/"));
    assert!(output_text.contains("べつべつ /別々/"));

    let ojp_output_text = fs::read_to_string(&ojp_output).unwrap();
    assert!(ojp_output_text.contains(";; okuri-ari entries."));
    assert!(ojp_output_text.contains(";; okuri-nasi entries."));
    assert!(ojp_output_text.contains("あるk /歩/"));

    let jion_output_text = fs::read_to_string(&jion_output).unwrap();
    assert!(jion_output_text.contains(";; okuri-ari entries."));
    assert!(jion_output_text.contains(";; okuri-nasi entries."));
    assert!(jion_output_text.contains("がくかう /学校/"));
    assert!(!jion_output_text.contains("あるk /歩/"));
    assert!(!jion_output_text.contains("いどう /移動/する/"));

    let report_text = fs::read_to_string(&report).unwrap();
    assert!(report_text.contains("metric\tcount"));
    assert!(report_text.contains("pages\t8"));
    assert!(report_text.contains("ojp_entries\t1"));
    assert!(report_text.contains("jion_entries\t1"));
    assert!(report_text.contains("shared_entries\t0"));
    assert!(report_text.contains("invalid_pages\t2"));
    assert!(report_text.contains("redirect_entries\t2"));

    fs::remove_file(output).unwrap();
    fs::remove_file(jion_output).unwrap();
    fs::remove_file(ojp_output).unwrap();
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
    assert!(report_text.contains("entries\t8"));

    fs::remove_file(report).unwrap();
}
