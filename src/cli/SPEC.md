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
V47: stdout closed by reader (EPIPE, `ctrm check | head`) → stop writing, keep verdict, ⊥ panic. other write error → named, exit 2 ∵ lost output ⊥ silent. ∀ verb.
V53: `guard` exit ⊥ verdict: 0 = decided (decision JSON | silence = pass). adapter failure (stdin ⊥ JSON, ⊥ `hook_event_name`, lost write) → named on stderr, exit 1 (overrides V47's 2). exit 2 ⊥ EVER ∵ Claude Code reads 2 as block ∴ a broken adapter would brick the session; 1 = non-blocking error. hazards ⊥ depend on config: `.ctrm` unusable → hazards still judged (`src/lint:V49`) + note. tool output judged for hazard ONLY ∵ ⊥ path for `.ctrm` & `ascii` default ⇒ note on every non-ASCII page. reason quotes data ∴ its non-ASCII → `<U+XXXX>` ∵ a reason ⊥ smuggles what it warns of.

## §T TASKS

id|status|task|cites
T8|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/tokens:V9`,`.:I.cmd`
T11|x|ARCHIVED to SPEC-ARCHIVE.md|`src/tokens:V10`
T12|x|ARCHIVED to SPEC-ARCHIVE.md|`src/rules:V2`,V7
T34|x|`explain --as` renderers (args, lines, prompt) + round-trip property test|V32,`src/rules:V18`
T37|x|`guard` hook adapter: pre-read block, post-output taint decision, harness JSON fixtures|V35,V53,`src/lint:V34`
T44|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/tokens:V43`
T50|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/fix:V4`,`src/rules:V45`
T55|x|wire the flag twins (`.:I.flag`) into every verb: `--rule` `--map` `--set` `--*-file` `--no-files` `--no-builtin-map` `--no-builtin-sets` `--strict` `--pedantic`; same parser & origin as the dotfiles|`src/rules:V18`,`src/rules:V19`,`src/lint:V36`

## §B BUGS

id|date|cause|fix
B3|2026-09-27|report via `println!` → PANIC on closed stdout ∴ `ctrm check \| head` crashed ∀ verb. via wave 1 (`.:R7`)|V47
