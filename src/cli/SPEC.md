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

V7: only `check` & `fix --check` gate. bare `fix` rewrites only on explicit call; exit 1 only if an unmapped char is LEFT (`src/fix:V4`), ⊥ for what it repaired. `stats`/`explain`/`sets` report-only, exit 0.
V32: `explain --as args|lines|prompt` renders effective config (per path | whole repo): `args` = flags, `lines` = data-file lines, `prompt` = deterministic agent instruction (allowed chars, replacements, fidelity) for drafting compliant code up front. round trip: `--as args` output fed back ≡ same config, property-tested w/ `src/rules:V18`. ⊥ model (CPU only).
V35: `guard` = hook adapter (itok `guard` shape): harness hook JSON stdin → decision JSON stdout; signal in JSON ⊥ exit code. before file read (`tool_name` = `Read` only; `Edit`, `Write` & co → pass, ⊥ judged): hazard (`src/lint:V34`) → block, naming path, line, code point. after tool output (web fetch, web search, shell): hazard → blocking decision w/ reason = content tainted. ⊥ daemon, ⊥ silent strip. non-hazard violation → PASS + note naming lint & count ∴ hook stays usable; only hazard blocks. a hook that blocks on every stray char is one people switch off, & a switched-off hook gates ⊥.
V47: stdout closed by reader (EPIPE, `ctrm check | head`) → stop writing, keep verdict, ⊥ panic. other write error → named, exit 2 ∵ lost output ⊥ silent. ∀ verb. LIMIT: a stdout CLOSED before start (`ctrm check >&-`) ⊥ detectable: the Rust runtime reopens fd 1 on `/dev/null` before `main` & `unsafe` is forbidden ∴ ≡ discarded output, exit by verdict.
V53: `guard` exit ⊥ verdict: 0 = decided (decision JSON | silence = pass). adapter failure (stdin ⊥ JSON, ⊥ `hook_event_name`, lost write) → named on stderr, exit 1 (overrides V47's 2). exit 2 ⊥ EVER ∵ Claude Code reads 2 as block ∴ a broken adapter would brick the session; 1 = non-blocking error. hazards ⊥ depend on config: `.ctrm` unusable → hazards still judged (`src/lint:V49`) + note. tool output judged for hazard ONLY ∵ ⊥ path for `.ctrm` & `ascii` default ⇒ note on every non-ASCII page. tool output has ⊥ file start ∴ BOM at a string's byte 0 = `stray-bom`, ⊥ signature exemption (`src/lint:V34`). reason quotes data ∴ its non-ASCII → `<U+XXXX>` ∵ a reason ⊥ smuggles what it warns of.
V66: pre-read of a file ⊥ text (`src/scan:V8`): harness decodes it LOSSILY ∴ guard judges the lossy decode for hazard ONLY ∖ `control-character` (C0 = every binary) → hazard = deny, else pass + note. `check` stays ⊥ lossy: guard judges what the model is SHOWN.
V67: stdin ⊥ parsed (⊥ JSON, nested past the depth bound) → raw scan, every `\u` escape decoded (pairs too), hazard ONLY → `block` decision, exit 0 ∵ an attacker shapes MCP output & V53's exit 1 lets it through. clean → V53's named error, exit 1.
V71: path matched (`src/rules`) & shown in LEXICAL normal form: `.` dropped, `..` folded ∴ `sub/../sub/c.md` ≡ `sub/c.md` ∀ anchored rule. symlink ⊥ resolved.
V72: bare `fix` = 2 phases: judge ∀ file, THEN write ∴ a refusal (`src/fix:V5`, `src/fix:V6`) or read error writes ⊥ file.
V73: path arity: `sets` 0, `explain` (any `--as`) ≤ 1, other verbs any. extra path → exit 2 naming it ∵ `explain a.md b.txt` answered for `a.md` only & `sets a.md` ignored it, each read as an answer about what was asked.
V74: ∀ verb ⊥ `guard`: whole config (rules, sets, map) parsed & validated before dispatch; any kind broken → exit 2, naming its origin ∵ a verb reading only its own kind reported on a config that ⊥ parsed (B29). `--map` value = 1 line, as `--set` & `--rule` (`src/rules:V18`).

## §T TASKS

id|status|task|cites
T8|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/tokens:V9`,`.:I.cmd`
T11|x|ARCHIVED to SPEC-ARCHIVE.md|`src/tokens:V10`
T12|x|ARCHIVED to SPEC-ARCHIVE.md|`src/rules:V2`,V7
T34|x|ARCHIVED to SPEC-ARCHIVE.md|V32,`src/rules:V18`
T37|x|ARCHIVED to SPEC-ARCHIVE.md|V35,V53,`src/lint:V34`
T44|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/tokens:V43`
T50|x|ARCHIVED to SPEC-ARCHIVE.md|V7,`src/fix:V4`,`src/rules:V45`
T55|x|ARCHIVED to SPEC-ARCHIVE.md|`src/rules:V18`,`src/rules:V19`,`src/lint:V36`

## §B BUGS

id|date|cause|fix
B3|2026-09-27|report via `println!` → PANIC on closed stdout ∴ `ctrm check \| head` crashed ∀ verb. via wave 1 (`.:R7`)|V47
B16|2026-10-02|guard output scan reused the FILE byte-0 BOM exemption per string ∴ `"\ufeff..."` in tool output passed. via release review|V53
B17|2026-10-02|pre-read PASSED a file ⊥ text ∴ U+202E + a `\xff` byte read in silence; the harness showed the model the override. via release review|V66
B18|2026-10-02|PostToolUse nested > 256 deep = parse error → exit 1, ⊥ block ∴ a hazard in deep MCP output passed. via release review|V67
B21|2026-10-02|`fix` & `stats` asked UTF-8 only, ⊥ `src/scan` binary test ∴ NUL-laden blob `check` skipped → rewritten by `fix`, counted by `stats`. via review|`src/scan:V8`
B22|2026-10-02|`check` exit = findings only, skips ignored ∴ invalid UTF-8 named & exit 0, ∀ format. now: not-UTF-8 skip → 1, binary skip → 0. via review|`src/scan:V8`
B23|2026-10-02|`fix` counted unmapped chars, rendered ⊥ ∴ exit 1 w/ ⊥ output. now per-path hits → `check` rows + json `unmapped`. via review|`src/fix:V4`
B26|2026-10-02|`stats` dropped a not-UTF-8 file w/o a word ∴ a total short of a file read as complete. now binary & not-UTF-8 named as skipped (json `skipped`), exit 0 (V7). via review|`src/scan:V8`
B29|2026-10-02|map parsed only by `fix` & `stats` ∴ `check --map 'U+ZZZZ x y z'` & a 2-line `--map` ran as if the config were valid. via release review|V74
B37|2026-10-02|guard judged ∀ PreToolUse w/ `tool_input.file_path` as a Read ∴ an Edit of a hazard file was denied as "Read blocked". via release review|V35
B38|2026-10-02|a named path w/ `..` (`sub/../sub/c.md`) ⊥ normalised ∴ missed anchored rules & was judged `ascii`. via release review|V71
B39|2026-10-02|`sets a.md` & `explain a.md b.txt` silently ignored the extra path ∴ the run read as if it had judged it. via release review|V73
