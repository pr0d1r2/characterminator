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

## §V INVARIANTS

V4: `fix` replaces only via declared transliteration map. char w/o mapping → kept & reported, exit 1. ⊥ silent drop.
V5: `fix` idempotent: `fix(fix(x)) == fix(x)`, property-tested.
V6: `fix` touches ⊥ allowed char: bytes outside violations ! identical pre/post, asserted before write.
V26: typography = builtin map targets, ⊥ default grant (`src/charset:R4`: 45.1% of non-ASCII files need no grant after map): `—`→`--` · `–` `−`→`-` · curly quotes → straight · `…`→`...` · `«»`→`"` · NBSP → space · ZWSP & BOM → delete. `typography` set ? for prose that keeps them.
V27: character families = open tree, declared by map line `family <name> <parent>`; parent chain ! end @ `ascii` (intrinsic root); cycle → config error, exit 2. builtin: `ascii` ← `text` ← `emoji`. new family (e.g. `nerd`) = 1 line + members in classes.
V28: map line `= <class> <family>:<member>[,<member>...] ...` declares equivalence class; members labelled by family, first per family preferred. disallowed member → preferred member of rule's fidelity family if allowed, else along its fallback path (V27) → `ascii`; none allowed → unmapped (V4). `--map` twin takes this form (`src/rules:V18`). multi-codepoint members per V31.
V31: `emoji` preset = single code points only: ⊥ VS15/VS16, ⊥ skin tone U+1F3FB–U+1F3FF, ⊥ ZWJ U+200D, ⊥ keycap U+20E3, ⊥ tag chars ∴ ∀ sequence has a disallowed code point → builtin sequence map (vendored Unicode emoji data) compresses: selector & skin tone → delete (`👍🏽`→`👍`); ZWJ sequence → single code point equivalent if one exists (`👨‍👩‍👧`→`👪`), else first emoji; keycap & flag → ASCII (`1`, `PL`) ?. map `from` & class members ? be sequences; scan = longest declared sequence first. `emoji-seq` preset ? keeps sequences.

## §T TASKS

id|status|task|cites
T10|.|transliteration map + `fix` & `fix --check`; property tests: idempotency, untouched bytes|V4,V5,V6
T26|.|builtin map: typography defaults|V26,`src/charset:V22`
T29|.|family lines: parse, tree validation, fallback path|V27
T30|.|class lines: parse, family-labelled members, resolution along fallback path|V28,V27
T33|.|vendor Unicode emoji data; sequence map generator; longest-match scan|V31,`src/charset:V22`
