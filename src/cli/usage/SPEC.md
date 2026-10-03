# SPEC

## §G GOAL

What a reader of `ctrm` sees when asking for help or getting the command wrong: global & per-verb help, the verb a command line names, message style. ⊥ dispatch & exit mapping (`src/cli`), ⊥ a verb's logic.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter
self|src/cli/usage|help text, verb detection, did-you-mean, message style
sib|src/cli/guard|`guard` hook adapter: harness payload in, hook decision out, JSON reader
sib|src/cli/explain|`explain` & `sets` answers, `explain --as` forms: args, lines, agent prompt
sib|src/cli/init|`init` verb: survey tracked files by type, greedy preset cover, draft `.ctrm`

## §V INVARIANTS

V119: verb = 1st argv word that is ⊥ a flag & ⊥ a flag's value ∴ flags before it work (`ctrm -C .. check`), origins stay process-argv positions. unknown verb → stderr `ctrm: unknown verb '<w>'` + ` (did you mean '<v>'?)` at edit distance ≤ 2, then 1 line pointing at `ctrm --help`, exit 2, ⊥ the whole usage ∵ a usage dump hides the one line that says what was wrong. flags & ⊥ verb → named, exit 2. `--help` & `--version` = flag-table entries ∴ `ctrm --help --bogus` exit 2, as `check --help --bogus` (`src/cli:V101`). `ctrm guard --help` | `-h` as the ONLY word → guard help, exit 0 (`src/cli/guard:V93`). runner `dispatch_test.rs`, `args.rs`, `tests/surface.rs`.

## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
B70|2026-10-03|`ctrm chek` dumped the usage w/o an error line; `ctrm -C .. check` refused; `ctrm --help --bogus` exit 0 while `check --help --bogus` exit 2; `guard --help` read stdin. via DX review|V119
