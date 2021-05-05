# SKKのWikitonary辞書

このリポジトリは日本語版Wikitonaryから生成したいくつかの辞書が含まれています。

* SKK_JISYO.shikakugoma: 四角号碼の辞書です
* SKK_JISYO.seikana: (experimental) 歴史的仮名遣い（正假名）の辞書です

# SKKの四角号碼辞書
## これは何？

SKKで使える[四角号碼](https://ja.wikipedia.org/wiki/四角号碼)辞書です。[Wiktionaryの漢字の記事](https://ja.wiktionary.org/wiki/カテゴリ:漢字https://ja.wiktionary.org/wiki/カテゴリ:漢字)から生成しています。

## 四角号碼入力について

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

# SKKの歴史的仮名遣い辞書
## これは何？

漢字を歴史的仮名遣いで変換するための（実験的な）辞書です。また、[アノテーション](http://openlab.ring.gr.jp/skk/wiki/wiki.cgi?page=annotation)も含みます。歴史的仮名遣いを普段から使っている人は正假名と呼んでいるようです。こっちの方が短いので以後こっちで説明します。

## 正假名入力について

漢字の音読みには同音なものが多く、例えば「しょう」だとSKK_JISYO.Lには174のエントリがあります。この中から「笑」を探すのは四葉のクローバーを探すくらい難しいですよね。でも「笑」の正假名「せう」だと候補は57に絞られます。同様に「渉」も「しょう」ですが正假名の「せふ」で変換すると候補は24に減ります。このように選択性の高い入力を使って変換のときに目grepする手間を少なくするのを目的としたのがこの辞書です。

正假名は1つ1つ覚えないといけないので使いはじめるのはちょっと大変です。そこで現代仮名遣いの候補のアノテーションに正假名の読みも付与しました。辞書には以下のようなエントリが含まれています。

```
せう /笑/
しょう /笑;セウ/
せふ /渉/
しょう /渉;セフ/
```

アノテーションをサポートしている辞書なら変換候補内に表示してくれます。正假名を覚えたいときは現代仮名遣いで一旦変換候補を出し、アノテーションを見て覚えてから再度正假名で変換するように習慣づければ覚えられるのではないかと思います。

## 使い方

普通のSKK辞書のように使えます。アノテーションをサポートしていないエンジンで問題が発生する場合は[unannotation.awk](http://openlab.jp/skk/skk/tools/unannotation.awk)などを利用して削除して下さい。

## experimental版の制限事項

Wiktionaryの漢字のページから生成しているので単一の漢字にのみ対応しています。熟語には対応していません。例えば「高笑」は正假名で「かうせう」ですが「かうせう」では変換できず、「かう」「せう」とそれぞれ変換しないといけません。

将来的には熟語に対応したいと考えていますが、ソースとなる情報源がなく、既存の辞書を泥臭く変換していく作業になりそうなので予定は未定です。

# ライセンス

* 辞書はWiktionaryのライセンスに従い[CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/deed.ja)で提供されます。
* その他のコード類はMITライセンスです。

# 自分で生成する

自分でデータを生成する人のために手順を示す。簡単には以下のコマンドでできる。

``` console
$ ./make.sh
```

`make.sh` を使わずに生成する場合は以下の手順を踏む。

### データの取得

[Wikimediaプロジェクトのダンプ](https://dumps.wikimedia.org/backup-index.html)のjawikitonaryの最新版にいく。そこから必要なデータをダウンロードする。必要なデータは以下の2つ。

* `jawiktionary-*-categorylinks.sql`
* `jawiktionary-*-pages-articles.xml`

ダウンロードしたらgzやbz2を解答しておく。

### MySQLのセットアップ

`categorylinks.sql` からデータを取り出すためにMySQLを立てる。dockerを使うと早い。

```console
$ docker run --name wiktionary --rm -e MYSQL_ALLOW_EMPTY_PASSWORD=true  -e MYSQL_DATABASE=wiktionary mysql
$ docker exec  -i wiktionary mysql wiktionary < jawiktionary-*-categorylinks.sql
```

まあまあの時間がかかる。

sqlite3でできたらよかったが、スキーマの `unsigned` に対応していないので無理そうだった。

### 漢字記事IDの取得

ここから「カテゴリー:漢字」に属する記事のIDを取得する。 `ids.txt` に出力する。

```console
$ docker exec -it wiktionary mysql wiktionary --skip-column-names -Be 'SELECT cl_from FROM categorylinks WHERE cl_to = 0xE6BCA2E5AD97 ORDER BY cl_from' > ids.txt
```

MySQLはもう不要なので落としておく

``` console
$ docker stop wiktionary
```

### 辞書生成

`cargo run` する。

```console
# 四角号碼辞書
$ cargo run --release --bin shikakugoma ids.txt jawiktionary-*-pages-articles.xml > output.log
# 正假名辞書辞書
$ cargo run --release --bin seikana ids.txt jawiktionary-*-pages-articles.xml > output.log
```

このデータは正しくソートされていないので `skkdic-sort` を使ってソートする

``` console
# 四角号碼辞書
$ cat header.txt > SKK_JISYO.shikakugoma
$ cat tmp.shikakugoma | skkdic-sort >> SKK_JISYO.shikakugoma
# 正假名辞書辞書
$ cat header.txt > SKK_JISYO.seikana
$ cat tmp.seikana | skkdic-sort >> SKK_JISYO.seikana
```

カレントディレクトリに辞書ができる。

```console
$ ls SKK_JISYO.*
SKK_JISYO.seikana  SKK_JISYO.shikakugoma
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

後片付けしておく

``` console
$ rm tmp.* ids.txt
```

# Future Work

* 自動更新
* 正假名の熟語対応
