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

V48: tracked SYMLINK ⊥ in V9 fileset nor V43 expansion ∵ git stores link TEXT ⊥ target: reading through scans a dir (abort), or a file twice or outside repo. link NAMED on command line followed (V9).

V70: tracked path ⊥ on disk (deleted, deletion unstaged) ⊥ in V9 fileset nor V43 expansion ∵ ⊥ bytes to judge; reading it aborted the WHOLE run.

## §T TASKS

id|status|task|cites
T41|x|ARCHIVED to SPEC-ARCHIVE.md|V9,V10
T47|x|ARCHIVED to SPEC-ARCHIVE.md|V43,`src/cli:T44`

## §B BUGS

id|date|cause|fix
B4|2026-09-27|V9 fileset took `itok::walk::tracked` as-is, symlinks incl. ∴ tracked link to a dir → `Is a directory`, exit 2, WHOLE run aborted (`.:R7`)|V48
B25|2026-10-02|tracked file deleted from the tree stayed in V9 fileset ∴ read failed → exit 2, WHOLE run aborted, & bare `fix` had already written the files before it. via review|V70,`src/cli:V72`
