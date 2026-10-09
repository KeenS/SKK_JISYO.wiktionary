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
fn converts_katakana_readings_to_hiragana() {
    let jisyo = generate_jisyo(
        "\
140   6343   アポイ岳    アポイだけ    Apoi Dake    42°06'   143°02'   Mountain
3   6443   ニセコ町    ニセコちょう    Niseko Cho    42°48'   140°41'   Municipality
129   5338   南アルプス市    みなみアルプスし    Minami-alps Shi    35°36'   138°28'   Municipality
263   4739   ベヨネース列岩    ベヨネースれつがん    Beyonesu Retsugan    31°53'   139°55'   Island
334      千島・カムチャツカ海溝    ちしま・カムチャツカかいこう    Chishima-Kamuchatsuka Kaiko    42°10'   147°02'   Undersea feature
",
    );

    assert_eq!(
        jisyo.get("あぽいだけ").unwrap().to_line(),
        "あぽいだけ /アポイ岳/\n"
    );
    assert!(jisyo.get("アポイだけ").is_none());
    assert_eq!(
        jisyo.get("にせこちょう").unwrap().to_line(),
        "にせこちょう /ニセコ町/\n"
    );
    assert_eq!(jisyo.get("にせこ").unwrap().to_line(), "にせこ /ニセコ/\n");
    assert_eq!(
        jisyo.get("みなみあるぷすし").unwrap().to_line(),
        "みなみあるぷすし /南アルプス市/\n"
    );
    assert_eq!(
        jisyo.get("みなみあるぷす").unwrap().to_line(),
        "みなみあるぷす /南アルプス/\n"
    );
    assert_eq!(
        jisyo.get("べよねーすれつがん").unwrap().to_line(),
        "べよねーすれつがん /ベヨネース列岩/\n"
    );
    assert_eq!(
        jisyo.get("ちしま・かむちゃつかかいこう").unwrap().to_line(),
        "ちしま・かむちゃつかかいこう /千島・カムチャツカ海溝/\n"
    );
    for line in jisyo.render().lines() {
        let reading = line.split_once(" /").unwrap().0;
        assert!(
            reading.chars().all(|ch| !('ァ'..='ヶ').contains(&ch)),
            "{reading}"
        );
    }
}

#[test]
fn generates_nakatsu_base_entry_from_municipality_row() {
    assert_eq!(
        municipality_base_entry("中津市", "なかつし", "Nakatsu Shi"),
        Some(("中津".to_string(), "し".to_string(), String::new()))
    );
}
