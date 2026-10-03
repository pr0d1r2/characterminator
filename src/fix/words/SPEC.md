# SPEC

## §G GOAL

The opt-in `words` map: notation → the words it abbreviates, a word kept off its neighbours, & the measurements that justify it. ⊥ map grammar, ⊥ the rewrite walk (`src/fix`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
self|src/fix/words|opt-in `words` map: named maps for `use`, word spacing, token measurements
sib|src/fix/emoji|emoji compression: modifier deletes, sequence map, flag pairing, multi-pass settle

## §R RESEARCH

id|topic|finding|src
R8|builtin map saving|`fix` over whole corpus: 3,589,988 → 3,589,906 tok (o200k) = 82 saved, 0.002%; 1 private repo +1. per char in prose: U+2014 U+2013 U+2026 U+2212 U+201C U+201D → 0 saved; U+2019 1; NBSP 1; ZWSP 2 ∴ typography rewrite buys ⊥ tokens, only apostrophe & invisibles do|`src/tokens:R7` scan + 200-line probe per char
R9|caveman cost|o200k per use: `⊥` 3 tok vs `not` 1; `∴` `∀` `∵` 2 vs `so` `all` `because` 1; `→` `§` `·` 1 = parity. 148 `SPEC*.md` (corpus + this repo): `⊥∴∀∵` → words = 391,852 → 381,007 tok (-10,845, -2.8%) ≈ 130× whole map|`src/tokens:R7` scan. o200k only; Claude tokenizer ⊥ measured
R11|`words` map saving (V51)|112 caveman `SPEC*.md` (this repo + public `itok` `microlith` `pklith` `sherd` `xenolith`), `*.md ascii`: 266,472 → 259,513 tok (-6,959, -2.6%). saved/uses: `⊥` 4817/2447 · `∴` 1366/1366 · `∵` 359/359 · `∀` 332/354 · `∈` 36/36 · `≠` 28/28 · `∃` 21/21 · `→` 0/1167 · `⇒` 0/75 ∴ ⊥ entry costs MORE. this repo's 10 `SPEC.md`: 10,901 → 10,617 (-284, -2.6%) vs builtin map alone -20|`ctrm stats --bpe`: builtin vs builtin + 1 `word` line, 2026-10-01. o200k only; Claude tokenizer ⊥ measured

## §V INVARIANTS

V51: `words` map OPT-IN, ⊥ default (`src/fix:V26`): map line `use <name>` reads it AT that line (`src/rules:V19`), origin = that line; unknown → exit 2. `⊥`→not `∴`→so `∵`→because `∀`→all `∈`→in `∃`→exists `≠`→`!=` `→`→`->` `⇒`→`=>`, each ⊥ MORE o200k tok (R11). `word <from> <to>` = word entry: `\w` edge meets `\w` → 1 space (`⊥owns`→`not owns`), across a delete too; space = PART of replacement ∴ inside span for `src/fix:V6`; `src/fix:V5` holds. plain entry ⊥ spaced. `--words` flag REJECTED: 2nd switch for 1 map line, ⊥ positional.

## §T TASKS

id|status|task|cites
T53|x|ARCHIVED to SPEC-ARCHIVE.md|V51,`src/fix:V5`,`src/fix:V6`,R11
