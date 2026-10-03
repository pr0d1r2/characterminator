# SPEC

## §G GOAL

`init` verb: tracked files → a draft `.ctrm`, commented, deterministic. ⊥ dispatch (`src/cli`), ⊥ set contents (`src/charset`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter
self|src/cli/init|`init` verb: survey tracked files by type, greedy preset cover, draft `.ctrm`
sib|src/cli/guard|`guard` hook adapter: harness payload in, hook decision out, JSON reader
sib|src/cli/explain|`explain` & `sets` answers, `explain --as` forms: args, lines, agent prompt
sib|src/cli/usage|help text, verb detection, did-you-mean, message style

## §V INVARIANTS

V128: `init [--print]` reads the git-tracked fileset (`src/tokens:V9`), 1 group per file type (`*.<ext>`, else the file name). per group: hazard (`check`'s verdict, RGI joiners exempt) → named "fix these", ⊥ EVER granted · builtin map → ASCII (`src/fix:V26`) → "`ctrm fix` rewrites", ⊥ granted · rest = needed → greedy cover over `caveman typography math legal marks box emoji cr` (max gain, tie → smaller set, then that order), then 1 locale holding the rest (`en-aux` first, else base code before variant, smaller, name), else `any` + comment naming what nothing covers. ∀ set line carries a comment of what it covers; pure-ASCII types listed, ⊥ line. same files → same bytes. `.ctrm` exists → exit 2 naming `--print`, ⊥ overwrite; `--print` → stdout, writes ⊥. runner `init_test.rs`.

## §T TASKS

id|status|task|cites
T68|.|split a group by directory when one dir's need differs (itok: 1 doc of Han text turned `*.md` into `any`) ∴ `docs/x/*.md any` + `*.md ascii+caveman`, ⊥ 1 file widening the type|V128
