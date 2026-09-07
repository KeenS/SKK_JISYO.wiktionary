use xml_xtract::jion::entry::Entry;
use xml_xtract::jion::jisyo::{parse_line, read_jisyo, write_jisyo};
use xml_xtract::jion::mapping::{
    parse_line as parse_mapping, read_mapping, to_index, write_mapping, Mapping,
};
use xml_xtract::jion::segmentation::{segment, Segmentation};

#[test]
fn parses_jisyo_entry() {
    let entry = parse_line("ほうこう /方向;ハウカウ/").unwrap();
    assert_eq!(entry.reading, "ほうこう");
    assert_eq!(entry.candidates, vec!["方向"]);
    assert_eq!(entry.annotations, vec!["ハウカウ"]);
    assert_eq!(entry.to_line(), "ほうこう /方向;ハウカウ/\n");
}

#[test]
fn ignores_jisyo_headers() {
    assert!(parse_line(";; okuri-nasi entries.").is_none());
}

#[test]
fn round_trips_jisyo_entries() {
    let path = std::env::temp_dir().join("xml-xtract-jisyo-test");
    let entries = vec![Entry::new("ほうこう", "方向")];
    write_jisyo(&path, &entries).unwrap();
    assert_eq!(read_jisyo(&path).unwrap(), entries);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn parses_mapping_line() {
    let mapping = parse_mapping("向\tこう\tかう").unwrap();
    assert_eq!(
        mapping,
        Mapping {
            kanji: "向".into(),
            modern: "こう".into(),
            historical: "かう".into(),
        }
    );
}

#[test]
fn round_trips_mapping_entries() {
    let path = std::env::temp_dir().join("xml-xtract-mapping-test");
    let mappings = vec![Mapping {
        kanji: "方".into(),
        modern: "ほう".into(),
        historical: "はう".into(),
    }];
    write_mapping(&path, &mappings).unwrap();
    assert_eq!(read_mapping(&path).unwrap(), mappings);
    let index = to_index(&mappings);
    assert_eq!(index["方"].len(), 1);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn rejects_invalid_mapping_line() {
    assert!(parse_mapping("向\tこう").is_none());
}

#[test]
fn handles_multiple_candidates() {
    let entry = parse_line("かう /向/向く/").unwrap();
    assert_eq!(entry.candidates, vec!["向", "向く"]);
    assert!(entry.annotations.is_empty());
}

#[test]
fn segments_unique_reading() {
    let mappings = vec![
        Mapping {
            kanji: "方".into(),
            modern: "ほう".into(),
            historical: "はう".into(),
        },
        Mapping {
            kanji: "向".into(),
            modern: "こう".into(),
            historical: "かう".into(),
        },
    ];
    match segment("方向", "ほうこう", &mappings) {
        Segmentation::Unique(segments) => {
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0].modern, "ほう");
            assert_eq!(segments[0].historical, "はう");
            assert_eq!(segments[1].modern, "こう");
            assert_eq!(segments[1].historical, "かう");
        }
        result => panic!("expected unique segmentation, got {result:?}"),
    }
}

#[test]
fn detects_ambiguous_segmentation() {
    let mappings = vec![
        Mapping {
            kanji: "方".into(),
            modern: "ほう".into(),
            historical: "はう".into(),
        },
        Mapping {
            kanji: "向".into(),
            modern: "こう".into(),
            historical: "かう".into(),
        },
        Mapping {
            kanji: "向".into(),
            modern: "うこう".into(),
            historical: "うかう".into(),
        },
        Mapping {
            kanji: "方".into(),
            modern: "ほ".into(),
            historical: "ほ".into(),
        },
    ];
    match segment("方向", "ほうこう", &mappings) {
        Segmentation::Ambiguous(candidates) => assert_eq!(candidates.len(), 2),
        result => panic!("expected ambiguous segmentation, got {result:?}"),
    }
}

#[test]
fn reports_missing_segmentation() {
    let mappings = vec![Mapping {
        kanji: "方".into(),
        modern: "ほう".into(),
        historical: "はう".into(),
    }];
    assert_eq!(
        segment("方向", "ほうこう", &mappings),
        Segmentation::Missing
    );
}
