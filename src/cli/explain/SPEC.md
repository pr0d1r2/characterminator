# SPEC

## §G GOAL

`explain` & `sets` verbs + `explain --as`: what is in force & why; effective config written back out. ⊥ resolution (`src/rules`), ⊥ dispatch (`src/cli`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter
self|src/cli/explain|`explain` & `sets` answers, `explain --as` forms: args, lines, agent prompt
sib|src/cli/guard|`guard` hook adapter: harness payload in, hook decision out, JSON reader

## §V INVARIANTS

V32: `explain --as args|lines|prompt` renders effective config (per path | whole repo): `args` = flags, `lines` = data-file lines, `prompt` = deterministic agent instruction (allowed chars, replacements, fidelity) for drafting compliant code up front. round trip: `--as args` output fed back ≡ same config, property-tested w/ `src/rules:V18`. ⊥ model (CPU only). `--as` writes text ∴ `--format` json | sarif → exit 2 (test `tests/explain_as.rs`).

## §T TASKS

id|status|task|cites
T34|x|ARCHIVED to SPEC-ARCHIVE.md|V32,`src/rules:V18`
