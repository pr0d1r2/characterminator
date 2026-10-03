# SPEC

## §G GOAL

`pedantic` lint group: opt-in purity lints, their char, line & text walks, the claim order; the one facade for `unicode-normalization` & `unicode-security` (`ucd.rs` only, `src:C`). ⊥ vocabulary & levels (`src/lint`).

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
up|src/lint|lint names, groups, levels, hazard, pedantic
self|src/lint/pedantic|pedantic lints: walks, claim order, Unicode crates facade
sib|src/lint/hazard|hazard lints: classes, compiled-in detection, joiner, tag & VS16 exemptions

## §V INVARIANTS

V37: `pedantic` group = maximum purity, opt-in (`--pedantic` ≡ `--rule '* !pedantic=warn'`), clippy::pedantic philosophy: ? flags legit text ∴ ⊥ default. lints SHIPPED (group stays `allow` ∴ ⊥ fire until asked): `not-nfc` · `nfkc-compat` (fullwidth, ligatures, superscripts → ASCII-foldable) · `unicode-space` (Zs ∖ U+0020, V55) · `mixed-script` · `confusable` (UTS #39) · `crlf` · `trailing-whitespace` · `final-newline`. PENDING T58, ⊥ registered: `locale-literal` (non-ASCII string literal in a code file → move to a locale file, `src/rules:V17`). lint → default group only after near-zero false positives in dogfood waves.
V55: pedantic, ⊥ vendored data (T39). `unicode-space` = Zs ∖ U+0020 (U+00A0, U+1680, U+2000–U+200A, U+202F, U+205F, U+3000). finding = 1 char IN the file ∴ `Hit`, json, SARIF unchanged: `crlf` → the CR of a CR LF · `trailing-whitespace` → 1st char of a line's trailing White_Space run · `final-newline` → last char of a non-empty file, ≠ LF. 1 char ≤ 1 claim ACROSS char, line & text walks: hazard > `outside-set` > `unicode-space` > `crlf` > `trailing-whitespace` > `final-newline` ∴ `ascii` (⊥ `cr`) reports CR LF as `outside-set`. human last column = lint name (`src/render:V94`). exemption: later rule `<glob> !<lint>=allow`; names ⊥ set ∴ grant untouched (`src/rules:V56`).
V58: pedantic via `unicode-normalization` & `unicode-security`, call site `ucd.rs` only (`src:C`). 1 char IN file (V55 shape): `nfkc-compat` → NFKC(c) ≠ NFC(c) · `confusable` → non-ASCII c, UTS #39 skeleton non-empty ASCII · `not-nfc` → per NFC segment (UAX #15 boundary: ccc 0 ∧ NFC_QC=Yes) NFC changes, 1st char differing · `mixed-script` → per word (alnum ∪ marks), 1st char emptying UTS #39 resolved script set. Zs → `unicode-space` only. claim order: V55 then `nfkc-compat` > `confusable` > `not-nfc` > `mixed-script`. ⊥ asked → ⊥ lookup.

## §T TASKS

id|status|task|cites
T39|x|ARCHIVED to SPEC-ARCHIVE.md|V37,`src/lint:V36`,V55,V58
T58|.|`locale-literal` (V37): needs code-file & string-literal notion ⊥ in tree; ⊥ registered til built|V37,`src/rules:V17`

## §B BUGS

id|date|cause|fix
B20|2026-10-02|dedup ran per walk ∴ `a ` = `trailing-whitespace` + `final-newline` at 1:2, `a` U+03BB = `final-newline` + `mixed-script`. via release review|V55
