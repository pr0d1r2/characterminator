# SPEC

## §G GOAL

How loud a finding is: lint names, groups, levels, hazard, pedantic.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §V INVARIANTS

V33: severity ∈ `error` (default, exit 1) | `warn` (reported, exit 0); per rule suffix `!<severity>` (`docs/** caveman !warn`); last matching rule naming one wins; twin via `--rule` (`src/rules:V18`). `--strict` → warn counts as error (clippy `-D warnings` shape). json violation ! carry `severity`. [superseded by V36]
V34: `hazard` set, from vendored Unicode properties: ∀ `Default_Ignorable_Code_Point` (ZWSP, word joiner, soft hyphen, VS outside declared emoji sequences, Hangul fillers, invisible math ops, tag chars U+E0000–U+E007F) ∪ bidi controls U+202A–U+202E, U+2066–U+2069 ∪ C0/C1 controls ∖ `\t` `\n` `\r` ∪ BOM ∉ file start. ZWJ ∈ declared emoji sequence (`src/fix:V31`) exempt; ZWJ/ZWNJ via CLDR language preset REJECTED: `forbid` ⊥ lowered by later rule | flag (V36) ∴ a preset-driven exemption contradicts it. hazard hit = forbid level ∀ rule (V36) ∴ `any` grants every char & a hazard STILL fires — the exclusion lives in the LEVEL, ⊥ in the set model, ∵ `src/charset:V3` forbids subtraction & "everything minus hazard" is ⊥ expressible. ⊥ lowered, ⊥ even by naming the `hazard` group or one of its lints (V36) ∴ prose that needs ZWNJ/ZWJ outside emoji (fa, hi) has ⊥ way through yet; a scoped exemption ! be specified before one ships. bidi controls ∉ builtin map ∴ reported, ⊥ auto-removed.
V36: ∀ check = named lint in a group; levels rustc/clippy shape `allow` | `warn` | `deny` | `forbid` (`forbid` ⊥ lowered by later rule | flag). groups: `hazard` (forbid, V34) · `charset` (deny, `src/rules:V1`, `src/rules:V2`, `src/charset:V3`) · `pedantic` (allow). rule suffix `!<level>` → `charset`; `!<lint|group>=<level>` → named one. `--strict` → warn ⇒ deny. deny | forbid hit → exit 1. json ! carry lint name & level.
V37: `pedantic` group = maximum purity, opt-in (`--pedantic` ≡ `--rule '* !pedantic=warn'`), clippy::pedantic philosophy: ? flags legit text ∴ ⊥ default. lints, all SHIP (group stays `allow` ∴ ⊥ fire until asked): `not-nfc` · `nfkc-compat` (fullwidth, ligatures, superscripts → ASCII-foldable) · `unicode-space` (NBSP, U+2000–U+200A) · `mixed-script` · `confusable` (UTS #39) · `crlf` · `trailing-whitespace` · `final-newline` · `locale-literal` (non-ASCII string literal in a code file → move to a locale file, `src/rules:V17`). lint → default group only after near-zero false positives in dogfood waves.
V49: hazard = 1 lint per class, first class holding the char wins: `bidi-control` (Bidi_Control) · `tag-character` · `stray-bom` · `control-character` (Cc ∖ `\t` `\n` `\r`) · `invisible` (rest of Default_Ignorable). char ∈ hazard ∧ ∉ set → 1 finding, the hazard; report names set `hazard`. classes read from COMPILED-IN data (`src/charset:V22`), ⊥ run's catalog ∴ `.ctrm-sets` | `--no-builtin-sets` ⊥ empty them. ZWJ exemption (V34) needs sequences ∴ lands w/ `src/fix:T33`; til then ∀ ZWJ fires, ⊥ approximated.

## §T TASKS

id|status|task|cites
T35|x|[superseded by T38 & `src/cli:T55`] severity: `!<level>` in rules (T38), json lint & level (T38), `--strict` flag (`src/cli:T55`)|V33,`src/render:V11`
T36|x|ARCHIVED to SPEC-ARCHIVE.md|V34,V36,V49
T38|x|ARCHIVED to SPEC-ARCHIVE.md|V36
T39|.|pedantic lints per V37 candidates, each w/ fixture of a legit false positive|V37,V36
