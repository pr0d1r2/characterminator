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
V120: global help: ∀ verb & ∀ flag w/ a 1-line meaning, exit codes (0 clean, 1 violation, 2 usage | config; guard ⊥ 2), docs URL. `<verb> --help`: purpose, the flags that verb reads w/ meanings, exit codes, 1 example. ∀ flag of the table named in the global help (test `args.rs`). lines ≤ 80 cols ∵ scannable in a terminal. runner `help.rs`, `tests/surface.rs`.
V121: user-facing message ⊥ spec id (`(V51)`) ∵ its reader has ⊥ spec; a self-check refusal says it is a ctrm bug instead. a config error names a flag as typed (`--rule '* asci'`), ⊥ `argv[3]` (`explain` keeps `argv[n]`, `src/rules:V20`). map error = `<file>:<line>: <what> -- <expected forms>`, as `.ctrm` & `.ctrm-sets`. a closed choice refused lists the choices (`--format`); an unknown set → did-you-mean over the run's sets (distance ≤ 2, `src/judge` `nearest`). missing named path → root-relative, ⊥ `(os error N)`. runner `error.rs`, `dispatch_test.rs`, `config_test.rs`, `walk.rs`.

## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
B70|2026-10-03|`ctrm chek` dumped the usage w/o an error line; `ctrm -C .. check` refused; `ctrm --help --bogus` exit 0 while `check --help --bogus` exit 2; `guard --help` read stdin. via DX review|V119
