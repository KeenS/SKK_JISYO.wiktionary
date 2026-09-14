# SKKのWiktionary辞書
このリポジトリは日本語版Wiktionaryから生成したいくつかの辞書が含まれています。

* SKK-JISYO.wiktionary: Wiktionaryをソースにした変換辞書です
* SKK-JISYO.shikakugoma: 四角号碼の変換辞書です
* SKK-JISYO.jion: 字音仮名遣いで変換する辞書です
* SKK-JISYO.ojp: 古典日本語（歴史的仮名遣い）の変換辞書です

## SKKのWiktionary辞書
### これは何？

SKK-JISYO.wiktionaryは日本語版Wiktionaryのページから生成した現代仮名遣いのSKK辞書です。ベースとなる辞書を目指しています。
SKK-JISYO.Lとの違いは以下の通りです。

|            | .L     | .wiktionary  |
|------------|--------|--------------|
| ライセンス | GPL    | CC BY-SA 4.0 |
| エントリ数 | 約18万 | 約5.5万       |

### 使い方

通常のSKK辞書として使えます。

## SKKの四角号碼辞書
### これは何？

SKK-JISYO.shikakugomaはSKKで使える[四角号碼](https://ja.wikipedia.org/wiki/四角号碼)辞書です。[Wiktionaryの漢字の記事](https://ja.wiktionary.org/wiki/カテゴリ:漢字https://ja.wiktionary.org/wiki/カテゴリ:漢字)から生成しています。

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

通常のSKK辞書として使えます。数字からの変換は「Q」から変換できます（SKKエンジンによって異なるかもしれません）。

1つの辞書しか扱えないSKKエンジンを使っている場合は[skkdic-expr2](http://openlab.ring.gr.jp/skk/wiki/wiki.cgi?page=%BC%AD%BD%F1%A5%E1%A5%F3%A5%C6%A5%CA%A5%F3%A5%B9%A5%C4%A1%BC%A5%EB)などで1つにまとめて下さい。

## SKKの古典日本語辞書

### これは何？

`SKK-JISYO.ojp` は日本語版Wiktionaryの古典日本語（`ojp` セクション）ら生成した辞書です。

この辞書には以下のようなエントリが含まれます。

```text
あるk /歩/
と /疾/
```

### 使い方

通常のSKK辞書として使えます。古典日本語の語彙だけを含むため、現代日本語用の`SKK-JISYO.wiktionary`の補助辞書という位置付けです。。

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

# ライセンス

* 辞書はWiktionaryのライセンスに従い[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/3.0/deed.ja)で提供されます。
* その他のコード類はMITライセンスです。

# 自分で生成する

自分でデータを生成する人のために手順を示す。簡単には以下のコマンドでwiktionaryからデータをダウンロードし、辞書の生成までできる。

``` console
$ ./make.sh
```

既にデータをダウンロードしてあるならそれを使うこともできる。

``` console
$ ./make.sh path/to/jawiktionary-latest-categorylinks.sql path/to/jawiktionary-latest-pages-articles.xml
```


`make.sh` を使わずに生成する場合は以下の手順を踏む。

### データの取得

[Wikimediaプロジェクトのダンプ](https://dumps.wikimedia.org/backup-index.html)のjawikitonaryの最新版にいく。そこから必要なデータをダウンロードする。必要なデータは以下の2つ。

* `jawiktionary-*-categorylinks.sql`
* `jawiktionary-*-pages-articles.xml`

ダウンロードしたらgzやbz2を解凍しておく。

### MySQLのセットアップ

`categorylinks.sql` からデータを取り出すためにMySQLを立てる。dockerを使うと早い。

```console
$ docker run --name wiktionary --rm -e MYSQL_ALLOW_EMPTY_PASSWORD=true  -e MYSQL_DATABASE=wiktionary mysql
$ docker exec  -i wiktionary mysql wiktionary < jawiktionary-*-categorylinks.sql
```

まあまあの時間がかかる。

sqlite3でできたら手軽でよかったが、スキーマの `unsigned` に対応していないので無理そうだった。

### 漢字記事IDの取得

ここから「カテゴリー:漢字」に属する記事のIDを取得する。 `ids.txt` に出力する。

```console
$ docker exec -it wiktionary mysql wiktionary --skip-column-names -Be 'SELECT cl_from FROM categorylinks WHERE cl_target_id = 90955 ORDER BY cl_from' > ids.txt
```

MySQLはもう不要なので落としておく

``` console
$ docker stop wiktionary
```

### 辞書生成

四角号碼と字音のデータを生成する


```console
# 四角号碼辞書と漢字読み対応表
$ cargo run --release --bin shikakugoma ids.txt jawiktionary-*-pages-articles.xml > output.log
$ cargo run --release --bin jion ids.txt jawiktionary-*-pages-articles.xml >> output.log
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
$ cat header.txt > SKK-JISYO.wiktionary
$ cat tmp.wiktionary | skkdic-sort | skkdic-expr2 > tmp.wiktionary.sorted
$ cargo run --release --bin sort_candidates -- \
     --xml jawiktionary-*-pages-articles.xml \
     --input tmp.wiktionary.sorted \
     --output SKK-JISYO.wiktionary
# 古典日本語辞書
$ cat header.txt > SKK-JISYO.ojp
$ cat tmp.ojp | skkdic-sort | skkdic-expr2 > tmp.ojp.sorted
$ cargo run --release --bin sort_candidates -- \
     --xml jawiktionary-*-pages-articles.xml \
     --input tmp.ojp.sorted \
     --output SKK-JISYO.ojp
# 四角号碼辞書
$ cat header.txt > SKK-JISYO.shikakugoma
$ cat tmp.shikakugoma | skkdic-sort | skkdic-expr2 >> SKK-JISYO.shikakugoma
# 字音
cat header.txt > SKK-JISYO.jion
cat tmp.jion tmp.wiktionary.jion | skkdic-sort | skkdic-expr2 > tmp.jion.sorted
cargo run --release --bin sort_candidates -- \
     --xml jawiktionary-*-pages-articles.xml \
     --input tmp.jion.sorted \
     --output SKK-JISYO.jion
```

`sort_candidates` は日本語版Wiktionaryの日本語セクション内での内部リンク回数を候補の使用頻度の近似値として使い、変換候補を多い順に並べ替えます。リンクされていない候補は元の順序を保ったまま後ろに置きます。

カレントディレクトリに辞書ができます。

```console
$ ls SKK-JISYO.*
SKK-JISYO.wiktionary  SKK-JISYO.jion  SKK-JISYO.shikakugoma
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


``` console
$ rm tmp.shikakugoma tmp.jion ids.txt
```

# Future Work

* 自動更新
* 例外辞書の整備
