# SPEC

## §G GOAL

Say it once: human table & the stable json contract.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/render|human & json output, stable json contract
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §V INVARIANTS

V11: `--format json` = stable contract; human output cosmetic.
V50: `check --format sarif` → SARIF 2.1.0 log (code scanning). ∀ violation = 1 `result`: `ruleId` = lint name; `level` forbid|deny → `error`, warn → `warning`; message = code point + set; `region` = `startLine` `startColumn` `endColumn` (exclusive, 1 char). `run.columnKind` = `unicodeCodePoints` ∵ `src/scan` column counts chars ⊥ UTF-16 units (SARIF default). `tool.driver.rules` = lint registry (`src/lint`) ⊥ hand list. uri = path rel. to run dir, percent-encoded. unread file → `toolExecutionNotifications` ⊥ dropped (`src/scan:V8`). deterministic: report order, ⊥ timestamp, ⊥ absolute root. exit code = `check`'s. other verbs refuse `sarif`, exit 2 ∵ empty log reads as clean run.

## §T TASKS

id|status|task|cites
T9|x|`--format json` ∀ verbs|V11
T43|x|json carries a rule's `default_level` ∴ `explain` json ⊥ silent on it|V11
T52|x|`--format sarif` for `check`: renderer + cli flag, whole-document tests|V50,`src/lint:V36`
