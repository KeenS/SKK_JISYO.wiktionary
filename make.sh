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

DESCRIPTION:
  Generate dictionaries from Wiktionary. Give CATLINK as
  jawiktionary-*-categorylinks.sql and ARTICLES as
  jawiktionary-*-pages-articles.xml. 

  -h  --help      Print this help.
      --verbose   Enables verbose mode.

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
        echo "Preparing Database"
        docker exec  -i wiktionary mysql wiktionary < "$CATLINK"
        echo "Extracting page ids of kanji articles"
        docker exec -it wiktionary mysql wiktionary --skip-column-names -Be 'SELECT cl_from FROM categorylinks WHERE cl_to = 0xE6BCA2E5AD97 ORDER BY cl_from' > ids.txt
        echo "Stopping MySQL"
        docker stop wiktionary
        echo "Generating prototype of dictionaries"
        cargo run --release --bin shikakugoma ids.txt "$ARTICLES" > output_shikakugoma.log
        cargo run --release --bin seikana ids.txt "$ARTICLES" > output_seikana.log
        echo "Generating dictionaries"
        cat header.txt > SKK_JISYO.shikakugoma
        cat tmp.shikakugoma | skkdic-sort >> SKK_JISYO.shikakugoma
        cat header.txt > SKK_JISYO.seikana
        cat tmp.seikana | skkdic-sort >> SKK_JISYO.seikana
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

    CATLINK="$1"
    ARTICLES="$2"

    if fetch_data; then
        generate
    fi
}

main "$@"

