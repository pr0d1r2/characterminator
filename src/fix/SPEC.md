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

V4: `fix` replaces only via declared transliteration map. char w/o mapping → kept & reported as `check` row (json `unmapped`), exit 1. ⊥ silent drop.
V5: `fix` idempotent: `fix(fix(x)) == fix(x)`, property-tested.
V6: `fix` touches ⊥ allowed char: bytes outside violations ! identical pre/post, asserted before write.
V26: typography = builtin map targets, ⊥ default grant (`src/charset:R4`: 45.1% of non-ASCII files need no grant after map): `—`→`--` · `–` `−`→`-` · curly quotes → straight · `…`→`...` · `«»`→`"` · NBSP → space · ZWSP & BOM → delete. `typography` set SHIPS ∴ prose that wants real typography grants it & `fix` leaves those chars (V6). per-locale `typography-<code>` from CLDR punctuation (`src/charset:R6`) DEFERRED til `src/charset:T32` vendors CLDR.
V27: character families = open tree, declared by map line `family <name> <parent>`; parent chain ! end @ `ascii` (intrinsic root); cycle → config error, exit 2. builtin: `ascii` ← `text` ← `emoji`. new family (e.g. `nerd`) = 1 line + members in classes.
V28: map line `= <class> <family>:<member>[,<member>...] ...` declares equivalence class; members labelled by family, first per family preferred. disallowed member → preferred member of rule's fidelity family if allowed, else along its fallback path (V27) → `ascii`; none allowed → unmapped (V4). `--map` twin takes this form (`src/rules:V18`). multi-codepoint members per V31.
V31: `emoji` preset = single code points only: ⊥ VS15/VS16, ⊥ skin tone U+1F3FB–U+1F3FF, ⊥ ZWJ U+200D, ⊥ keycap U+20E3, ⊥ tag chars ∴ ∀ sequence has a disallowed code point → builtin sequence map (vendored Unicode emoji data) compresses: selector & skin tone → delete (`👍🏽`→`👍`); ZWJ sequence → single code point ⟺ V62's curated table names one (`U+1F468+U+200D+U+1F469+U+200D+U+1F467`→`👪`), else first emoji; keycap → its ASCII digit | `#` | `*`; flag → its 2-letter region code (`PL`). map `from` & class members ? be sequences (notation: §I); scan = longest declared sequence first. `emoji-seq` preset REJECTED: a set = CODE POINTS & a sequence ⊥ one ∴ ⊥ expressible as a set; reopens only if the set model itself gains sequences.
V51: `words` map OPT-IN, ⊥ default (V26): map line `use <name>` reads it AT that line (`src/rules:V19`), origin = that line; unknown → exit 2. `⊥`→not `∴`→so `∵`→because `∀`→all `∈`→in `∃`→exists `≠`→`!=` `→`→`->` `⇒`→`=>`, each ⊥ MORE o200k tok (`.:R11`). `word <from> <to>` = word entry: `\w` edge meets `\w` → 1 space (`⊥owns`→`not owns`), across a delete too; space = PART of replacement ∴ inside span for V6; V5 holds. plain entry ⊥ spaced. `--words` flag REJECTED: 2nd switch for 1 map line, ⊥ positional.
V60: builtin map DELETES VS15 U+FE0E, VS16 U+FE0F & skin tones U+1F3FB–U+1F3FF ∴ modifier sequence → its base (`👍🏽`→`👍`) w/ ⊥ sequence scan: each is 1 code point the `emoji` preset withholds (V31). = the cheap half of V31; ZWJ sequences, keycaps, flags → T33. a file granting them keeps them (V6).
V62: 2A = T33 data: pinned Unicode emoji 18.0 `emoji-sequences.txt` + `emoji-zwj-sequences.txt`, vendored in `src/charset` (`src/charset:V22`; sha256 in header, as `src/charset:V59`) ∴ 1 list feeds this map & the hazard exemption (`src/lint:V63`) → generated `emoji-seq.ctrm-map`, 1 line ∀ RGI sequence V60 ⊥ already covers, `from` = the sequence (§I notation), longest match = existing scan. targets: keycap `<k> U+FE0F U+20E3` → `<k>` (digit | `#` | `*`) · flag (2 regional indicators) → its region code (`PL`) · tag flag (U+1F3F4 + tags) → `GB-ENG` | `GB-SCT` | `GB-WLS` · ZWJ seq → 1 code point where a CURATED equivalence holds (family → U+1F46A, couple w/ heart → U+1F491, kiss → U+1F48F; hand-written table, each row cited), else its first emoji. ⊥ guess beyond the table. a file granting a target keeps the base emoji (V6).
V65: `fix` = passes til 1 changes nothing (≤3 past the 1st), each V6-guarded on its own input; ⊥ settled → refused (V5) ∵ a rewrite can REVEAL a sequence the scan already passed (B14). ∀ row (rewrite & unmapped) @ ORIGINAL text, 1 byte-ordered list: a later pass's pos mapped back through each earlier pass; inside an earlier replacement → its start.

## §T TASKS

id|status|task|cites
T10|x|ARCHIVED to SPEC-ARCHIVE.md|V4,V5,V6
T26|x|ARCHIVED to SPEC-ARCHIVE.md|V26,`src/charset:V22`
T29|x|ARCHIVED to SPEC-ARCHIVE.md|V27
T30|x|ARCHIVED to SPEC-ARCHIVE.md|V28,V27
T33|x|ARCHIVED to SPEC-ARCHIVE.md|V62,V31,`src/charset:V22`,`src/lint:V34`
T53|x|ARCHIVED to SPEC-ARCHIVE.md|V51,V5,V6,`.:R11`
T59|x|ARCHIVED to SPEC-ARCHIVE.md|V60,V31

## §B BUGS

id|date|cause|fix
B14|2026-10-02|stray VS16 or skin tone in a ZWJ seq (`U+1F469 U+FE0F U+200D U+1F4BB`): V60 delete revealed an RGI seq the scan had passed ∴ 2nd run differed & `fix` refused the file (V5). via T33 fixtures|V65
B24|2026-10-02|V65 later pass reported @ intermediate text (earlier rewrites shifted it) & mixed into 1 list ∴ rows ⊥ resolve in the file on disk. via review|V65
B34|2026-10-02|curated ZWJ table had 3 of UTS #51 §2.6's groupings ∴ mixed-tone men holding hands → U+1F468 while same-tone → U+1F46C (V60); handshake → U+1FAF1|V62
