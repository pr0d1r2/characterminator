# SPEC

## §G GOAL

Arg dispatch, verbs, exit codes. parent of the `guard` hook adapter & the `explain` verbs. ⊥ a verb's logic.

## §F FEDERATION

dir|owns|⊥owns|tokens
guard|`guard` hook adapter: harness payload in, hook decision out, JSON reader|dispatch, hazard detection|-
explain|`explain` & `sets` answers, `explain --as` forms: args, lines, agent prompt|dispatch, rule resolution|-

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/render|human & json output, stable json contract

## §I INTERFACES

- cmd: `ctrm check [paths]` → violations, 1 per line, row shape `src/render:V94`. 0 clean / 1 violation / 2 usage.
- cmd: `ctrm fix [--check] [paths]` → rewrite disallowed chars via transliteration map. `--check` reports & writes ⊥, exit 1 on drift (`rustfmt` grammar). bare: exit 1 only on unmapped char left (`src/cli:V7`).
- cmd: `ctrm stats [--bpe] [paths]` → per file: chars outside set, bytes, tokens now vs after `fix`, via `itok`. report-only.
- cmd: `ctrm explain [<path>] [--as args|lines|prompt]` → effective set + winning rule + its config line; `--as` renders effective config as flags, data-file lines, or agent prompt (`src/cli/explain:V32`).
- cmd: `ctrm sets` → builtin sets & members.
- cmd: `ctrm guard` → hook adapter: harness hook JSON stdin → decision JSON stdout; fuse on hazard (`src/cli/guard:V35`).
- flag: `--format human|json` ∀ verb ∖ `guard` (argv ignored, `src/cli/guard:V93`); `sarif` `check` only (`src/render:V50`) · `-C <dir>`.
- flag (file twins, repeatable, `src/rules:V18`): `--rule <line>` · `--map <line>` · `--set <line>` · `--rules-file <f>` · `--map-file <f>` · `--sets-file <f>` · `--no-files` (skip discovered dotfiles) · `--no-builtin-map` · `--no-builtin-sets` · `--fidelity <family>` (`src/rules:V29`) · `--strict` (`src/lint:V36`) · `--pedantic` (`src/lint:V37`) · `--no-color` (∀ verb, no-op: output ⊥ ever coloured; accepted ∵ unknown flags exit 2).
- flag: unknown | another verb's (`check --bpe`) → exit 2 (∖ `guard`, `src/cli/guard:V93`) ∵ a typo'd flag silently ignored reads as one that worked. `--` ends flags.

## §V INVARIANTS

V7: only `check` & `fix --check` gate. bare `fix` rewrites only on explicit call; exit 1 only if an unmapped char is LEFT (`src/fix:V4`) or a not-UTF-8 file went unjudged (`src/scan:V8`), ⊥ for what it repaired. `stats`/`explain`/`sets` report-only, exit 0.
V47: stdout closed by reader (EPIPE, `ctrm check | head`) → stop writing, keep verdict, ⊥ panic. other write error → named, exit 2 ∵ lost output ⊥ silent. ∀ verb. LIMIT: a stdout CLOSED before start (`ctrm check >&-`) ⊥ detectable: the Rust runtime reopens fd 1 on `/dev/null` before `main` & `unsafe` is forbidden ∴ ≡ discarded output, exit by verdict.
V71: path matched (`src/rules`) & shown in LEXICAL normal form: `.` dropped, `..` folded ∴ `sub/../sub/c.md` ≡ `sub/c.md` ∀ anchored rule. symlink ⊥ resolved.
V72: bare `fix` = 2 phases: judge ∀ file, THEN write ∴ a refusal (`src/fix:V5`, `src/fix:V6`) or read error writes ⊥ file.
V73: path arity: `sets` 0, `explain` (any `--as`) ≤ 1, other verbs any. extra path → exit 2 naming it ∵ `explain a.md b.txt` answered for `a.md` only & `sets a.md` ignored it, each read as an answer about what was asked.
V74: ∀ verb ⊥ `guard`: whole config (rules, sets, map) parsed & validated before dispatch; any kind broken → exit 2, naming its origin ∵ a verb reading only its own kind reported on a config that ⊥ parsed (B29). ∀ rule: every set it names resolves, matched or ⊥ (B47). `--map` value = 1 line, as `--set` & `--rule` (`src/rules:V18`).
V80: `fix` & `fix --check` judge what they LEAVE via `check`'s `Checker` (levels, `--strict`, hazards), rows @ ORIGINAL pos (`src/fix:V65`) ∴ exit = drift (`--check`) ∨ `check`(output) ∨ not-UTF-8 skip.

## §T TASKS

id|status|task|cites
T8|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/tokens:V9`,`.:I.cmd`
T11|x|ARCHIVED to SPEC-ARCHIVE.md|`src/tokens:V10`
T12|x|ARCHIVED to SPEC-ARCHIVE.md|`src/rules:V2`,V7
T44|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/tokens:V43`
T50|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/fix:V4`,`src/rules:V45`
T55|x|ARCHIVED to SPEC-ARCHIVE.md|`src/rules:V18`,`src/rules:V19`,`src/lint:V36`

## §B BUGS

id|date|cause|fix
B3|2026-09-27|report via `println!` → PANIC on closed stdout ∴ `ctrm check \| head` crashed ∀ verb. via wave 1 (`src/tokens:R7`)|V47
B21|2026-10-02|`fix` & `stats` asked UTF-8 only, ⊥ `src/scan` binary test ∴ NUL-laden blob `check` skipped → rewritten by `fix`, counted by `stats`. via review|`src/scan:V8`
B22|2026-10-02|`check` exit = findings only, skips ignored ∴ invalid UTF-8 named & exit 0, ∀ format. now: not-UTF-8 skip → 1, binary skip → 0. via review|`src/scan:V8`
B23|2026-10-02|`fix` counted unmapped chars, rendered ⊥ ∴ exit 1 w/ ⊥ output. now per-path hits → `check` rows + json `unmapped`. via review|`src/fix:V4`
B26|2026-10-02|`stats` dropped a not-UTF-8 file w/o a word ∴ a total short of a file read as complete. now binary & not-UTF-8 named as skipped (json `skipped`), exit 0 (V7). via review|`src/scan:V8`
B29|2026-10-02|map parsed only by `fix` & `stats` ∴ `check --map 'U+ZZZZ x y z'` & a 2-line `--map` ran as if the config were valid. via release review|V74
B38|2026-10-02|a named path w/ `..` (`sub/../sub/c.md`) ⊥ normalised ∴ missed anchored rules & was judged `ascii`. via release review|V71
B39|2026-10-02|`sets a.md` & `explain a.md b.txt` silently ignored the extra path ∴ the run read as if it had judged it. via release review|V73
B41|2026-10-03|B22 fixed `check` only: `fix` & `fix --check` never read skips ∴ a not-UTF-8 file named & exit 0 while `check` exits 1. via release review|V7
B42|2026-10-03|`fix` judged leftovers as `outside-set` @ `deny`, ⊥ hazard ∴ `fix --check` ≠ `check` under `=allow`/`=warn` & `* any` + U+202E; `ascii` U+202E labelled `outside-set`. via release review|V80
B47|2026-10-03|rule sets resolved only when a file matched ∴ `check --rule '*.txt asci' --rule '* any'` exit 0, & a match's refusal ⊥ named `argv[n]`. via release review|V74
