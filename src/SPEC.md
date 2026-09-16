# SPEC

## §G GOAL

`src/` — hub. code nodes of `characterminator`. ⊥ own logic itself.

## §F FEDERATION

dir|owns|⊥owns|tokens
charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data|rule resolution, rewriting|-
rules|config files & flags, precedence, origin, rule resolution, fidelity choice|set contents, rewriting|-
scan|read text: positions, UTF-8 validity, binary skip|which chars are allowed, rewriting|-
fix|rewriting: map, families, equivalence classes, typography, emoji compression|which chars are allowed, reporting|-
lint|lint names, groups, levels, hazard, pedantic|char sets, rewriting|-
tokens|`itok` facade: counts w/ method label, git-tracked fileset|char sets, rewriting|-
render|human & json output, stable json contract|what is reported|-
cli|arg dispatch, verbs, exit codes, `guard` hook adapter|every verb's logic|-

## §N NAV

rel|path|lens
up|.|-
self|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI

## §C CONSTRAINTS

- module = dir + `mod.rs`. `mod.rs` composes, ⊥ implements.
- ∀ external dep ! ONE facade node: `itok` → `src/tokens`.

## §V INVARIANTS

V38: `main.rs` & `lib.rs` dispatch & re-export only, ⊥ logic.
V39: node ⊥ dep sibling node except through its public surface ∴ a rule moves w/ its node.
