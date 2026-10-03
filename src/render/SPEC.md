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
sib|src/judge|findings from rules, sets, lints, scan & fix; config assembly
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §R RESEARCH

id|topic|finding|src
R17|cost at scale|11 MB, 1.82 M findings, `ascii`: human 1.1 s 753 MB → 0.3 s 238 MB; json 4.8 s 2.2 GB → 0.53 s 448 MB; sarif 11 s 2.8 GB → 1.0 s 578 MB. cause: `String` per field & path char, 3 copies per finding, UCD lookups unasked. floor = findings 64 B + `Report.text`. `--pedantic` 10 MB ASCII 0.5 s 320 MB → 0.19 s 13 MB. 1 ZWJ in 13 MB: 0.45 → 0.14 s. 5k files `zh`: 0.13 → 0.09 s. startup 4.3 → 3.9 ms. output byte-identical|2026-10-03 macOS arm64 release, `time -l`, noisy host

## §V INVARIANTS

V11: `--format json` = stable contract; human output cosmetic, but its paths ⊥ raw: ∀ char ∉ printable ASCII → `<U+XXXX>` (as `src/cli/guard:V53`) ∵ a file name w/ ESC or bidi controls injected into the terminal (B31). json escapes ∴ unchanged.
V50: `check --format sarif` → SARIF 2.1.0 log (code scanning). ∀ violation = 1 `result`: `ruleId` = lint name; `level` forbid|deny → `error`, warn → `warning`; message = code point + set; `region` = `startLine` `startColumn` `endColumn` (exclusive, 1 char). `run.columnKind` = `unicodeCodePoints` ∵ `src/scan` column counts chars ⊥ UTF-16 units (SARIF default). `tool.driver.rules` = lint registry (`src/lint`) ⊥ hand list. uri = path rel. to run dir, percent-encoded. unread file → `toolExecutionNotifications` ⊥ dropped (`src/scan:V8`). deterministic: report order, ⊥ timestamp, ⊥ absolute root. exit code = `check`'s. other verbs ∖ `guard` (argv ignored, `src/cli/guard:V93`) refuse `sarif`, exit 2 ∵ empty log reads as clean run.
V94: human `check` row = `path:line:col U+XXXX <last>`, 1 per finding. `<last>` = set judged against for `charset` & `hazard` groups (hazard → `hazard`), LINT name for `pedantic` ∵ the set GRANTS a pedantic char (trailing U+0020) ∴ naming it reads as if the char fell outside. json keeps `set` & `lint` apart. level `warn` → ` (warn)` appended ∵ a warn row read as a failing one; deny & forbid ⊥ suffix ∵ both fail & forbid = every hazard's default. unread file → `path: invalid UTF-8 at byte N` | `path: skipped, binary`. test: `human.rs` (`a_pedantic_finding_names_its_lint_in_the_last_column`, `a_warn_finding_names_its_level`).
V95: json keys per verb (V11 contract), written out w/ units & enums in `docs/JSON.md`; ∀ listed key present (empty → `[]`, absent → `null`). ∀ doc starts {`schema` `verb`}. `check` {`violations` `skipped`}; violation {`path` `line` `column` `byte` `codepoint` `character` `set` `lint` `level` `replacement` `fixable`}; skipped {`path` `reason`: `not-utf8` (+ `byte`) | `binary`}. `fix` {`rewrites` `unmapped` `skipped`}; rewrite = violation + `to` ∴ 1 shape ∀ verb; unmapped = violation. `stats` {`files` `total` `skipped`}; total = file − `path` + `files`; file {`path` `outside` `bytes` `tokens_now` `tokens_after`}; count {`tokens` `method`: `estimate` | `bpe`}. `sets` {`containing` `sets` `unlisted_locales`}; set {`name` `kind`: `preset` | `declared` | `locale` `description` `ranges`: [{`start` `end`}]} (`src/cli/explain:V127`). `explain` {`path` `set` `rule` `effective`}; rule {`pattern` `sets` `family` `levels` `default_level` `origin`}; effective {`family` `default_level` `levels`}. key renamed | dropped = contract break. runner: whole-document tests, `json_test.rs`.
V96: human lines, other verbs (V94 shape): `fix` → `path:line:col U+XXXX -> "<to>"` per rewrite, then unmapped as `left: ` + `check` row ∵ a bare leftover read as 1 more rewrite, then unread. `stats` → `path outside=N bytes=B tokens=<now>-><after>` ∵ `outside 3 bytes 26` read 3 as bytes; pair `~N->~M` (estimate) | `N->M (o200k)` (bpe, `src/tokens:V10`); > 1 file counted → last `TOTAL files=F ...` (json `total`, null ⊥ file); then unread. `sets` → `<name> -- <description>` then `  <range>...`, range `U+XXXX` | `U+XXXX-U+YYYY`; ⊥ range → `none`; then `<N> CLDR locale sets: ...` when counted. test: `human.rs`.
V122: `schema` = 1; +1 only when a key is renamed, retyped, dropped or changes meaning; key added ⊥ bump ∴ consumer ignores unknown keys. `replacement` = text `fix` writes for the span STARTING at the char, `null` ⊥ such span; `fixable` = `fix` clears the char (next `check` ⊥ it). both from RUNNING the fix (`src/cli` `remedy.rs`) ⊥ map lookup ∵ family, classes, sequences & `words` decide; file `fix` refuses → `null`/`false`. json only ∵ a 2nd pass per file w/ a finding. `--format json` & run fails → stdout `{schema verb error}` + stderr, exit 2 ∵ a script parsing json got an empty stream. runner: `check_json_test.rs`, `json_test.rs`, `tests/surface.rs`.
V123: human `fix` ends w/ 1 tally line on STDERR (stdout stays rows): `rewrote N, M left (no map entry): grant a set in .ctrm or edit by hand`; `--check` → `would rewrite N`; M = 0 → `, nothing left`; N = M = 0 → ⊥ line ∵ a clean run stays silent. json & sarif ⊥ tally. exit ⊥ changed. test: `human.rs` (`a_fix_tally_says_what_was_rewritten_and_what_is_left`).
V124: human `check` ends w/ 1 tally line on STDERR, only when it found something: `N findings in F files; most: U+XXXX xN, ... -- see 'ctrm sets --containing U+XXXX' or 'ctrm init'` (top 3 code points) ∵ 2,136 rows w/ ⊥ total & 10k findings ≈ 80k tokens for an agent. `--summary` → 1 row per (path, code point, lint, set): first occurrence + ` xN` + ` -> "<to>"` (human); json {`schema` `verb` `groups` `skipped`}, group = violation + `count`. `--max N` → human rows cut at N + `... M more`; json & sarif refuse `--max`, sarif refuses `--summary`, exit 2 ∵ ignored flag ⊥ silent. tally counts ALL findings. test: `group_test.rs`, `check_shape_test.rs`.

## §T TASKS

id|status|task|cites
T9|x|ARCHIVED to SPEC-ARCHIVE.md|V11
T43|x|ARCHIVED to SPEC-ARCHIVE.md|V11
T52|x|ARCHIVED to SPEC-ARCHIVE.md|V50,`src/lint:V36`

## §B BUGS

id|date|cause|fix
B31|2026-10-02|human report printed paths raw ∴ a tracked `e<ESC>[2Jx<U+202E>y.md` cleared the screen & reversed its line. via release review|V11
