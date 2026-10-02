# SPEC

## §G GOAL

Rewrite a disallowed char SAFELY: map, families, equivalence classes, compression.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- file: `.ctrm-map` — transliteration, `<from> <to>`: `from` = literal char | `U+XXXX`; `to` = replacement, empty = explicit delete. `to` ∉ target file's set → char counts unmapped (V4). + `family <name> <parent>` (V27). + `= <class> <family>:<member>[,<member>...] ...` (V28). sequence = code points joined by `+` (`U+1F44D+U+1F3FD`) | written literally (the chars of `U+2714+U+FE0F`), ∀ `from` & ∀ member. + `word <from> <to>` (V51). + `use <name>` (V51).

## §V INVARIANTS

V4: `fix` replaces only via declared transliteration map. char w/o mapping → kept & reported, exit 1. ⊥ silent drop.
V5: `fix` idempotent: `fix(fix(x)) == fix(x)`, property-tested.
V6: `fix` touches ⊥ allowed char: bytes outside violations ! identical pre/post, asserted before write.
V26: typography = builtin map targets, ⊥ default grant (`src/charset:R4`: 45.1% of non-ASCII files need no grant after map): `—`→`--` · `–` `−`→`-` · curly quotes → straight · `…`→`...` · `«»`→`"` · NBSP → space · ZWSP & BOM → delete. `typography` set SHIPS ∴ prose that wants real typography grants it & `fix` leaves those chars (V6). per-locale `typography-<code>` from CLDR punctuation (`src/charset:R6`) DEFERRED til `src/charset:T32` vendors CLDR.
V27: character families = open tree, declared by map line `family <name> <parent>`; parent chain ! end @ `ascii` (intrinsic root); cycle → config error, exit 2. builtin: `ascii` ← `text` ← `emoji`. new family (e.g. `nerd`) = 1 line + members in classes.
V28: map line `= <class> <family>:<member>[,<member>...] ...` declares equivalence class; members labelled by family, first per family preferred. disallowed member → preferred member of rule's fidelity family if allowed, else along its fallback path (V27) → `ascii`; none allowed → unmapped (V4). `--map` twin takes this form (`src/rules:V18`). multi-codepoint members per V31.
V31: `emoji` preset = single code points only: ⊥ VS15/VS16, ⊥ skin tone U+1F3FB–U+1F3FF, ⊥ ZWJ U+200D, ⊥ keycap U+20E3, ⊥ tag chars ∴ ∀ sequence has a disallowed code point → builtin sequence map (vendored Unicode emoji data) compresses: selector & skin tone → delete (`👍🏽`→`👍`); ZWJ sequence → single code point equivalent if one exists (`U+1F468+U+200D+U+1F469+U+200D+U+1F467`→`👪`), else first emoji; keycap → its ASCII digit | `#` | `*`; flag → its 2-letter region code (`PL`). map `from` & class members ? be sequences (notation: §I); scan = longest declared sequence first. `emoji-seq` preset REJECTED: a set = CODE POINTS & a sequence ⊥ one ∴ ⊥ expressible as a set; reopens only if the set model itself gains sequences.
V51: `words` map OPT-IN, ⊥ default (V26): map line `use <name>` reads it AT that line (`src/rules:V19`), origin = that line; unknown → exit 2. `⊥`→not `∴`→so `∵`→because `∀`→all `∈`→in `∃`→exists `≠`→`!=` `→`→`->` `⇒`→`=>`, each ⊥ MORE o200k tok (`.:R11`). `word <from> <to>` = word entry: `\w` edge meets `\w` → 1 space (`⊥owns`→`not owns`), across a delete too; space = PART of replacement ∴ inside span for V6; V5 holds. plain entry ⊥ spaced. `--words` flag REJECTED: 2nd switch for 1 map line, ⊥ positional.
V60: builtin map DELETES VS15 U+FE0E, VS16 U+FE0F & skin tones U+1F3FB–U+1F3FF ∴ modifier sequence → its base (`👍🏽`→`👍`) w/ ⊥ sequence scan: each is 1 code point the `emoji` preset withholds (V31). = the cheap half of V31; ZWJ sequences, keycaps, flags → T33. a file granting them keeps them (V6).

## §T TASKS

id|status|task|cites
T10|x|ARCHIVED to SPEC-ARCHIVE.md|V4,V5,V6
T26|x|ARCHIVED to SPEC-ARCHIVE.md|V26,`src/charset:V22`
T29|x|ARCHIVED to SPEC-ARCHIVE.md|V27
T30|x|ARCHIVED to SPEC-ARCHIVE.md|V28,V27
T33|.|vendor Unicode emoji data; sequence map generator; longest-match scan|V31,`src/charset:V22`,`src/lint:V34`
T53|x|ARCHIVED to SPEC-ARCHIVE.md|V51,V5,V6,`.:R11`
T59|x|ARCHIVED to SPEC-ARCHIVE.md|V60,V31
