#!/bin/sh
# Generates `locales.ctrm-sets`, the CLDR letter presets (V30), from a
# pinned cldr-json release. Two steps, so the network is never part of the
# step that has to be reproducible:
#
#   sh src/charset/cldr-letters.sh fetch <dir>
#   sh src/charset/cldr-letters.sh generate <dir> > src/charset/locales.ctrm-sets
#
# `generate` reads only <dir> and `hazard.ctrm-sets` beside this script, so
# the same inputs give the same bytes. Needs `jq` (the dev shell has it),
# `curl` for `fetch`, and `sha256sum`.
set -eu

TAG=48.2.3
UCD=18.0.0
CLDR_URL=https://raw.githubusercontent.com/unicode-org/cldr-json/$TAG/cldr-json/cldr-misc-full/main
UCD_URL=https://www.unicode.org/Public/$UCD/ucd/UnicodeData.txt

# The European Union's official languages less English (whose letters are
# ASCII), then the Latin, Cyrillic and Greek script languages of Europe
# with the most speakers outside it. No CJK: those sets run to thousands.
LOCALES="bg cs da de el es et fi fr ga hr hu it lt lv mt nl pl pt ro sk sl sv
cy is nb ru sr tr uk"

here=$(dirname "$0")

fetch() {
  for code in $LOCALES; do
    mkdir -p "$1/main/$code"
    curl -sfL -o "$1/main/$code/characters.json" \
      "$CLDR_URL/$code/characters.json"
  done
  curl -sfL -o "$1/UnicodeData.txt" "$UCD_URL"
}

header() {
  cat <<EOF
# THE LOCALE LETTER PRESETS (V30), GENERATED from CLDR and compiled in
# beside \`sets.ctrm-sets\` (V22). Do not edit a member by hand: regenerate
# with \`src/charset/cldr-letters.sh\`, which says how.
#
# SOURCE: cldr-json tag $TAG, \`cldr-misc-full/main/<code>/characters.json\`,
# from $CLDR_URL;
# uppercase forms from Unicode $UCD \`UnicodeData.txt\` (simple uppercase
# mapping). Unicode License v3; see \`docs/THIRD-PARTY-NOTICES.md\`.
#
# \`<code>\` = main \`exemplarCharacters\` plus their uppercase forms.
# \`<code>-aux\` = \`auxiliary\` plus uppercase forms, less \`<code>\`. Both
# less ASCII and less every member of \`hazard.ctrm-sets\`. A multi-character
# exemplar (\`{ch}\`, \`{a\` + combining circumflex \`}\`) is granted as its
# code points one by one: a set holds code points, and a sequence's
# combining mark is what a decomposed spelling of it needs. A locale with
# nothing left after the subtraction declares no set.
#
# Input sha256, as \`sha256sum\` prints them:
#
EOF
  (cd "$1" && sha256sum UnicodeData.txt $(for code in $LOCALES; do
    printf 'main/%s/characters.json\n' "$code"; done)) | sed 's/^/#   /'
  echo
}

# shellcheck disable=SC2016
FILTER='
def hex: ascii_upcase | explode
  | reduce .[] as $c (0; . * 16 + (if $c >= 65 then $c - 55 else $c - 48 end));
def digits: if . < 16 then "0123456789ABCDEF"[.:.+1]
  else ((. / 16 | floor) | digits) + ((. % 16) | digits) end;
def u4: digits | if length < 4 then ("000" + .)[-4:] else . end;
def runs: reduce .[] as $c ([];
  if length > 0 and .[-1][1] + 1 == $c then .[-1][1] = $c else . + [[$c, $c]] end);
def render: runs | map(if .[0] == .[1] then "U+\(.[0] | u4)"
  else "U+\(.[0] | u4)-U+\(.[1] | u4)" end) | join(" ");
def points: if test("[-\\\\]") then error("unhandled exemplar syntax: \(.)")
  else ltrimstr("[") | rtrimstr("]") | explode
  | map(select(. != 32 and . != 123 and . != 125)) end;
($ucd | split("\n") | map(split(";") | select(length > 12 and .[12] != ""))
  | map({key: .[0], value: (.[12] | hex)}) | from_entries) as $upper
| ($hz | split("\n") | map(select(startswith("#") | not)) | join(" ")
  | [scan("U\\+([0-9A-F]+)(?:-U\\+([0-9A-F]+))?")]
  | map([(.[0] | hex), ((.[1] // .[0]) | hex)])) as $hazard
| def keep: map(select(. > 127) | . as $c
    | select(any($hazard[]; .[0] <= $c and $c <= .[1]) | not));
  def cased: . + map($upper[u4] // empty) | unique;
inputs | .main | to_entries[0] | .key as $code | .value.characters
| (.exemplarCharacters | points | cased | keep) as $main
| (((.auxiliary // "[]") | points | cased | keep) - $main) as $aux
| (if $main == [] then empty else "\($code) \($main | render)" end),
  (if $aux == [] then empty else "\($code)-aux \($aux | render)" end)
'

generate() {
  dir=$1
  header "$dir"
  set -- $(for code in $LOCALES; do
    printf '%s/main/%s/characters.json\n' "$dir" "$code"; done)
  jq -nr --rawfile ucd "$dir/UnicodeData.txt" \
    --rawfile hz "$here/hazard.ctrm-sets" "$FILTER" "$@"
}

case "${1:-}" in
  fetch) fetch "$2" ;;
  generate) generate "$2" ;;
  *) echo "usage: $0 fetch|generate <dir>" >&2; exit 2 ;;
esac
