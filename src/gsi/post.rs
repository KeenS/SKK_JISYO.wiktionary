use super::jisyo::Jisyo;

#[derive(Debug, PartialEq, Eq)]
pub struct Row {
    pub postal_code: String,
    pub prefecture_kana: String,
    pub city_kana: String,
    pub town_kana: String,
    pub prefecture: String,
    pub city: String,
    pub town: String,
}

/// Generate dictionary entries from Japan Post's utf_ken_all.csv.
pub fn generate_jisyo(rows: &[Row]) -> Jisyo {
    let mut jisyo = Jisyo::default();
    for row in rows {
        if row.town == "以下に掲載がない場合" || row.town.is_empty() {
            continue;
        }
        let town = strip_parenthetical(&row.town);
        if town.is_empty() {
            continue;
        }
        let prefecture_reading = kana_to_hiragana(&row.prefecture_kana);
        let city_reading = kana_to_hiragana(&row.city_kana);
        let town_reading = kana_to_hiragana(&row.town_kana);
        let city_candidate = format!("{}{}", row.prefecture, row.city);
        let town_candidate = format!("{}{}", row.city, town);
        let address = format!("{}{}", row.prefecture, town_candidate);
        jisyo.add_entry(&row.postal_code, &address);
        jisyo.add_entry(&prefecture_reading, row.prefecture.clone());
        jisyo.add_entry(&city_reading, city_candidate);
        if !town_reading.is_empty() {
            jisyo.add_entry(&town_reading, town);
            jisyo.add_entry(&town_reading, town_candidate.clone());
            jisyo.add_entry(&town_reading, &address);
        }
    }
    jisyo
}

fn kana_to_hiragana(kana: &str) -> String {
    strip_parenthetical(kana)
        .chars()
        .map(|ch| {
            if ('ア'..='ヶ').contains(&ch) {
                char::from_u32(ch as u32 - 0x60).unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect()
}

fn strip_parenthetical(value: &str) -> &str {
    value.split_once('（').map_or(value, |(head, _)| head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_address_entries() {
        let rows = vec![Row {
            postal_code: "1000005".into(),
            prefecture_kana: "トウキョウト".into(),
            city_kana: "チヨダク".into(),
            town_kana: "マルノウチ".into(),
            prefecture: "東京都".into(),
            city: "千代田区".into(),
            town: "丸の内".into(),
        }];
        let jisyo = generate_jisyo(&rows);
        assert_eq!(
            jisyo.render(),
            "1000005 /東京都千代田区丸の内/\nちよだく /東京都千代田区/\nとうきょうと /東京都/\nまるのうち /丸の内/千代田区丸の内/東京都千代田区丸の内/\n"
        );
        assert_eq!(
            jisyo.get("とうきょうと").unwrap().to_line(),
            "とうきょうと /東京都/\n"
        );
        assert_eq!(
            jisyo.get("ちよだく").unwrap().to_line(),
            "ちよだく /東京都千代田区/\n"
        );
        assert_eq!(
            jisyo.get("まるのうち").unwrap().to_line(),
            "まるのうち /丸の内/千代田区丸の内/東京都千代田区丸の内/\n"
        );
    }
}
