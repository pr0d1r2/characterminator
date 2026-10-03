# SPEC

## §G GOAL

Rewrite a disallowed char SAFELY: map, families, equivalence classes, compression. parent of the `emoji` compression & the opt-in `words` map.

## §F FEDERATION

dir|owns|⊥owns|tokens
emoji|emoji compression: modifier deletes, sequence map, flag pairing, multi-pass settle|map grammar, the rewrite walk|-
words|opt-in `words` map: named maps for `use`, word spacing, token measurements|map grammar, the rewrite walk|-

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
sib|src/judge|findings from rules, sets, lints, scan & fix; config assembly
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- file: `.ctrm-map` — transliteration, `<from> <to>`: `from` = literal char | `U+XXXX`; `to` = replacement, empty = explicit delete. `to` ∉ target file's set → char counts unmapped (V4). + `family <name> <parent>` (V27). + `= <class> <family>:<member>[,<member>...] ...` (V28). sequence = code points joined by `+` (`U+1F44D+U+1F3FD`) | written literally (the chars of `U+2714+U+FE0F`), ∀ `from` & ∀ member. + `word <from> <to>` (`src/fix/words:V51`). + `use <name>` (`src/fix/words:V51`).

## §R RESEARCH

id|topic|finding|src
R18|`fix --check` peak memory|5.5 M chars drawn from `a` `b` space U+00E9 U+2014 U+4E2D `x` LF (8.9 MB): 687 K rewrites + 1.37 M left. human 915 → 385 MB (`check` 266 ∴ 1.45×), json 1103 → 579; output byte-identical. cost was copies: path & set per row, each rewrite & finding cloned for render, each replacement cloned pass → report, engine `unmapped` held while leftovers judged. now path & set once per file (`check`'s batches), rows moved or borrowed, hits placed in place|`/usr/bin/time -l`, release, macOS arm64, 2026-10-03

## §V INVARIANTS

V4: `fix` replaces only via declared transliteration map. char w/o mapping → kept & reported as `check` row (json `unmapped`), exit 1. ⊥ silent drop. target judged AFTER its whole chain (V5): ∃ char ∉ set → match refused ∴ source kept & reported, ⊥ written (B40).
V5: `fix` idempotent: `fix(fix(x)) == fix(x)`, property-tested.
V6: `fix` touches ⊥ allowed char that is ⊥ a hazard in that file (V104): bytes outside violations & hazards ! identical pre/post, asserted before write.
V26: typography = builtin map targets, ⊥ default grant (`src/charset:R4`: 45.1% of non-ASCII files need no grant after map): `—`→`--` · `–` `−`→`-` · curly quotes → straight · `…`→`...` · `«»`→`"` · NBSP → space · ZWSP & BOM → delete. `typography` set SHIPS ∴ prose that wants real typography grants it & `fix` leaves those chars (V6). per-locale `typography-<code>` from CLDR punctuation (`src/charset/locale:R6`): CLDR now vendored (`src/charset/locale:T60`) ∴ open as T61.
V27: character families = open tree, declared by map line `family <name> <parent>`; parent chain ! end @ `ascii` (intrinsic root); cycle → config error, exit 2. builtin: `ascii` ← `text` ← `emoji`. new family (e.g. `nerd`) = 1 line + members in classes.
V28: map line `= <class> <family>:<member>[,<member>...] ...` declares equivalence class; members labelled by family, first per family preferred. disallowed member → preferred member of rule's fidelity family if allowed, else along its fallback path (V27) → `ascii`; none allowed → unmapped (V4). `--map` twin takes this form (`src/rules:V18`). multi-codepoint members per `src/fix/emoji:V31`.
V104: a hazard IN THAT FILE (= `check`'s verdict: `src/lint/hazard:V57`, `src/lint/hazard:V63`, byte-0 BOM; `src/judge` `Judge::fix` per pass (`src/fix/emoji:V65`)) → rewritten even where granted: map entry first, else DELETED if its class draws ⊥ text; `control-character` ? carry meaning ∴ w/o entry kept & reported. REJECTED: delete only map-covered (bidi ∉ map: B59); delete controls (guesses meaning). `stats` after = same. runners `apply_hazards_test.rs`, `src/cli/fix_hazard_test.rs`.

## §T TASKS

id|status|task|cites
T10|x|ARCHIVED to SPEC-ARCHIVE.md|V4,V5,V6
T26|x|ARCHIVED to SPEC-ARCHIVE.md|V26,`src/charset:V22`
T29|x|ARCHIVED to SPEC-ARCHIVE.md|V27
T30|x|ARCHIVED to SPEC-ARCHIVE.md|V28,V27
T61|.|per-locale `typography-<code>`: CLDR `punctuation` exemplars (locale quotes, pl `„”`) → 1 set ∀ locale, same generator & pinned tag as `src/charset/locale:V61`; granted ⇒ V26 map leaves them|V26,`src/charset/locale:R6`,`src/charset/locale:V61`

## §B BUGS

id|date|cause|fix
B40|2026-10-03|replacement re-run through the map (V5), inner run's unmapped dropped & result ⊥ judged ∴ `--map 'U+2261 U+2295'` wrote U+2295 into an `ascii` file, reported ⊥, exit 0; ZWJ seq → 1 emoji under `ascii` alike. via release review|V4
B59|2026-10-03|`* any` + U+202E: V6 shielded a granted hazard from `fix` while `check` fired forbid ∴ only a hand removed it. via user|V104
