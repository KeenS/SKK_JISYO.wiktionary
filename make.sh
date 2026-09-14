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
  Generate dictionaries from Wiktionary. Give CATLINK as
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
    check_dependency zcat
    check_dependency bzcat
    check_dependency docker
    check_dependency cargo
    check_dependency skkdic-sort
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
             https://dumps.wikimedia.org/jawiktionary/latest/jawiktionary-latest-pages-articles.xml.bz2
        echo "Decompressing data"
        if [ $(find . -mmin -5 | wc -l) = 0 ] ; then
            echo "No new update"
            return 1
        fi
        zcat  jawiktionary-latest-categorylinks.sql.gz   > jawiktionary-latest-categorylinks.sql
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

generate() {
    (
        cd "$SCRIPT_DIR"
        echo "Checking dependencies"
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
        echo "Extracting page ids of kanji articles"
        # 漢字 = 0xE6BCA2E5AD97
        # namespace 14: category
        # SELECT lt_id FROM linktarget WHERE lt_title = 0xE6BCA2E5AD97 AND lt_namespace = 14;
        # -> 90955
        docker exec -i wiktionary mysql wiktionary --skip-column-names -B -e 'SELECT cl_from FROM categorylinks WHERE cl_target_id = 90955 ORDER BY cl_from' > ids.txt
        echo "Stopping MySQL"
        docker stop wiktionary
        echo "Generating prototype of dictionaries"
        cargo run --release --bin shikakugoma ids.txt "$ARTICLES" > output_shikakugoma.log
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
        cat tmp.shikakugoma | skkdic-sort | skkdic-expr2 > tmp.shikakugoma.sorted
        cat header.txt tmp.shikakugoma.sorted > SKK-JISYO.shikakugoma

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

    if [ $# = 2 ]; then
        CATLINK="$1"
        ARTICLES="$2"
    else
        fetch_data || exit 1
        CATLINK=data/jawiktionary-latest-categorylinks.sql
        ARTICLES=data/jawiktionary-latest-pages-articles.xml
    fi
    generate

}

main "$@"
