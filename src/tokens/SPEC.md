# SPEC

## §G GOAL

Count tokens & list files. Sole call site for `itok`.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §C CONSTRAINTS

- `itok::` named here & nowhere else in crate (`src:V39`).

## §V INVARIANTS

V9: default fileset = git-tracked (`itok::walk::tracked`); explicit paths reach untracked.
V10: token figure self-describes unit & method, per `itok`: `~` = bytes/4 estimate, `(o200k)` = `--bpe`. ⊥ claim measurement it cannot make.

V43: named DIRECTORY → expands to the tracked files under it (prefix over V9 fileset), ⊥ refused. 0 tracked file under it → error naming the dir, exit 2. `ctrm check src/` = what a reader types ∴ refusing that spelling teaches ⊥.

## §T TASKS

id|status|task|cites
T41|x|ARCHIVED to SPEC-ARCHIVE.md|V9,V10
T47|.|expand a named directory to its tracked files; empty dir = error naming it|V43,`src/cli:T44`
