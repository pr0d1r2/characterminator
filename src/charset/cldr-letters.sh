#!/bin/sh
# Generates `locales.ctrm-sets`, the CLDR letter presets (V30, V61), from a
# pinned cldr-json release. Two steps, so the network is never part of the
# step that has to be reproducible:
#
#   sh src/charset/cldr-letters.sh fetch <dir>
#   sh src/charset/cldr-letters.sh generate <dir> > src/charset/locales.ctrm-sets
#
# `generate` reads only <dir> and `hazard.ctrm-sets` beside this script, so
# the same inputs give the same bytes. Needs `jq` (the dev shell has it),
# `curl` and `xargs` for `fetch`, and `sha256sum`.
set -eu

TAG=48.2.3
UCD=18.0.0
CLDR=https://raw.githubusercontent.com/unicode-org/cldr-json/$TAG/cldr-json
CLDR_URL=$CLDR/cldr-misc-full/main
UCD_URL=https://www.unicode.org/Public/$UCD/ucd/UnicodeData.txt

# Every locale `cldr-misc-full` ships (V61), the parent chain that decides
# which of them is an alias, the likely scripts that chain consults, and
# the default-content codes (`pt-BR`, `de-DE`) CLDR ships no file for
# because their data IS their parent's.
CORE="availableLocales.json defaultContent.json
supplemental/parentLocales.json supplemental/likelySubtags.json"

here=$(dirname "$0")

locales() {
  jq -r '.availableLocales.full[]' "$1/availableLocales.json"
}

fetch() {
  for file in $CORE; do
    curl -sfL -o "$1/$(basename "$file")" "$CLDR/cldr-core/$file"
  done
  locales "$1" | xargs -P 16 -I{} sh -c 'mkdir -p "$1/main/$2" &&
    curl -sfL -o "$1/main/$2/characters.json" "$3/$2/characters.json"' \
    sh "$1" {} "$CLDR_URL"
  curl -sfL -o "$1/UnicodeData.txt" "$UCD_URL"
}

# The sha256 of every input: the core files one by one, and the
# `characters.json` files as ONE digest of their `sha256sum` listing, so a
# header of 766 lines does not ship in the binary to say what one line can.
digests() {
  (cd "$1" && sha256sum UnicodeData.txt availableLocales.json \
    defaultContent.json parentLocales.json likelySubtags.json
  locales . | sed 's|.*|main/&/characters.json|' | xargs sha256sum \
    | sha256sum | sed 's/-$/main\/*\/characters.json/')
}

header() {
  cat <<EOF2
# THE LOCALE LETTER PRESETS (V30, V61), GENERATED from CLDR and compiled
# in beside \`sets.ctrm-sets\` (V22) but parsed LAZILY: a run reads the
# line of a name only when its rules use that name and nothing else
# declares it. Do not edit a member by hand: regenerate with
# \`src/charset/cldr-letters.sh\`, which says how.
#
# SOURCE: cldr-json tag $TAG, every locale \`availableLocales.json\`
# lists as full, \`cldr-misc-full/main/<code>/characters.json\`, from
# $CLDR_URL;
# the parent chain from \`parentLocales.json\` and \`likelySubtags.json\`;
# uppercase forms from Unicode $UCD \`UnicodeData.txt\` (simple uppercase
# mapping). Unicode License v3; see \`docs/THIRD-PARTY-NOTICES.md\`.
#
# \`<code>\` = main \`exemplarCharacters\` plus their uppercase forms.
# \`<code>-aux\` = \`auxiliary\` plus uppercase forms, less \`<code>\`. Both
# less ASCII and less every member of \`hazard.ctrm-sets\`. A range
# (\`a-z\`) and an escape (\`\\uXXXX\`) are expanded. A multi-character
# exemplar (\`{ch}\`, \`{a\` + combining circumflex \`}\`) is granted as its
# code points one by one: a set holds code points, and a sequence's
# combining mark is what a decomposed spelling of it needs. A locale
# whose letters are all ASCII (\`en\`, \`id\`) is ONE alias line naming
# \`ascii\` (\`en ascii\`), so its code resolves and grants ASCII only. An
# \`-aux\` with nothing left after the subtraction declares no set.
#
# A regional or script variant whose set equals its CLDR parent's is ONE
# alias line naming the parent (\`pt-BR pt\`), so a variant resolves and
# no data is written twice. The parent is what \`parentLocales\` names;
# else none for a script-only locale whose script is not its language's
# likely one (\`sr-Latn\`); else the code less its last subtag. A
# default-content code (\`pt-BR\`), which CLDR ships no file for because
# its data is its parent's, is always an alias, listed after the rest.
#
# Input sha256, as \`sha256sum\` prints them; the last is the sha256 of
# \`sha256sum\` over all $(locales "$1" | wc -l | tr -d ' ') \`characters.json\`, in \`availableLocales\` order:
#
EOF2
  digests "$1" | sed 's/^/#   /'
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
# One token of a UnicodeSet: an escape is always a code point, an unescaped
# hyphen the range operator, braces and spaces only group or separate.
def token: if test("^\\\\[uU]") then .[2:] | hex
  elif startswith("\\x{") then .[3:-1] | hex
  elif startswith("\\") then .[1:] | explode[0]
  elif . == "-" then "range"
  elif . == "{" or . == "}" or . == " " then empty
  elif test("^[\\[\\]$&^]") then error("unhandled exemplar syntax: \(.)")
  else explode[0] end;
def points: ltrimstr("[") | rtrimstr("]")
  | [scan("\\\\u[0-9A-Fa-f]{4}|\\\\U[0-9A-Fa-f]{8}|\\\\x\\{[0-9A-Fa-f]+\\}|\\\\.|.")
    | token]
  | reduce .[] as $t ({out: [], span: false};
      if $t == "range" and .out == [] then .out = [45]
      elif $t == "range" then .span = true
      elif .span then .out += [range(.out[-1] + 1; $t + 1)] | .span = false
      else .out += [$t] end)
  | .out;
($ucd | split("\n") | map(split(";") | select(length > 12 and .[12] != ""))
  | map({key: .[0], value: (.[12] | hex)}) | from_entries) as $upper
| ($hz | split("\n") | map(select(startswith("#") | not)) | join(" ")
  | [scan("U\\+([0-9A-F]+)(?:-U\\+([0-9A-F]+))?")]
  | map([(.[0] | hex), ((.[1] // .[0]) | hex)])) as $hazard
| ($pl[0].supplemental.parentLocales.parentLocale) as $explicit
| ($ls[0].supplemental.likelySubtags) as $likely
| def keep: map(select(. > 127) | . as $c
    | select(any($hazard[]; .[0] <= $c and $c <= .[1]) | not));
  def cased: . + map($upper[u4] // empty) | unique;
  def own: .main | to_entries[0] | {key: .key, value: (.value.characters
    | (.exemplarCharacters | points | cased | keep) as $main
    | {main: $main, aux: (((.auxiliary // "[]") | points | cased | keep)
        - $main)})};
  [inputs | own] as $order
| ($order | from_entries) as $data
| def parent: (if $explicit[.] then $explicit[.]
    else split("-") as $p
    | if ($p | length) == 1 then "und"
      elif ($p | length) == 2 and ($p[1] | test("^[A-Z][a-z]{3}$"))
      then (if ($likely[$p[0]] // "" | split("-")[1]) == $p[1]
            then $p[0] else "und" end)
      else $p[:-1] | join("-") end end)
    | if . == "und" or $data[.] then . else parent end;
  def line($code; $part; $suffix):
    $data[$code][$part] as $own | ($code | parent) as $up
    | if $own == [] and $part == "aux" then empty
      elif $up != "und" and $data[$up][$part] == $own
      then "\($code)\($suffix) \($up)\($suffix)"
      elif $own == [] then "\($code) ascii"
      else "\($code)\($suffix) \($own | render)" end;
  def alias($code; $part; $suffix): ($code | parent) as $up
    | if $up == "und" and $part == "main" then "\($code) ascii"
      elif $up == "und" or ($part == "aux" and $data[$up][$part] == [])
      then empty
      else "\($code)\($suffix) \($up)\($suffix)" end;
  ($order[] | .key | line(.; "main"; ""), line(.; "aux"; "-aux")),
  ($dc[0].defaultContent[] | select($data[.] | not)
    | alias(.; "main"; ""), alias(.; "aux"; "-aux"))
'

generate() {
  dir=$1
  header "$dir"
  set -- $(locales "$dir" | sed "s|.*|$dir/main/&/characters.json|")
  jq -nr --rawfile ucd "$dir/UnicodeData.txt" \
    --rawfile hz "$here/hazard.ctrm-sets" \
    --slurpfile pl "$dir/parentLocales.json" \
    --slurpfile dc "$dir/defaultContent.json" \
    --slurpfile ls "$dir/likelySubtags.json" "$FILTER" "$@"
}

case "${1:-}" in
  fetch) fetch "$2" ;;
  generate) generate "$2" ;;
  *) echo "usage: $0 fetch|generate <dir>" >&2; exit 2 ;;
esac
