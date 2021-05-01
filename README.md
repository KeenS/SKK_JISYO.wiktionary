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

## ライセンス

* 辞書はWiktionaryのライセンスに従い[CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/deed.ja)で提供されます。
* その他のコード類はMITライセンスです。

## 自分で生成する

自分でデータを生成する人のために手順を示す。

### データの取得

[Wikimediaプロジェクトのダンプ](https://dumps.wikimedia.org/backup-index.html)のjawikitonaryの最新版にいく。そこから必要なデータをダウンロードする。必要なデータは以下の2つ。

* `jawiktionary-*-categorylinks.sql`
* `jawiktionary-*-pages-articles.xml`

ダウンロードしたらgzやbz2を解答しておく。

### MySQLのセットアップ

`categorylinks.sql` からデータを取り出すためにMySQLを立てる。dockerを使うと早い。

```console
$ docker run --name wiktionary --rm -e MYSQL_ALLOW_EMPTY_PASSWORD=true  -e MYSQL_DATABASE=wiktionary mysql
$ docker exec  -i wiktionary mysql wiktionary < jawiktionary-20210401-categorylinks.sql
```

まあまあの時間がかかる。

sqlite3でできたらよかったが、スキーマの `unsigned` に対応していないので無理そうだった。

### 漢字記事IDの取得

ここから「カテゴリー:漢字」に属する記事のIDを取得する。 `ids.txt` に出力する。

```console
$ docker exec -it wiktionary mysql wiktionary --skip-column-names -Be 'SELECT cl_from FROM categorylinks WHERE cl_to = 0xE6BCA2E5AD97 ORDER BY cl_from' > ids.txt
```

### 辞書生成

`cargo run` する。

```console
$ cargo run --release --bin shikakugoma ids.txt jawiktionary-*-pages-articles.xml > output.log
```

カレントディレクトリに辞書ができる。

```console
$ ls SKK_JISYO.shikakugoma
SKK_JISYO.shikakugoma
```

wikitionaryに四角号碼の情報が載ってないものもあるので `output.log` にはそれらの情報が出力されている。

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

# Future Work

* 手順の自動化
* 自動更新
* その他の辞書（歴史的仮名づかいなど）の生成
