# SPEC

## §G GOAL

`hazard` lint group: which hazard lint a char fires, from compiled-in data; the joiner, tag & VS16 exemptions. ⊥ vocabulary & levels (`src/lint`), ⊥ code points (`src/charset`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/lint|lint names, groups, levels, hazard, pedantic
self|src/lint/hazard|hazard lints: classes, compiled-in detection, joiner, tag & VS16 exemptions
sib|src/lint/pedantic|pedantic lints: walks, claim order, Unicode crates facade

## §V INVARIANTS

V34: `hazard` set, generated from UCD 18.0.0 properties (inputs hashed, ⊥ vendored, `src/charset:V97`): ∀ `Default_Ignorable_Code_Point` (ZWSP, word joiner, soft hyphen, VS outside declared emoji sequences, Hangul fillers, invisible math ops, tag chars U+E0000–U+E007F) ∪ bidi controls (Bidi_Control: U+061C, U+200E–U+200F, U+202A–U+202E, U+2066–U+2069) ∪ C0/C1 controls ∖ `\t` `\n` `\r` ∪ BOM ∉ file start. ZWJ ∈ declared emoji sequence (`src/fix:V31`) exempt; ZWJ/ZWNJ via CLDR language preset REJECTED: `forbid` ⊥ lowered by later rule | flag (`src/lint:V36`) ∴ a preset-driven exemption contradicts it. hazard hit = forbid level ∀ rule (`src/lint:V36`) ∴ `any` grants every char & a hazard STILL fires — the exclusion lives in the LEVEL, ⊥ in the set model, ∵ `src/charset:V3` forbids subtraction & "everything minus hazard" is ⊥ expressible. ⊥ lowered, ⊥ even by naming the `hazard` group or one of its lints (`src/lint:V36`); joiners in a script that spells w/ them → V57. bidi controls ∉ builtin map, yet `fix` deletes them & every hazard that draws ⊥ text, granted or ⊥ (`src/fix:V104`); `control-character` ⊥ deleted w/o a map entry ∴ reported.
V49: hazard = 1 lint per class, first class holding the char wins: `bidi-control` (Bidi_Control) · `tag-character` · `stray-bom` · `control-character` (Cc ∖ `\t` `\n` `\r`) · `invisible` (rest of Default_Ignorable). char ∈ hazard ∧ ∉ set → 1 finding, the hazard; report names set `hazard`. classes read from COMPILED-IN data (`src/charset:V22`), ⊥ run's catalog ∴ `.ctrm-sets` | `--no-builtin-sets` ⊥ empty them. ZWJ & tag exemption (V34) = V63, landed w/ `src/fix:T33`: exact RGI match, ⊥ approximated.
V57: ZWNJ U+200C & ZWJ U+200D = SPELLING in fa & Devanagari ∴ a file whose rule names a COMPILED-IN preset granting a joiner & ⊥ other hazard (`persian`: ZWNJ; `hindi`: both) ⊥ fires on it. ⊥ level change (`src/lint:V36` holds): the char ⊥ hazard IN THAT FILE. presets read from builtin data ∴ `.ctrm-sets` redeclaring `persian` ⊥ widens it: excuse only where the run resolves the name EXACTLY as shipped (a name ⊥ provenance; redeclared | builtin-less → ⊥ excuse); `any` grants other hazards ∴ excuses ⊥. bidi, tag, control ⊥ excusable ∴ `persian` omits U+061C. tool output (guard) has ⊥ grant ∴ ⊥ excuse.
V63: 2A exemption (V34): ZWJ U+200D inside an RGI ZWJ sequence, tag chars inside an RGI tag sequence (the 3 subdivision flags) & VS16 U+FE0F inside an RGI presentation (`Basic_Emoji` + VS16) | keycap | ZWJ sequence (B15) ⊥ hazard. EXACT match against the vendored list (`src/fix:V62`), longest first ∴ a joiner | tag ⊥ in a listed sequence still fires, & ⊥ "joiner between 2 emoji" heuristic (a 2nd, private definition of a sequence). the exempted char then = `outside-set` (`emoji` withholds it, `src/fix:V31`) ∴ `fix` compresses the sequence. guard: same list, same exemption on tool output. list read from compiled-in data ∴ config ⊥ widens it (V49).

## §T TASKS

id|status|task|cites
T36|x|ARCHIVED to SPEC-ARCHIVE.md|V34,`src/lint:V36`,V49

## §B BUGS

id|date|cause|fix
B15|2026-10-02|VS16 U+FE0F = Default_Ignorable ∴ hazard everywhere; V34 exempts it inside declared emoji sequences but ⊥ list held `Basic_Emoji` presentation sequences ∴ `U+2764+U+FE0F` (red heart) fired forbid under `ascii+emoji`. via 2A smoke run|V63
B19|2026-10-02|V57 excuse matched the set NAME ∴ `.ctrm-sets` `hindi ascii U+200C U+200D` (or `--no-builtin-sets --set`) + `* hindi` excused ZWNJ. via release review|V57
