# SPEC

## §G GOAL

How loud a finding is: lint names, groups, levels, registry, resolution, findings & 1 claim per char. parent of the `hazard` & `pedantic` groups.

## §F FEDERATION

dir|owns|⊥owns|tokens
hazard|hazard lints: classes, compiled-in detection, joiner, tag & VS16 exemptions|code points, levels|-
pedantic|pedantic lints: walks, claim order, Unicode crates facade|levels, registry|-

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/judge|findings from rules, sets, lints, scan & fix; config assembly
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §V INVARIANTS

V33: severity ∈ `error` (default, exit 1) | `warn` (reported, exit 0); per rule suffix `!<severity>` (`docs/** caveman !warn`); last matching rule naming one wins; twin via `--rule` (`src/rules:V18`). `--strict` → warn counts as error (clippy `-D warnings` shape). json violation ! carry `severity`. [superseded by V36; `severity` key ⊥ shipped: json carries `lint` & `level`, `src/render:V95`]
V36: ∀ check = named lint in a group; levels rustc/clippy shape `allow` | `warn` | `deny` | `forbid` (`forbid` ⊥ lowered by later rule | flag). groups: `hazard` (forbid, `src/lint/hazard:V34`) · `charset` (deny, `src/rules:V1`, `src/rules:V2`, `src/charset:V3`) · `pedantic` (allow). rule suffix `!<level>` → `charset`; `!<lint|group>=<level>` → named one. `--strict` → warn ⇒ deny. deny | forbid hit → exit 1. json ! carry lint name & level.

## §T TASKS

id|status|task|cites
T35|x|ARCHIVED to SPEC-ARCHIVE.md|V33,`src/render:V11`
T38|x|ARCHIVED to SPEC-ARCHIVE.md|V36
