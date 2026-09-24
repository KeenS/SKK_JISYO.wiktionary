use xml_xtract::gsi::pdf_text::{generate_jisyo, municipality_base_entry};

#[test]
fn generates_dictionary_entries_from_pdf_rows() {
    let jisyo = generate_jisyo(
        r#"     Grid   Japanese(Kanji)   Japanese(Kana)   Romanized Japanese   Latitude   Longitude   Classification
2270   5031   中津市    なかつし    Nakatsu Shi    33°36'   131°11'   Municipality
4117   5740   蔵王山   ざおうざん（ざおうさん）   Zao Zan (Zao San)   38°08'   140°27'   Mountain
"#,
    );

    assert_eq!(jisyo.get("なかつ").unwrap().to_line(), "なかつ /中津/\n");
    assert_eq!(
        jisyo.get("なかつし").unwrap().to_line(),
        "なかつし /中津市/\n"
    );
    assert_eq!(
        jisyo.get("ざおうざん").unwrap().to_line(),
        "ざおうざん /蔵王山/\n"
    );
    assert_eq!(
        jisyo.get("ざおうさん").unwrap().to_line(),
        "ざおうさん /蔵王山/\n"
    );
}

#[test]
fn keeps_municipality_base_name_when_kana_has_an_alternate() {
    let row = "2270   5031   中津市    なかつし（なかづし）    Nakatsu Shi (Nakazu Shi)    33°36'   131°11'   Municipality";
    let jisyo = generate_jisyo(row);
    assert_eq!(jisyo.get("なかつ").unwrap().to_line(), "なかつ /中津/\n");
    assert_eq!(
        jisyo.get("なかつし").unwrap().to_line(),
        "なかつし /中津市/\n"
    );
    assert_eq!(
        jisyo.get("なかづし").unwrap().to_line(),
        "なかづし /中津市/\n"
    );
}

#[test]
fn skips_short_name_for_a_non_municipality() {
    let row = "2270   5031   中津市    なかつし（なかづし）    Nakatsu Shi (Nakazu Shi)    33°36'   131°11'   Mountain";
    let jisyo = generate_jisyo(row);
    assert!(jisyo.get("なかつ").is_none());
    assert_eq!(
        jisyo.get("なかつし").unwrap().to_line(),
        "なかつし /中津市/\n"
    );
    assert_eq!(
        jisyo.get("なかづし").unwrap().to_line(),
        "なかづし /中津市/\n"
    );
}

#[test]
fn generates_nakatsu_base_entry_from_municipality_row() {
    assert_eq!(
        municipality_base_entry("中津市", "なかつし", "Nakatsu Shi"),
        Some(("中津".to_string(), "し".to_string(), String::new()))
    );
}
