# SKKのWiktionary・地名・Unihan・CLDR・EDRDG辞書
このリポジトリは日本語版Wiktionary、国土地理院の地名データ、Unicode の Unihan、CLDR の注釈、EDRDG の辞書から生成したいくつかの辞書が含まれています。全てエンコーディングはUTF-8です。

* SKK-JISYO.wiktionary: Wiktionaryをソースにした変換辞書です
* SKK-JISYO.shikakugoma: 四角号碼の変換辞書です
* SKK-JISYO.jion: 字音仮名遣いで変換する辞書です
* SKK-JISYO.ojp: 古典日本語（歴史的仮名遣い）の変換辞書です
* SKK-JISYO.gsi: 地名集日本から生成した地名の変換辞書です
* SKK-JISYO.post: 日本郵便の郵便番号データから生成した住所の変換辞書です
* SKK-JISYO.unihan: Unicodeに含まれる漢字の辞書です
* SKK-JISYO.emoji: 絵文字、ギリシャ文字、記号の辞書です
* SKK-JISYO.jmnedict: 人名、地名、駅、会社、組織、製品、作品の辞書です。GPL の SKK-JISYO.jinmei とは別の辞書です
* SKK-JISYO.jmdict: 語彙の辞書です
* SKK-JISYO.kanjidic: 漢字一字の音読み、訓読み、名乗りの辞書です

# ライセンス

* `SKK-JISYO.wiktionary`、`SKK-JISYO.jion`、`SKK-JISYO.ojp` はWiktionaryのライセンスに従い[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/deed.ja)で提供されます。
* `SKK-JISYO.gsi` は国土地理院のデータを加工して生成しており、公共データ利用規約（第1.0版）に基づき[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/deed.ja)互換の条件で提供されます。
* `SKK-JISYO.post` は日本郵便の郵便番号データを加工して生成しており、日本郵便の利用規約に基づき提供されます。
* `SKK-JISYO.unihan`、`SKK-JISYO.shikakugoma`、`SKK-JISYO.emoji` は [Unicode License v3](https://www.unicode.org/license.txt) で提供されます。CC BY-SA ではありません。これらの辞書を単体でコピーする場合も、ファイルに入っている許諾表示を残して下さい。
* `SKK-JISYO.jmnedict`、`SKK-JISYO.jmdict`、`SKK-JISYO.kanjidic` は James William Breen と Electronic Dictionary Research and Development Group の [JMnedict](https://www.edrdg.org/enamdict/enamdict_doc.html)、[JMdict](https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project)、[KANJIDIC](https://www.edrdg.org/wiki/index.php/KANJIDIC_Project) を加工して生成しており、[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/deed.ja) で提供されます。条件は [EDRDG のライセンス](https://www.edrdg.org/edrdg/licence.html) にあります。`自動更新` が元データを取り直します。Wiktionary の許諾表示とは別です。
* その他のコード類はMITライセンスです。

# 辞書について
SKK-JISYO.wiktionaryとSKK-JISYO.jmdictはベースとなる辞書を目指しています。SKK-JISYO.Lとの違いは以下の通りです。`SKK-JISYO.general` は「1つの辞書にまとめる」の普段の変換用、`SKK-JISYO.all` はこのリポジトリの辞書をすべて入れたものです。

|            | .L     | .wiktionary  | .jmdict      | .general     | .all         |
|------------|--------|--------------|--------------|--------------|--------------|
| ライセンス | GPL    | CC BY-SA 4.0 | CC BY-SA 4.0 | 各辞書の許諾 | 各辞書の許諾 |
| エントリ数 | 約18万 | 約6.9万      | 約16万       | 約65万       | 約67万       |

SKK-JISYO.wiktionaryは音便などの活用に強く、これをベースに他の辞書で補完すると上手く使えるでしょう。

## SKKのWiktionary辞書
### これは何？

SKK-JISYO.wiktionaryは[日本語版Wiktionary](https://ja.wiktionary.org)のページから生成した現代仮名遣いのSKK辞書です。ベースとなる辞書を目指しています。また、Wiktionaryを編集することで、間接的にですが誰でもこの辞書にエントリを追加することができます

### 使い方

通常のSKK辞書として使えます。

## SKKの語彙辞書
### これは何？

`SKK-JISYO.jmdict` は[JMDICT](http://jedict.com/HTML/edict_doc.html)にある一般的な語彙の辞書です。見出しは読み、候補は漢字の表記です。これはかなりの単語をカバーしていますが、一部wiktionaryの辞書にしかないエントリもあるので相補的に利用下さい

```text
たb /食/
かk /書/
あu /会/
たかi /高/
いk /行/
にほんご /日本語/
べんきょう /勉強/
```

送り仮名がある動詞と形容詞は、語幹とかなの子音を見出しにします。`行く` は `いk /行/`、`会う` は `あu /会/` です。`勉強` は `べんきょう /勉強/` です。同じ見出しでは、優先タグのある候補が先に並びます。

### 使い方

一般的な語を変換するときに、`SKK-JISYO.wiktionary` と並べて使います。複数辞書をサポートしていない場合は、「1つの辞書にまとめる」のコマンドで SKK-JISYO.wiktionary とまとめて利用下さい。

## SKKの固有名詞辞書
### これは何？

SKK-JISYO.jmnedict は[JMDICT](http://jedict.com/HTML/edict_doc.html)から人名、地名、駅名、会社名、組織名、製品名、作品名を取り出した辞書です。wiktionaryの弱い固有名詞を補完する目的で作られました。アノテーションは `姓`、`名`、`人名`、`地名`、`駅`、`会社`、`組織`、`製品`、`作品` です。

```text
やまだ /山田;姓/
しんじゅくえき /新宿駅;駅/
ANC /ＡＮＣ;組織/
```

見出しはひらがな（`ー` と `・` を含む）か、英字と数字です。ラテン文字をかなに開いた読みは見出しにしません。

再配布するときは、James William Breen と EDRDG の表示を残して下さい。

### 使い方

人名、駅名、会社名、製品名を変換するときに使います。通常のSKK辞書として使えます。複数辞書をサポートしていない場合は、「1つの辞書にまとめる」のコマンドで SKK-JISYO.wiktionary とまとめて利用下さい。

## SKKのUnihan辞書
### これは何？

`SKK-JISYO.unihan` は Unicode に含まれる漢字の辞書です。音読みと訓読みがある字が入ります。見出しはひらがなの読み全体で、送り仮名の見出しはありません。読みが記録されていない字は入りません。

Wiktionary に記事がない漢字も入ります。「こう」のようなありふれた音読みには、その読みの漢字がすべて並ぶので候補は長くなります。`.wiktionary` とはライセンスが異なるので再配布の際はご注意下さい。

たとえば次のように変換できます。

```text
あかるい /明/
ぎょう /硤/
```

### 使い方

通常のSKK辞書として使えます。`SKK-JISYO.wiktionary` にない漢字を読みから変換するときの補助辞書です。特に変換できなかった候補をその場で登録できるSKKにおいてはひとまず単一の漢字を全て出せるというのは非常に重要になります。候補が増えて変換に手間がかかるデメリットとのトレードオフですがきっと有用でしょう。
複数辞書をサポートしていない場合は、「1つの辞書にまとめる」のコマンドで SKK-JISYO.wiktionary とまとめて利用下さい。

## SKKの一字辞書
### これは何？

`SKK-JISYO.kanjidic` は[KANJIDIC](https://www.japaneselanguagetools.com/docs/Kanjidic.html)から生成した辞書です。漢字一字の音読み、訓読み、名乗りの辞書です。

```text
たb /食/
ひとt /一/
やま /山/
はじめ /一/
かず /一/
ひとつ /一/
こう /硤/
ぎょう /硤/
```

送り仮名のある訓は `たb /食/` や `ひとt /一/` です。送り仮名のない訓は `やま /山/` です。名乗りは `はじめ /一/`、`かず /一/`、`ひとつ /一/` のように、読みごとに分かれます。`こう` と `ぎょう` には `硤` が入り、`そ` には入りません。再配布するときは、James William Breen と EDRDG の表示を残して下さい。

### 使い方

漢字一字を変換するときに使います。その字の名乗りもここから入ります。漢字だけでいうとUnihanの方がカバー率は高いですが、Unicodeのものと比べて送りがなありのエントリが含まれていたりUnicodeない珍しい読みなどを拾えます。

複数辞書をサポートしていない場合は、「1つの辞書にまとめる」のコマンドで SKK-JISYO.wiktionary とまとめて利用下さい。

## SKKの絵文字辞書
### これは何？

`SKK-JISYO.emoji` はUnicodeに含まれる絵文字、現代ギリシャ文字、かなで書ける記号の辞書です。絵文字の見出しはかなの語と、英語の読み上げ名をつないだアルファベットです。ギリシャ文字の見出しは文字名です。`kappa` は `κ`、`Kappa` は `Κ`、`lamda` と `lambda` は `λ`、`sigma` は `σ` と `ς` です。記号では `いんふぃにてぃ` が `∞`、`ゆーろ` が `€` です。

`にっこり` と `grinningface` は `😀`、`はーと` と `redheart` は `❤️`、`flagjapan` は `🇯🇵` です。候補に U+FE0F が入ることがあります。古い SKK では、U+FE0F や ZWJ を含む候補を 1 文字として確定できないことがあります。

`.wiktionary` とはライセンスが異なるので再配布の際はご注意下さい。

たとえば次のように変換できます。

```text
にっこり /😀/
はーと /❤️/
grinningface /😀/
kappa /κ/
いんふぃにてぃ /∞/
ゆーろ /€/
さんかく /▲/▼/
やじるし /→/
こめじるし /※/
くろまる /●/
```

### 使い方

絵文字を日本語の語やアルファベットの名前から、ギリシャ文字を文字名から、記号をかなの外来語と、辞書から戻した漢字の読みから入力するときに使います。通常のSKK辞書として使えます。複数辞書をサポートしていない場合は、「1つの辞書にまとめる」のコマンドで SKK-JISYO.wiktionary とまとめて利用下さい。

## SKKの住所辞書
### これは何？
`SKK-JISYO.post` は[日本郵便の郵便番号-住所対応表](https://www.post.japanpost.jp/service/search/zipcode/download/utf-zip.html)から生成したSKK辞書です。国土地理院のデータでカバーできない字(あざ)レベルの地名を拾えるのでこちらもSKK-JISYO.gsiと併せてSKK-JISYO.wiktionaryの補助的な辞書として使うことを意図しています。利用規約は以下になっています。

> 郵便番号データに限っては日本郵便株式会社は著作権を主張しません。自由に配布していただいて結構です。

住所辞書は住所の読みからそのままの漢字変化の他、市区町村、都道府県を加えた住所を変換できます

``` console
まるのうち /丸の内/米沢市丸の内/山形県米沢市丸の内/丸内/寒河江市丸内/山形県寒河江市丸内/西白河郡矢吹町丸の内/福島県西白河郡矢吹 町丸の内/千代田区丸の内/東京都千代田区丸の内/富山市丸の内/富山県富山市丸の内/高岡市丸の内/富山県高岡市丸の内/氷見市丸の内/富山県氷見市丸の内/金沢市丸の内/石川県金沢市丸の内/甲府市丸の内/山梨県甲府市丸の内/松本市丸の内/長野県松本市丸の内/大垣市丸の内/岐阜県大垣市丸の内/名古屋市中区丸の内/愛知県名古屋市中区丸の内/丸之内/津市丸之内/三重県津市丸之内/名張市丸之内/三重県名張市丸之内/岡山市北区丸の内/岡山県岡山市北区丸の内/福山市丸之内/広島県福山市丸之内/高松市丸の内/香川県高松市丸の内/松山市丸之内/愛媛県松山市丸之内/宇和島市丸之内/愛媛県宇和島市丸之内/丸ノ内/高知市丸ノ内/高知県高知市丸ノ内/
```

また、郵便番号から住所を変換することもできます。

``` console
1000001 /東京都千代田区千代田/
```

## 使い方

通常のSKK辞書として使えます。Wiktionaryでは地名は網羅されていないのでSKK-JISYO.wiktionaryの補助的な辞書として使うことを意図しています。
また、郵便番号からの変換は意外と便利なことがあります。数字からの変換は「Q」から変換できます（SKKエンジンによって異なるかもしれません）。

1つの辞書しか扱えないSKKエンジンを使っている場合は[skkdic-expr2](http://openlab.ring.gr.jp/skk/wiki/wiki.cgi?page=%BC%AD%BD%F1%A5%E1%A5%F3%A5%C6%A5%CA%A5%F3%A5%B9%A5%C4%A1%BC%A5%EB)などで1つにまとめて下さい。


## SKKの地名辞書
### これは何？

`SKK-JISYO.gsi` は国土地理院・海上保安庁海洋情報部の [地名集日本](https://www.gsi.go.jp/kihonjohochousa/gazetteer.html)（2021年版）から生成したSKK辞書です。行政地名のほか、山、川、湖沼、海域、海底地形などの地名を含みます。.wiktionaryとはライセンスが異なるので再配布の際はご注意下さい。

行政地名からは正式名称に加えて、 `中津市` のような `市` `町` `村` `郡` `区` などの接尾語を除いた表記も登録します。たとえば次の両方を変換できます。

```text
なかつ /中津/
なかつし /中津市/
```

### 使い方

通常のSKK辞書として使えます。Wiktionaryでは地名は網羅されていないのでSKK-JISYO.wiktionaryの補助的な辞書として使うことを意図しています。郵便番号辞書は郵便番号が付与されている住所しかカバーしておらず、逆にこちらは字未満の「丸の内」などをカバーしていません。jmnedictは日本の地名を読みまで含めて完璧には収録しておらず例えば和水町(なごみまち)などは変換できませんが、上海などの海外地名をサポートしています。
相補的に利用下さい。

複数辞書をサポートしていない場合は、「1つの辞書にまとめる」のコマンドで SKK-JISYO.wiktionary とまとめて利用下さい。

## SKKの四角号碼辞書
### これは何？

`SKK-JISYO.shikakugoma` は[四角号碼](https://ja.wikipedia.org/wiki/四角号碼)で漢字に変換する辞書です。漢字の形から決まる4桁の番号と、附画を加えた5桁の番号が入っています。

`碼` は `1162` と `11627`、`酊` は `1162` と `11620`、`硤` は `1463` と `14638` です。`.wiktionary` とはライセンスが異なるので再配布の際はご注意下さい。

### 四角号碼入力について

漢字に対応する4つの数字（附画を加えると5つ）のコードから漢字に変換します。このコードは漢字の見た目から決まるので漢字ごとに番号を覚えたりしなくても変換したい漢字さえ思い浮かんでいれば変換できます。読み方が分からなくても大丈夫です。また、選択性が非常に高く、4文字のコードで候補を数個に絞り込めます。附画を加えるとさらに限定できます。

例えば「碼」に割り当てられた四角号碼は1162<sub>7</sub>なので以下のように入力すると

```
▽1162
```

以下のように変換できます。

```
▼碼
```

四角号碼の1162に該当する漢字は「碼」の他にも「酊」など4つほどその候補が全て出てきます。ですが附画の0を加えて11620とすると「酊」のみ出てきます。

```
▽11620
```

```
▼酊
```

## 使い方

通常のSKK辞書として使えます。数字からの変換は「Q」から変換できます（SKKエンジンによって異なるかもしれません）。読みが分からない漢字を形から出すときに使います。複数辞書をサポートしていない場合は、「1つの辞書にまとめる」のコマンドで SKK-JISYO.wiktionary とまとめて利用下さい。

## SKKの古典日本語辞書

### これは何？

`SKK-JISYO.ojp` は日本語版Wiktionaryの古典日本語（`ojp` セクション）ら生成した辞書です。

この辞書には以下のようなエントリが含まれます。

```text
あるk /歩/
と /疾/
```

### 使い方

通常のSKK辞書として使えます。古典日本語の語彙だけを含むため、現代日本語用の`SKK-JISYO.wiktionary`の補助辞書という位置付けです。

## SKKの字音仮名遣い辞書
### これは何？

SKK-JISYO.jionはSKKで[字音仮名遣い](https://ja.wikipedia.org/wiki/%E5%AD%97%E9%9F%B3%E4%BB%AE%E5%90%8D%E9%81%A3)を使って変換するためのツールです。また、[アノテーション](http://openlab.ring.gr.jp/skk/wiki/wiki.cgi?page=annotation)も含みます。

### 字音仮名入力について

漢字の音読みには同音なものが多く、例えば「しょう」だとSKK-JISYO.Lには174のエントリがあります。この中から「笑」を探すのは四葉のクローバーを探すくらい難しいですよね。でも「笑」の字音仮名「せう」だと候補は57に絞られます。同様に「渉」も「しょう」ですが字音仮名の「せふ」で変換すると候補は24に減ります。このように選択性の高い入力を使って変換のときに目grepする手間を少なくするのを目的としたのがこの辞書です。

字音仮名は1つ1つ覚えないといけないので使いはじめるのはちょっと大変です。そこで現代仮名遣いの候補のアノテーションに字音仮名の読みも付与しました。辞書には以下のようなエントリが含まれています。

```
せう /笑/
しょう /笑;セウ/
せふ /渉/
しょう /渉;セフ/
```

アノテーションをサポートしている辞書なら変換候補内に表示してくれます。字音仮名を覚えたいときは現代仮名遣いで一旦変換候補を出し、アノテーションを見て覚えてから再度字音仮名で変換するように習慣づければ覚えられるのではないかと思います。


### 使い方

普通のSKK辞書のように使えます。アノテーションをサポートしていないエンジンで問題が発生する場合は[unannotation.awk](http://openlab.jp/skk/skk/tools/unannotation.awk)などを利用して削除して下さい。

### 変換精度に関する注意

熟語の読みは必ずしも漢字ごとの音読みの単純な結合ではありません。促音化、拗音、連濁、音便などがあります。基本的な促音化と拗音を処理していますが、一意に分割できないものや対応する読みが見つからないものは辞書にありません。読み戻し検査（字音仮名から現代仮名遣いを復元して元の読みと一致するか）を通らないものは辞書に出力しません。

wiktionaryに載っている古音から生成しているので、原義の字音仮名遣いとはずれがあると思われます。

# 1つの辞書にまとめる

複数辞書をサポートしていない場合は、ダウンロードした辞書を `skkdic-expr2` で1つにまとめます。`skkdic-expr2` は skktools に入っています。先に書いたファイルの候補が前に残ります。ありふれた音読みの候補は長くなります。まとめたファイルには、各辞書の先頭にある許諾表示は入りません。再配布するときは、元のファイルの許諾表示を残して下さい。

このリポジトリの辞書をすべて入れるときは、次のコマンドを使います。字音仮名遣い、古典日本語、四角号碼の見出しも入ります。

``` console
$ { echo ';; -*- coding: utf-8 -*-'
  skkdic-expr2 \
    SKK-JISYO.wiktionary \
    + SKK-JISYO.jmdict \
    + SKK-JISYO.jmnedict \
    + SKK-JISYO.gsi \
    + SKK-JISYO.post \
    + SKK-JISYO.kanjidic \
    + SKK-JISYO.unihan \
    + SKK-JISYO.emoji \
    + SKK-JISYO.shikakugoma \
    + SKK-JISYO.jion \
    + SKK-JISYO.ojp
  } > SKK-JISYO.all
```

普段の変換には、現代の語彙、固有名詞、地名、住所、漢字の読み、絵文字をまとめます。

``` console
$ { echo ';; -*- coding: utf-8 -*-'
  skkdic-expr2 \
    SKK-JISYO.wiktionary \
    + SKK-JISYO.jmdict \
    + SKK-JISYO.jmnedict \
    + SKK-JISYO.gsi \
    + SKK-JISYO.post \
    + SKK-JISYO.kanjidic \
    + SKK-JISYO.unihan \
    + SKK-JISYO.emoji
  } > SKK-JISYO.general
```

できたファイルを登録して使います。辞書の登録については各自で使用しているOSやIMEに合わせて行なって下さい。

# 自分で生成する

自分でデータを生成する人のために手順を示す。`make.sh` を使う方法と使わない方法がある。

# `make.sh` を使う場合

## Wiktionary系辞書

Wiktionary、四角号碼、字音、古典日本語の各辞書を生成する。データをダウンロードしていない場合は `make.sh` がダウンロードから行う。

``` console
$ ./make.sh
```

このコマンドは `SKK-JISYO.unihan`、`SKK-JISYO.shikakugoma`、`SKK-JISYO.emoji`、`SKK-JISYO.jmnedict`、`SKK-JISYO.jmdict`、`SKK-JISYO.kanjidic` も生成します。EDRDG の JMnedict、JMdict_e、KANJIDIC2 もダウンロードします。

既にダウンロードしてあるならデータを指定する。

``` console
$ ./make.sh path/to/jawiktionary-latest-categorylinks.sql path/to/jawiktionary-latest-linktarget.sql path/to/jawiktionary-latest-pages-articles.xml
```

## 地名辞書

[地名集日本](https://www.gsi.go.jp/kihonjohochousa/gazetteer.html)からダウンロードしたPDFから生成する。データをダウンロードしていない場合は `make.sh` がダウンロードから行う。

``` console
$ ./make.sh --gsi
```

既にPDFからテキストを抽出してあるならテキストを指定する。

``` console
$ ./make.sh --gsi data/gazetteer-of-japan.txt
```

## 住所辞書

[日本郵便の郵便番号-住所対応表](https://www.post.japanpost.jp/service/search/zipcode/download/utf-zip.html)からダウンロードしたCSVから生成する。データをダウンロードしていない場合は `make.sh` がダウンロードから行う。

``` console
$ ./make.sh --post
```

# `make.sh` を使わない場合

## Wikitonaryデータの取得

[Wikimediaプロジェクトのダンプ](https://dumps.wikimedia.org/backup-index.html)のjawikitonaryの最新版にいく。そこから必要なデータをダウンロードする。必要なデータは以下の3つ。

* `jawiktionary-*-categorylinks.sql`
* `jawiktionary-*-linktarget.sql`
* `jawiktionary-*-pages-articles.xml`

ダウンロードしたらgzやbz2を解凍しておく。

## MySQLのセットアップ

`categorylinks.sql` からデータを取り出すためにMySQLを立てる。dockerを使うと早い。

```console
$ docker run --name wiktionary --rm -e MYSQL_ALLOW_EMPTY_PASSWORD=true  -e MYSQL_DATABASE=wiktionary mysql
$ docker exec  -i wiktionary mysql wiktionary < jawiktionary-*-categorylinks.sql
$ docker exec  -i wiktionary mysql wiktionary < jawiktionary-*-linktarget.sql
```

まあまあの時間がかかる。

sqlite3でできたら手軽でよかったが、スキーマの `unsigned` に対応していないので無理そうだった。

## 漢字記事IDの取得

ここから「カテゴリー:漢字」に属する記事のIDを取得する。 `ids.txt` に出力する。

```console
$ docker exec -i wiktionary mysql wiktionary --skip-column-names -B -e 'SELECT cl_from FROM categorylinks WHERE cl_target_id = (SELECT lt_id FROM linktarget WHERE lt_namespace = 14 AND lt_title = 0xE6BCA2E5AD97) ORDER BY cl_from' > ids.txt
```

MySQLはもう不要なので落としておく

``` console
$ docker stop wiktionary
```

## Wiktionary系辞書の生成

字音のデータを生成する

```console
$ cargo run --release --bin jion ids.txt jawiktionary-*-pages-articles.xml > output.log
```

辞書や対応表などを生成する

``` console
$ cargo run --release --bin wiktionary_jisyo -- \
     --xml jawiktionary-*-pages-articles.xml \
     --mapping kanji_readings.tsv \
     --output tmp.wiktionary \
     --jion-output tmp.wiktionary.jion \
     --report wiktionary-jisyo-report.tsv
```

このデータは正しくソートされていないので `skkdic-sort` を使ってソートし、 `skkdic-expr2` でエントリをまとめる

``` console
# wiktionary辞書
$ cat tmp.wiktionary | skkdic-sort | skkdic-expr2 > tmp.wiktionary.sorted
$ cat header.txt tmp.wiktionary.sorted > tmp.wiktionary.headered
$ cargo run --release --bin sort_candidates -- \
     --xml jawiktionary-*-pages-articles.xml \
     --input tmp.wiktionary.headered \
     --output SKK-JISYO.wiktionary
# 古典日本語辞書
$ cat tmp.ojp | skkdic-sort | skkdic-expr2 > tmp.ojp.sorted
$ cat header.txt tmp.ojp.sorted > tmp.ojp.headered
$ cargo run --release --bin sort_candidates -- \
     --xml jawiktionary-*-pages-articles.xml \
     --input tmp.ojp.headered \
     --output SKK-JISYO.ojp
# 字音
cat tmp.jion tmp.wiktionary.jion | skkdic-sort | skkdic-expr2 > tmp.jion.sorted
cat header.txt tmp.jion.sorted > tmp.jion.headered
cargo run --release --bin sort_candidates -- \
     --xml jawiktionary-*-pages-articles.xml \
     --input tmp.jion.headered \
     --output SKK-JISYO.jion
```

`sort_candidates` は日本語版Wiktionaryの日本語セクション内での内部リンク回数を候補の使用頻度の近似値として使い、変換候補を多い順に並べ替える。リンクされていない候補は元の順序を保ったまま後ろに置く。

カレントディレクトリに辞書ができる。

```console
$ ls SKK-JISYO.*
SKK-JISYO.wiktionary  SKK-JISYO.jion
```

wikitionaryに適切な情報が載ってないものもあるので `output.log` にはそれらの情報が出力されている。

```console
$ head output.log
氷: no match
仏: no match
権: no match
県: no match
塩: no match
争: no match
蝉: no match
続: no match
総: no match
鉃: no match
```

中間生成ファイルは削除しておく


``` console
$ rm tmp.jion ids.txt
```

### 四角号碼辞書の生成

`Unihan_DictionaryLikeData.txt` は `./make.sh` が `Unihan.zip` から取り出します。次のコマンドで辞書を生成します。

``` console
$ cargo run --release --bin shikakugoma -- Unihan_DictionaryLikeData.txt tmp.shikakugoma
$ skkdic-sort < tmp.shikakugoma | skkdic-expr2 > tmp.shikakugoma.sorted
$ cat unicode-header.txt tmp.shikakugoma.sorted > SKK-JISYO.shikakugoma
```

``` console
$ ls SKK-JISYO.shikakugoma
SKK-JISYO.shikakugoma
```

### Unihan辞書の生成

[Unihan.zip](https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip) をダウンロードして `Unihan_Readings.txt` を取り出してから、次のコマンドで辞書を生成します。

``` console
$ wget -N https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip
$ cargo run --release --bin unihan_jisyo -- Unihan_Readings.txt tmp.unihan
$ skkdic-sort < tmp.unihan | skkdic-expr2 > tmp.unihan.sorted
$ cat unicode-header.txt tmp.unihan.sorted > SKK-JISYO.unihan
```

``` console
$ ls SKK-JISYO.unihan
SKK-JISYO.unihan
```

### 絵文字辞書の生成

`./make.sh` が次のファイルを `data/` に置きます。リポジトリのルートで辞書を生成します。末尾の2つは漢字の記号名を読みに戻す辞書で、先に生成しておきます。`make.sh` は `SKK-JISYO.wiktionary` と `SKK-JISYO.jmdict` のあとで絵文字辞書を作ります。

``` console
$ cargo run --release --bin emoji_jisyo -- \
    data/emoji-test.txt \
    data/emoji-annotations-ja.xml \
    data/emoji-annotations-en.xml \
    data/emoji-annotations-derived-ja.xml \
    data/emoji-annotations-derived-en.xml \
    data/UnicodeData.txt \
    data/NamesList.txt \
    tmp.emoji \
    SKK-JISYO.wiktionary \
    SKK-JISYO.jmdict
$ skkdic-sort < tmp.emoji | skkdic-expr2 > tmp.emoji.sorted
$ cat unicode-header.txt tmp.emoji.sorted > SKK-JISYO.emoji
```

``` console
$ ls SKK-JISYO.emoji
SKK-JISYO.emoji
```

### 固有名詞辞書の生成

`./make.sh` が `data/JMnedict.xml.gz`、`data/JMdict_e.gz`、`data/kanjidic2.xml.gz` をダウンロードし、`data/JMnedict.xml` を展開します。リポジトリのルートで辞書を生成します。

``` console
$ cargo run --release --bin jmnedict_jisyo -- data/JMnedict.xml tmp.jmnedict
$ skkdic-sort < tmp.jmnedict | skkdic-expr2 > tmp.jmnedict.sorted
$ cat edrdg-header.txt tmp.jmnedict.sorted > SKK-JISYO.jmnedict
```

``` console
$ ls SKK-JISYO.jmnedict
SKK-JISYO.jmnedict
```

### 語彙辞書の生成

`./make.sh` が `data/JMdict_e.gz` をダウンロードし、`data/JMdict_e.xml` を展開します。リポジトリのルートで辞書を生成します。

``` console
$ cargo run --release --bin jmdict_jisyo -- data/JMdict_e.xml tmp.jmdict
$ skkdic-sort < tmp.jmdict | skkdic-expr2 > tmp.jmdict.sorted
$ cat edrdg-header.txt tmp.jmdict.sorted > SKK-JISYO.jmdict
```

``` console
$ ls SKK-JISYO.jmdict
SKK-JISYO.jmdict
```

### 一字辞書の生成

`./make.sh` が `data/kanjidic2.xml.gz` をダウンロードし、`data/kanjidic2.xml` を展開します。リポジトリのルートで辞書を生成します。

``` console
$ cargo run --release --bin kanjidic_jisyo -- data/kanjidic2.xml tmp.kanjidic
$ skkdic-sort < tmp.kanjidic | skkdic-expr2 > tmp.kanjidic.sorted
$ cat edrdg-header.txt tmp.kanjidic.sorted > SKK-JISYO.kanjidic
```

``` console
$ ls SKK-JISYO.kanjidic
SKK-JISYO.kanjidic
```

## 地名データのダウンロードとテキスト抽出
[地名集日本](https://www.gsi.go.jp/kihonjohochousa/gazetteer.html)のpdfデータをdata/にダウンロードしておく

``` console
$ wget -N -O gazetteer-of-japan.pdf https://www.gsi.go.jp/common/000238259.pdf
```

そこからテキストデータを抽出する

``` console
$ pdftotext -layout data/gazetteer-of-japan.pdf data/gazetteer-of-japan.txt
```

## 地名辞書の生成

コマンドを使って辞書を生成する

``` console
$ cargo run --release --bin gsi -- data/gazetteer-of-japan.txt tmp.gsi
$ skkdic-sort < tmp.gsi | skkdic-expr2 > tmp.gsi.sorted
$ cat gsi-header.txt tmp.gsi.sorted > SKK-JISYO.gsi
```

辞書ができる

``` console
$ ls SKK-JISYO.gsi
SKK-JISYO.gsi
```

中間生成ファイルは削除しておく

``` console
$ rm tmp.gsi
```

## 郵便データのダウンロード

[日本郵便の郵便番号-住所対応表](https://www.post.japanpost.jp/service/search/zipcode/download/utf-zip.html)からダウンロードしておく

``` console
$ wget https://www.post.japanpost.jp/zipcode/utf/zip/utf_ken_all.zip
$ unzip utf_ken_all.zip
$ mv utf_ken_all.csv data/utf_ken_all.csv
```

## 住所辞書の生成

コマンドを使って辞書を生成する

``` console
$ cargo run --release --bin post -- data/utf_ken_all.csv tmp.post
$ skkdic-sort < tmp.post | skkdic-expr2 > tmp.post.sorted
$ cat post-header.txt tmp.post.sorted > SKK-JISYO.post
```

中間生成ファイルは削除しておく

``` console
$ rm tmp.post tmp.post.sorted
```

# 自動更新

GitHub Actions で毎月 1 回辞書を再生成しています。同じ実行で Unihan、CLDR の注釈、JMnedict、JMdict、KANJIDIC2 も取り直し、`SKK-JISYO.unihan`、`SKK-JISYO.emoji`、`SKK-JISYO.jmnedict`、`SKK-JISYO.jmdict`、`SKK-JISYO.kanjidic` をコミットします。四角号碼辞書もその Unihan から作り直します。ワークフローは [.github/workflows/monthly-build.yml](.github/workflows/monthly-build.yml) です。

# Future Work

* 例外辞書の整備
