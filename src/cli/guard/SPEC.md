# SPEC

## §G GOAL

`guard` hook adapter: harness hook JSON in, decision JSON out; only a hazard blocks. ⊥ detection (`src/lint`), ⊥ dispatch (`src/cli`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter
self|src/cli/guard|`guard` hook adapter: harness payload in, hook decision out, JSON reader

## §V INVARIANTS

V35: `guard` = hook adapter (itok `guard` shape): harness hook JSON stdin → decision JSON stdout; signal in JSON ⊥ exit code. before file read (`tool_name` = `Read` only; `Edit`, `Write` & co → pass, ⊥ judged): hazard (`src/lint:V34`) → block, naming path, line, code point. after tool output (web fetch, web search, shell): hazard → blocking decision w/ reason = content tainted. ⊥ daemon, ⊥ silent strip. non-hazard violation → PASS + note naming lint & count ∴ hook stays usable; only hazard blocks. a hook that blocks on every stray char is one people switch off, & a switched-off hook gates ⊥.
V53: `guard` exit ⊥ verdict: 0 = decided (decision JSON | silence = pass). adapter failure (stdin ⊥ JSON, ⊥ `hook_event_name`, lost write) → named on stderr, exit 1 (overrides `src/cli:V47`'s 2). exit 2 ⊥ EVER ∵ Claude Code reads 2 as block ∴ a broken adapter would brick the session; 1 = non-blocking error. hazards ⊥ depend on config: `.ctrm` unusable → hazards still judged (`src/lint:V49`) + note. tool output judged for hazard ONLY ∵ ⊥ path for `.ctrm` & `ascii` default ⇒ note on every non-ASCII page. tool output has ⊥ file start ∴ BOM at a string's byte 0 = `stray-bom`, ⊥ signature exemption (`src/lint:V34`). reason quotes data ∴ its non-ASCII → `<U+XXXX>` ∵ a reason ⊥ smuggles what it warns of.
V66: pre-read of a file ⊥ text (`src/scan:V8`): harness decodes it LOSSILY ∴ guard judges the lossy decode for hazard ONLY ∖ `control-character` (C0 = every binary) → hazard = deny, else pass + note. `check` stays ⊥ lossy: guard judges what the model is SHOWN.
V67: stdin ⊥ parsed (⊥ JSON, nested past the depth bound) → raw scan, every `\u` escape decoded (pairs too), hazard ONLY → `block` decision, exit 0 ∵ an attacker shapes MCP output & V53's exit 1 lets it through. clean → V53's named error, exit 1.

## §T TASKS

id|status|task|cites
T37|x|ARCHIVED to SPEC-ARCHIVE.md|V35,V53,`src/lint:V34`

## §B BUGS

id|date|cause|fix
B16|2026-10-02|guard output scan reused the FILE byte-0 BOM exemption per string ∴ `"\ufeff..."` in tool output passed. via release review|V53
B17|2026-10-02|pre-read PASSED a file ⊥ text ∴ U+202E + a `\xff` byte read in silence; the harness showed the model the override. via release review|V66
B18|2026-10-02|PostToolUse nested > 256 deep = parse error → exit 1, ⊥ block ∴ a hazard in deep MCP output passed. via release review|V67
B37|2026-10-02|guard judged ∀ PreToolUse w/ `tool_input.file_path` as a Read ∴ an Edit of a hazard file was denied as "Read blocked". via release review|V35
