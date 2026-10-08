#!/bin/sh
# templated by http://qiita.com/blackenedgold/items/c9e60e089974392878c8
set -e
usage() {
    cat <<HELP
NAME:
   $0 -- Generate dictionaries

SYNOPSIS:
  $0 CATLINK ARTICLES
  $0 [-h|--help]
  $0 [--verbose]
  $0 --gsi PDF_TEXT
  $0 --gsi

DESCRIPTION:
  Generate dictionaries from Wiktionary. Also download Unihan,
  CLDR annotations, and the EDRDG files, and write SKK-JISYO.unihan,
  SKK-JISYO.shikakugoma, SKK-JISYO.emoji, SKK-JISYO.jmnedict, and
  SKK-JISYO.jmdict.
  Give CATLINK as
  jawiktionary-*-categorylinks.sql and ARTICLES as
  jawiktionary-*-pages-articles.xml. 

  -h  --help      Print this help.
      --verbose   Enables verbose mode.
      --gsi       Generate only SKK-JISYO.gsi. With PDF_TEXT, use an
                  existing pdftotext output; otherwise download the
                  official Gazetteer of Japan PDF and extract it.

EXAMPLE:
  $ $0

HELP
}

check_dependency() {
    if ! command -v "$1" > /dev/null; then
        echo "$1 not installed"
        return 1
    fi
}

check_dependencies() {
    check_dependency wget
    check_dependency unzip
    check_dependency zcat
    check_dependency bzcat
    check_dependency docker
    check_dependency cargo
    check_dependency skkdic-sort
    check_dependency gzip
}

fetch_unihan() {
    (
        cd "$SCRIPT_DIR/data"
        echo "Fetching Unihan"
        wget -N https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip
        unzip -qo -j Unihan.zip Unihan_Readings.txt Unihan_DictionaryLikeData.txt
    )
}

# Latest final CLDR tag. main and a beta tag would move under a monthly build.
CLDR_TAG=release-48-2

# wget -N compares the local file with the remote basename. -O skips that
# check, so save ja.xml under its own name, then copy it aside. annotations
# and annotationsDerived both ship ja.xml and en.xml.
fetch_cldr_file() {
    kind=$1
    name=$2
    dest=$3
    dir="cldr-${kind}"
    mkdir -p "$dir"
    (
        cd "$dir"
        wget -N "https://raw.githubusercontent.com/unicode-org/cldr/${CLDR_TAG}/common/${kind}/${name}"
    )
    cp "$dir/$name" "$dest"
}

fetch_emoji() {
    (
        cd "$SCRIPT_DIR/data"
        echo "Fetching emoji and symbols"
        wget -N https://www.unicode.org/Public/emoji/latest/emoji-test.txt
        fetch_cldr_file annotations ja.xml emoji-annotations-ja.xml
        fetch_cldr_file annotations en.xml emoji-annotations-en.xml
        fetch_cldr_file annotationsDerived ja.xml emoji-annotations-derived-ja.xml
        fetch_cldr_file annotationsDerived en.xml emoji-annotations-derived-en.xml
        wget -N https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt
        wget -N https://www.unicode.org/Public/UCD/latest/ucd/NamesList.txt
    )
}

# JMdict_e and KANJIDIC2 are fetched here with JMnedict. Their generators
# are separate. gzip -dc keeps the .gz for wget -N on the next run.
fetch_edrdg() {
    (
        cd "$SCRIPT_DIR/data"
        echo "Fetching EDRDG"
        wget -N http://ftp.edrdg.org/pub/Nihongo/JMnedict.xml.gz
        wget -N http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz
        wget -N http://ftp.edrdg.org/pub/Nihongo/kanjidic2.xml.gz
        gzip -dc JMnedict.xml.gz > JMnedict.xml
        gzip -dc JMdict_e.gz > JMdict_e.xml
        gzip -dc kanjidic2.xml.gz > kanjidic2.xml
    )
}

check_gsi_dependencies() {
    check_dependency cargo
    check_dependency pdftotext
    check_dependency skkdic-sort
    check_dependency skkdic-expr2
}

fetch_data() {
    (
        cd "$SCRIPT_DIR/data"
        echo "Fetching data"
        wget -N \
             https://dumps.wikimedia.org/jawiktionary/latest/jawiktionary-latest-categorylinks.sql.gz \
             https://dumps.wikimedia.org/jawiktionary/latest/jawiktionary-latest-linktarget.sql.gz \
             https://dumps.wikimedia.org/jawiktionary/latest/jawiktionary-latest-pages-articles.xml.bz2
        echo "Decompressing data"
        if [ $(find . -mmin -5 | wc -l) = 0 ] ; then
            echo "No new update"
            return 0
        fi
        zcat  jawiktionary-latest-categorylinks.sql.gz   > jawiktionary-latest-categorylinks.sql
        zcat  jawiktionary-latest-linktarget.sql.gz   > jawiktionary-latest-linktarget.sql
        bzcat jawiktionary-latest-pages-articles.xml.bz2 > jawiktionary-latest-pages-articles.xml
    )
}

sort_candidates() {
    input=$1
    output=$2
    cargo run --release --bin sort_candidates -- \
        --xml "$ARTICLES" \
        --input "$input" \
        --output "$output"
}

fetch_gsi_data() {
    (
        cd "$SCRIPT_DIR/data"
        echo "Fetching Gazetteer of Japan"
        wget -N -O gazetteer-of-japan.pdf \
             https://www.gsi.go.jp/common/000238259.pdf
        echo "Extracting Gazetteer of Japan text"
        pdftotext -layout gazetteer-of-japan.pdf gazetteer-of-japan.txt
    )
}

generate_gsi() {
    (
        cd "$SCRIPT_DIR"
        echo "Checking GSI dependencies"
        check_gsi_dependencies
        echo "Generating GSI dictionary"
        cargo run --release --bin gsi -- \
            "$GSI_TEXT" \
            tmp.gsi
        skkdic-sort < tmp.gsi | skkdic-expr2 > tmp.gsi.sorted
        cat gsi-header.txt tmp.gsi.sorted > SKK-JISYO.gsi
    )
}

generate_post() {
    (
        cd "$SCRIPT_DIR"
        echo "Checking post dependencies"
        check_dependency cargo
        check_dependency skkdic-sort
        check_dependency skkdic-expr2
        echo "Generating post dictionary"
        cargo run --release --bin post -- \
            data/utf_ken_all.csv tmp.post
        skkdic-sort < tmp.post | skkdic-expr2 > tmp.post.sorted
        cat post-header.txt tmp.post.sorted > SKK-JISYO.post
    )
}

generate() {
    (
        cd "$SCRIPT_DIR"
        echo "Checking dependencies"
        check_dependencies
        fetch_unihan
        echo "Generating Unihan dictionary"
        cargo run --release --bin unihan_jisyo -- \
            data/Unihan_Readings.txt \
            tmp.unihan
        skkdic-sort < tmp.unihan | skkdic-expr2 > tmp.unihan.sorted
        cat unicode-header.txt tmp.unihan.sorted > SKK-JISYO.unihan
        echo "Generating four-corner dictionary"
        cargo run --release --bin shikakugoma -- \
            data/Unihan_DictionaryLikeData.txt \
            tmp.shikakugoma
        skkdic-sort < tmp.shikakugoma | skkdic-expr2 > tmp.shikakugoma.sorted
        cat unicode-header.txt tmp.shikakugoma.sorted > SKK-JISYO.shikakugoma
        fetch_emoji
        echo "Generating emoji dictionary"
        cargo run --release --bin emoji_jisyo -- \
            data/emoji-test.txt \
            data/emoji-annotations-ja.xml \
            data/emoji-annotations-en.xml \
            data/emoji-annotations-derived-ja.xml \
            data/emoji-annotations-derived-en.xml \
            data/UnicodeData.txt \
            data/NamesList.txt \
            tmp.emoji
        skkdic-sort < tmp.emoji | skkdic-expr2 > tmp.emoji.sorted
        cat unicode-header.txt tmp.emoji.sorted > SKK-JISYO.emoji
        fetch_edrdg
        echo "Generating JMnedict dictionary"
        cargo run --release --bin jmnedict_jisyo -- \
            data/JMnedict.xml \
            tmp.jmnedict
        skkdic-sort < tmp.jmnedict | skkdic-expr2 > tmp.jmnedict.sorted
        cat edrdg-header.txt tmp.jmnedict.sorted > SKK-JISYO.jmnedict
        echo "Generating JMdict dictionary"
        cargo run --release --bin jmdict_jisyo -- \
            data/JMdict_e.xml \
            tmp.jmdict
        skkdic-sort < tmp.jmdict | skkdic-expr2 > tmp.jmdict.sorted
        cat edrdg-header.txt tmp.jmdict.sorted > SKK-JISYO.jmdict
        echo "Running MySQL"
        docker run --name wiktionary -d --rm -e MYSQL_ALLOW_EMPTY_PASSWORD=true  -e MYSQL_DATABASE=wiktionary mysql
        echo "Waiting MySQL"
        while ! docker exec wiktionary mysql wiktionary -e 'SELECT 1' 2> /dev/null ;  do
            printf "."
            sleep 1
        done
        echo
        echo "Preparing Database"
        docker exec  -i wiktionary mysql wiktionary < "$CATLINK"
        docker exec  -i wiktionary mysql wiktionary < "$LINKTARGET"
        echo "Extracting page ids of kanji articles"
        # 漢字 encoded as UTF-8. The linktarget id is assigned per dump import.
        docker exec -i wiktionary mysql wiktionary --skip-column-names -B -e 'SELECT cl_from FROM categorylinks WHERE cl_target_id = (SELECT lt_id FROM linktarget WHERE lt_namespace = 14 AND lt_title = 0xE6BCA2E5AD97) ORDER BY cl_from' > ids.txt
        if [ ! -s ids.txt ]; then
            echo "漢字 category id was not found in linktarget" >&2
            exit 1
        fi
        echo "Stopping MySQL"
        docker stop wiktionary
        echo "Generating prototype of dictionaries"
        cargo run --release --bin jion ids.txt "$ARTICLES" > output_jion.log
        echo "Generating Wiktionary dictionary"
        cargo run --release --bin wiktionary_jisyo -- \
            --xml "$ARTICLES" \
            --mapping kanji_readings.tsv \
            --output tmp.wiktionary \
            --jion-output tmp.wiktionary.jion \
            --ojp-output tmp.ojp \
            --report wiktionary-jisyo-report.tsv
        echo "Generating dictionaries"
        cat tmp.wiktionary | skkdic-sort | skkdic-expr2 > tmp.wiktionary.sorted
        cat header.txt tmp.wiktionary.sorted > tmp.wiktionary.headered
        sort_candidates tmp.wiktionary.headered SKK-JISYO.wiktionary

        echo "Generating ojp dictionary"
        cat tmp.ojp | skkdic-sort | skkdic-expr2 > tmp.ojp.sorted
        cat header.txt tmp.ojp.sorted > tmp.ojp.headered
        sort_candidates tmp.ojp.headered SKK-JISYO.ojp

        echo "Generating jion dictionary"
        cat tmp.jion tmp.wiktionary.jion | skkdic-sort | skkdic-expr2 > tmp.jion.sorted
        cat header.txt tmp.jion.sorted > tmp.jion.headered
        sort_candidates tmp.jion.headered SKK-JISYO.jion
        echo "Cleaning up"
        rm tmp.* ids.txt
    )

}

main() {
    SCRIPT_DIR="$(cd $(dirname "$0"); pwd)"

    while [ $# -gt 0 ]; do
        case "$1" in
        --help) usage; exit 0;;
        --verbose) set -x; shift;;
        --gsi)
            shift
            if [ $# -gt 0 ]; then
                GSI_TEXT=$1
                shift
            else
                fetch_gsi_data
                GSI_TEXT=data/gazetteer-of-japan.txt
            fi
            generate_gsi
            exit 0
            ;;
            --post)
            generate_post
            exit 0
            ;;
            --) shift; break;;
            -*)
                OPTIND=1
                while getopts h OPT "$1"; do
                    case "$OPT" in
                        h) usage; exit 0;;
                    esac
                done
                shift
                ;;
            *) break;;
        esac
    done

    if [ $# = 3 ]; then
        CATLINK="$1"
        LINKTARGET="$2"
        ARTICLES="$3"
    else
        fetch_data
        CATLINK=data/jawiktionary-latest-categorylinks.sql
        LINKTARGET=data/jawiktionary-latest-linktarget.sql
        ARTICLES=data/jawiktionary-latest-pages-articles.xml
    fi
    generate

}

main "$@"
