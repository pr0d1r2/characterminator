# SPEC

## §G GOAL

`explain` & `sets` verbs + `explain --as`: what is in force & why; effective config written back out. ⊥ resolution (`src/rules`), ⊥ dispatch (`src/cli`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter
self|src/cli/explain|`explain` & `sets` answers, `explain --as` forms: args, lines, agent prompt
sib|src/cli/guard|`guard` hook adapter: harness payload in, hook decision out, JSON reader

## §V INVARIANTS

V32: `explain --as args|lines|prompt` renders effective config (per path | whole repo): `args` = flags, `lines` = data-file lines, `prompt` = deterministic agent instruction (allowed chars, replacements, fidelity) for drafting compliant code up front. round trip: `--as args` output fed back ≡ same config, property-tested w/ `src/rules:V18`. ⊥ model (CPU only). `--as` writes text ∴ `--format` json | sarif → exit 2 (test `tests/explain_as.rs`).
V127: `sets` lists curated first: builtin presets w/ a `#: <name> <text>` line in `src/charset/sets.ctrm-sets` (data beside the set, ⊥ code), file order, `ascii` 1st; then declared (`.ctrm-sets`, `--set`); locales ONLY w/ `--locales`, else 1 line counting them. words = set NAMES, listed in order named; unknown → exit 2. `--containing <c>` (`U+XXXX` | the char, 1 code point) → sets holding c, same order, locales incl. json mirrors all of it (`src/render:V95`). runners `listing_test.rs`, `src/charset` `every_builtin_preset_has_exactly_one_description`.
V129: `--as prompt` speaks to a model: a rule = pattern + file line, ⊥ `argv[n]`. replacements = only what a writer types (dashes, quotes incl. low-9, ellipsis, NBSPs, apostrophes) + ∀ user-declared map line, sorted by code point, each once. hazards ONCE: 1 line of classes from the `#:` descriptions (V127), ⊥ ranges. set ≤ 64 members → 1 line of `U+XXXX "g"` pairs, else ranges; glyphs = OUTPUT a model reads, ⊥ source `.:V13` governs ∴ the rest stays ASCII. ⊥ contradiction: set grants U+200D (`any`) → ⊥ sequence sentence, an RGI joiner said fine. `--pedantic` → 1 line naming the enabled pedantic lints. runner `prompt_test.rs`.

## §T TASKS

id|status|task|cites
T34|x|ARCHIVED to SPEC-ARCHIVE.md|V32,`src/rules:V18`

## §B BUGS

id|date|cause|fix
B74|2026-10-03|`--as prompt` named rules by `argv[n]`, listed ∀ map source (skin tones, ZWSP) & every hazard range twice, Polish letters as code points only; under `any` a sequence sentence beside "never write U+200D". via DX review|V129
