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
- `inspect(bytes, judge, levels)` → findings | `Unreadable`: pure, ⊥ fs.

## §V INVARIANTS

V99: ∀ verb & `guard` judge bytes through ONE `Checker` ∴ ⊥ two verdicts on the same bytes (`src/cli/guard:V35`, `src/cli:V80`). per char ≤ 1 finding: hazard > `outside-set` > pedantic (`src/lint:V55`); a hazard stops the scan even where the set grants it (`src/lint:V34`); `allow` ⊥ reported. a union = resolved once per sets + family. runner: `checker_test.rs` & children.

## §T TASKS

id|status|task|cites
T65|x|extract the judge from `src/cli` (`checker`, config assembly) ∴ `guard` & `explain` import `src/judge`, ⊥ their parent's privates|V99,`src:V39`
