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
sib|src/judge|findings from rules, sets, lints, scan & fix; config assembly
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §C CONSTRAINTS

- `itok::` named here & nowhere else in crate (`src:V39`).

## §R RESEARCH

id|topic|finding|src
R7|wave 1 corpus|12 sibling Rust repos: public `itok` `microlith` `pklith` `sherd` `xenolith` + 7 private. `rekall` ⊥ scanned ∵ B4; rescanned after fix: 115 files, 42 non-ASCII, 1,187 outside, 0 tok saved. 1,484 tracked files, 561 non-ASCII, 17,271 chars outside `ascii`, 85% in `.md`. top: `⊥` 3660, `§` 3374, U+2014 2439, `→` 1717, `∴` 1703, `·` 1460|scan 2026-09-27, `ctrm` @`61b40d2`, private repos anonymous

## §V INVARIANTS

V9: default fileset = git-tracked (`itok::walk::tracked`); explicit paths reach untracked.
V10: token figure self-describes unit & method, per `itok`: `~` = bytes/4 estimate, `(o200k)` = `--bpe`. ⊥ claim measurement it cannot make.

V43: named DIRECTORY → expands to the tracked files under it (prefix over V9 fileset), ⊥ refused. 0 tracked file under it → error naming the dir, exit 2. `ctrm check src/` = what a reader types ∴ refusing that spelling teaches ⊥.

V48: tracked SYMLINK ⊥ in V9 fileset nor V43 expansion ∵ git stores link TEXT ⊥ target: reading through scans a dir (abort), or a file twice or outside repo. link NAMED on command line followed (V9).

V69: run root (`-C`) ⊥ a directory → error naming it, exit 2. bare run w/ root in ⊥ git work tree (⊥ `.git` @ root or above) → error naming it, exit 2 ∵ an empty V9 fileset reports a clean tree nobody read. named paths reach files w/o git (V9).

V70: tracked path ⊥ on disk (deleted, deletion unstaged) ⊥ in V9 fileset nor V43 expansion ∵ ⊥ bytes to judge; reading it aborted the WHOLE run.

V83: named paths folded LEXICALLY (`lexical`, owned here; `src/cli:V71` form) before dedup & before V43 prefix test ∴ `sub/../a.md a.md` = 1 file, `d/e/../e` expands as `d/e`.

V84: named LINK to a dir followed, as a named file link (V9): both sides resolved, expands to target's tracked files under target's paths ∴ `d dl` = 1 set. target outside root → error saying so, exit 2.

## §T TASKS

id|status|task|cites
T41|x|ARCHIVED to SPEC-ARCHIVE.md|V9,V10
T47|x|ARCHIVED to SPEC-ARCHIVE.md|V43,`src/cli:T44`

## §B BUGS

id|date|cause|fix
B4|2026-09-27|V9 fileset took `itok::walk::tracked` as-is, symlinks incl. ∴ tracked link to a dir → `Is a directory`, exit 2, WHOLE run aborted (R7)|V48
B25|2026-10-02|tracked file deleted from the tree stayed in V9 fileset ∴ read failed → exit 2, WHOLE run aborted, & bare `fix` had already written the files before it. via review|V70,`src/cli:V72`
B27|2026-10-02|`-C /nonexistent`, `-C <file>` & a bare run outside git → V9 fileset empty ∴ clean report, exit 0, about ⊥ file. via review|V69
B45|2026-10-03|dedup & V43 prefix test on raw `root.join(path)` ∴ `check sub/../a.md a.md` judged it twice (`stats`, `fix --check` too) & `check d/e/../e` → "holds no git-tracked file", exit 2. via release review|V83
B46|2026-10-03|V43 prefix test on the link's own name ∴ `ln -s d dl; check dl` → "holds no git-tracked file", exit 2, while a named file link was followed. via release review|V84
