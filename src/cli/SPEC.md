# SPEC

## §G GOAL

Arg dispatch, verbs, exit codes & the `guard` hook adapter. ⊥ a verb's logic.

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

## §V INVARIANTS

V7: only `check` & `fix --check` gate. bare `fix` rewrites only on explicit call. `stats`/`explain`/`sets` report-only, exit 0.
V32: `explain --as args|lines|prompt` renders effective config (per path | whole repo): `args` = flags, `lines` = data-file lines, `prompt` = deterministic agent instruction (allowed chars, replacements, fidelity) for drafting compliant code up front. round trip: `--as args` output fed back ≡ same config, property-tested w/ `src/rules:V18`. ⊥ model (CPU only).
V35: `guard` = hook adapter (itok `guard` shape): harness hook JSON stdin → decision JSON stdout; signal in JSON ⊥ exit code. before file read: hazard (`src/lint:V34`) → block, naming path, line, code point. after tool output (web fetch, web search, shell): hazard → blocking decision w/ reason = content tainted. ⊥ daemon, ⊥ silent strip. non-hazard violation → PASS + note naming lint & count ∴ hook stays usable; only hazard blocks. a hook that blocks on every stray char is one people switch off, & a switched-off hook gates ⊥.

## §T TASKS

id|status|task|cites
T8|x|`check` verb + fileset via `itok::walk::tracked`|V7,`src/tokens:V9`,`.:I.cmd`
T11|.|`stats` verb w/ `itok` counts now vs after fix|`src/tokens:V10`
T12|.|`explain` & `sets` verbs|`src/rules:V2`,V7
T34|.|`explain --as` renderers (args, lines, prompt) + round-trip property test|V32,`src/rules:V18`
T37|.|`guard` hook adapter: pre-read block, post-output taint decision, harness JSON fixtures|V35,`src/lint:V34`
T44|.|`[dir]` arg: `src/tokens` REFUSES a directory ∴ expand it \| keep refusing & name the working spelling ?|V7
