# SPEC

## §G GOAL

Judging: rules + charset + lint + scan + fix composed into findings. crate-internal (`src:V98`). ⊥ IO beyond the file read, ⊥ argv, ⊥ rendering.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/judge|findings from rules, sets, lints, scan & fix; config assembly
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- `Config` = 3 `Sources` (rules, sets, map) + root + `--strict`; FILLED by `src/cli` (argv, dotfiles), ASSEMBLED here: `validate` (`src/cli:V74`), `rules`, `catalog`, `listing`, `map`.
- `Checker` = one run's rules, catalog & hazards: `shared_law(path)` → set + levels; `effective(path)` → set + winning rule (`explain`); `findings(path, bytes)` (`guard`); `declared(family)` (`sets`).
- `Judge::fix(text, map)` = `src/fix` under the file's law: set + its hazards as `check` judges them (`src/fix:V104`) ∴ `fix` & `stats` ⊥ 2nd hazard verdict.
- `inspect(bytes, judge, levels)` → findings | `Unreadable`; `judged(text, …)` the same on text already decoded. pure, ⊥ fs.
- `files(root, paths)` = THE walk of `check`, `fix` & `stats`: select (`src/tokens`), read, decode ONCE (`src/scan:V8`), `shown_path` (V71).
- `unruled(bytes, hazards)` = hazards only, ⊥ rules (`guard`): text as is; not text → its lossy decode ∖ `control-character` (`src/cli/guard:V66`).

## §V INVARIANTS

V71: path matched (`src/rules`) & shown in LEXICAL normal form: `.` dropped, `..` folded ∴ `sub/../sub/c.md` ≡ `sub/c.md` ∀ anchored rule. symlink ⊥ resolved.
V99: ∀ verb & `guard` judge bytes through ONE `Checker` ∴ ⊥ two verdicts on the same bytes (`src/cli/guard:V35`, `src/cli:V80`). per char ≤ 1 finding: hazard > `outside-set` > pedantic (`src/lint:V55`); a hazard stops the scan even where the set grants it (`src/lint:V34`); `allow` ⊥ reported. a union = resolved once per sets + family. runner: `checker_test.rs` & children.

## §T TASKS

id|status|task|cites
T65|x|extract the judge from `src/cli` (`checker`, config assembly) ∴ `guard` & `explain` import `src/judge`, ⊥ their parent's privates|V99,`src:V39`
T66|x|ONE walk → read → decode (`files`) for `check`, `fix` & `stats`; guard's hazards-only path → `unruled`; `scan_bytes` returns its text ∴ ⊥ 2nd decode|V71,`src/scan:V8`

## §B BUGS

id|date|cause|fix
B38|2026-10-02|a named path w/ `..` (`sub/../sub/c.md`) ⊥ normalised ∴ missed anchored rules & was judged `ascii`. via release review|V71
