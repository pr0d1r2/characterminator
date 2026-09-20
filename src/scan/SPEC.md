# SPEC

## §G GOAL

Read text & locate chars: positions, UTF-8 validity, binary skip.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §V INVARIANTS

V8: invalid UTF-8 → error naming path & byte offset, exit 1; ⊥ lossy decode. binary file (NUL byte ANYWHERE in it) → skipped & named in report, ⊥ silent. `Unreadable` carries the REASON, ⊥ the path ∴ pairing is the reporter's (`src/render` `Skipped{path,reason}`).
V12: violation position = 1-based line + col (chars) + byte offset + `U+XXXX`. output sorted by path, then offset.

## §T TASKS

id|status|task|cites
T7|x|scan core over `&str`: positions, UTF-8 errors, binary skip|V8,V12
