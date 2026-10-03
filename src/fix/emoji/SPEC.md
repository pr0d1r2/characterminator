# SPEC

## §G GOAL

Emoji compression: modifier deletes, the generated sequence map, flag pairing, multi-pass settle. ⊥ map grammar, ⊥ the rewrite walk (`src/fix`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
self|src/fix/emoji|emoji compression: modifier deletes, sequence map, flag pairing, multi-pass settle
sib|src/fix/words|opt-in `words` map: named maps for `use`, word spacing, token measurements

## §V INVARIANTS

V31: `emoji` preset = single code points only: ⊥ VS15/VS16, ⊥ skin tone U+1F3FB–U+1F3FF, ⊥ ZWJ U+200D, ⊥ keycap U+20E3, ⊥ tag chars ∴ ∀ sequence has a disallowed code point → builtin sequence map (vendored Unicode emoji data) compresses: selector & skin tone → delete (`👍🏽`→`👍`); ZWJ sequence → single code point ⟺ V62's curated table names one (`U+1F468+U+200D+U+1F469+U+200D+U+1F467`→`👪`), else first emoji; keycap → its ASCII digit | `#` | `*`; flag → its 2-letter region code (`PL`). map `from` & class members ? be sequences (notation: `src/fix:I`); scan = longest declared sequence first. `emoji-seq` preset REJECTED: a set = CODE POINTS & a sequence ⊥ one ∴ ⊥ expressible as a set; reopens only if the set model itself gains sequences.
V60: builtin map DELETES VS15 U+FE0E, VS16 U+FE0F & skin tones U+1F3FB–U+1F3FF ∴ modifier sequence → its base (`👍🏽`→`👍`) w/ ⊥ sequence scan: each is 1 code point the `emoji` preset withholds (V31). = the cheap half of V31; ZWJ sequences, keycaps, flags → T33. a file granting them keeps them (`src/fix:V6`), bar a hazard (`src/fix:V104`).
V62: 2A = T33 data: pinned Unicode emoji 18.0 `emoji-sequences.txt` + `emoji-zwj-sequences.txt`, vendored in `src/charset` (`src/charset:V22`; sha256 in header, as `src/charset/locale:V59`) ∴ 1 list feeds this map & the hazard exemption (`src/lint/hazard:V63`) → generated `emoji-seq.ctrm-map`, 1 line ∀ RGI sequence V60 ⊥ already covers, `from` = the sequence (`src/fix:I` notation), longest match = existing scan. targets: keycap `<k> U+FE0F U+20E3` → `<k>` (digit | `#` | `*`) · flag (2 regional indicators) → its region code (`PL`) · tag flag (U+1F3F4 + tags) → `GB-ENG` | `GB-SCT` | `GB-WLS` · ZWJ seq → 1 code point where a CURATED equivalence holds (family → U+1F46A, couple w/ heart → U+1F491, kiss → U+1F48F; hand-written table, each row cited), else its first emoji. ⊥ guess beyond the table. a file granting a target keeps the base emoji (`src/fix:V6`).
V65: `fix` = passes til 1 changes nothing (≤3 past the 1st), each `src/fix:V6`-guarded on its own input; ⊥ settled → refused (`src/fix:V5`) ∵ a rewrite can REVEAL a sequence the scan already passed (B14). ∀ row (rewrite & unmapped) @ ORIGINAL text, 1 byte-ordered list: a later pass's pos mapped back through each earlier pass; inside an earlier replacement → its start.
V76: flags (V62). regional indicators U+1F1E6–U+1F1FF pair from the START of each run (UAX #29): a pair ⊥ flag → both kept & reported, ⊥ its 2nd half opens a flag w/ the next (`XU`+`S` ⊥ `X`+`US`). flag & tag-flag targets = `src/fix/words:V51` `word` entries ∴ code ⊥ fuses w/ a `\w` neighbour or the next code (`A`+flag+`B`→`A PL B`, 2 flags → `PL DE`). tests: `src/fix:V5` & `src/fix:V6` ∀ fixture under `ascii` & `emoji`.

## §T TASKS

id|status|task|cites
T33|x|ARCHIVED to SPEC-ARCHIVE.md|V62,V31,`src/charset:V22`,`src/lint/hazard:V34`
T59|x|ARCHIVED to SPEC-ARCHIVE.md|V60,V31

## §B BUGS

id|date|cause|fix
B14|2026-10-02|stray VS16 or skin tone in a ZWJ seq (`U+1F469 U+FE0F U+200D U+1F4BB`): V60 delete revealed an RGI seq the scan had passed ∴ 2nd run differed & `fix` refused the file (`src/fix:V5`). via T33 fixtures|V65
B24|2026-10-02|V65 later pass reported @ intermediate text (earlier rewrites shifted it) & mixed into 1 list ∴ rows ⊥ resolve in the file on disk. via review|V65
B34|2026-10-02|curated ZWJ table had 3 of UTS #51 §2.6's groupings ∴ mixed-tone men holding hands → U+1F468 while same-tone → U+1F46C (V60); handshake → U+1FAF1|V62
B35|2026-10-02|scan matched a flag at ANY regional indicator ∴ `X`,`US` adjacent → `X` + `US` (shown: `XU` + lone `S`); plain region code fused w/ neighbours (`PLDE`, `APLB`)|V76
