# SPEC

## §G GOAL

`characterminator` — Rust toolkit: find & eliminate chars outside allowed set, per file type & per file, so text costs fewer tokens. Default = pure ASCII; extended sets (locales, i18n data) granted only where declared.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI|repo gate, dogfood waves, release|-

## §N NAV

rel|path|lens
up|-|-
self|.|-

## §C CONSTRAINTS

- Rust **edition 2024**, stable, MSRV **1.95** = fleet pin (`nixpkgs-lock` → nixos-26.05). One crate: lib + bin. MIT. `unsafe_code` FORBID.
- bin name `characterminator` ? short alias (`rg`/`mth` shape) TBD. `ctr` REJECTED: containerd ships `cmd/ctr`.
- CPU only, offline, deterministic. ⊥ network, ⊥ model.
- SPEC.md FORMAT = cavekit **4.1.0** as vendored by `microlith` (upstream rev `c322f0b`) + its `FORMAT-EXTENSIONS.md` (`§F`/`§N`). Form gated by `mth`, ⊥ restated here (V14).
- token counting = `itok` lib dep (crates.io 0.3, `default-features = false`, `features = ["bpe"]`). ⊥ own tokenizer, ⊥ own bytes/4.
- fs walk = `itok::walk::tracked`. `itok::glob::matches` = `pub(crate)` ∴ own glob matcher | `globset` dep ?.
- federation = `sherd`: dir = node = Rust module (dir + `mod.rs`), ⊥ 2018 `foo.rs`+`foo/`.
- gate runner = `hk` from `nix-hk`; ops in `hk.pkl`; schema vendored `pkl/Config.pkl` ∴ gate runs w/ ⊥ network.
- dev shell = `flake.nix`; ∀ inputs follow `nixpkgs-lock`; pins: `microlith` `v0.7.0` ?, `itok` `v0.3.1`, `sherd` `v0.5.0`.
- source ASCII-only (dogfood, V13).
- deps minimal; each direct dep justified in `docs/THIRD-PARTY-NOTICES.md`.
- ⊥ non-public repo named in spec, source, or commit message. private repos ? scanned & improved; their data cited as anonymous counts only. named only once verified public; unknown → private (fail closed).

## §I INTERFACES

- cmd: `characterminator check [paths]` → violations `path:line:col U+XXXX <set>`, 1 per line. 0 clean / 1 violation / 2 usage.
- cmd: `characterminator fix [--check] [paths]` → rewrite disallowed chars via transliteration map. `--check` reports & writes ⊥, exit 1 on drift (`rustfmt` grammar).
- cmd: `characterminator stats [--bpe] [paths]` → per file: chars outside set, bytes, tokens now vs after `fix`, via `itok`. report-only.
- cmd: `characterminator explain [<path>] [--as args|lines|prompt]` → effective set + winning rule + its config line; `--as` renders effective config as flags, data-file lines, or agent prompt (`src/cli:V32`).
- cmd: `characterminator sets` → builtin sets & members.
- cmd: `characterminator guard` → hook adapter: harness hook JSON stdin → decision JSON stdout; fuse on hazard (`src/cli:V35`).
- flag: `--format human|json` ∀ verbs · `-C <dir>`.
- file: `.characterminator` ? — line-based, `<glob|path> <set>[+<set>...] [@<family>] [!<level>] [!<lint|group>=<level>]`, `#` comment. zero-dep parse (`.context-limits` shape).
- file: `.characterminator-map` ? — transliteration, `<from> <to>`: `from` = literal char | `U+XXXX`; `to` = replacement, empty = explicit delete. `to` ∉ target file's set → char counts unmapped (`src/fix:V4`). + `family <name> <parent>` (`src/fix:V27`). + `= <class> <family>:<member>[,<member>...] ...` (`src/fix:V28`).
- file: `.characterminator-sets` ? — custom sets, `<name> <member>...`: member = literal char | `U+XXXX` | `U+XXXX-U+YYYY` | set name (≥2 chars, `src/charset:V25`).
- flag (file twins, repeatable, `src/rules:V18`): `--rule <line>` · `--map <line>` · `--set <line>` · `--rules-file <f>` · `--map-file <f>` · `--sets-file <f>` · `--no-files` (skip discovered dotfiles) · `--no-builtin-map` · `--no-builtin-sets` · `--fidelity <family>` (`src/rules:V29`) · `--strict` (`src/lint:V36`) · `--pedantic` (`src/lint:V37`). names ?.
- sets (builtin function presets, `src/charset:V23`): `ascii` intrinsic (U+0020–U+007E + `\t` `\n`, `src/rules:V21`) · `caveman` (FORMAT.md symbols `→∴∀∃⊥≠∈∉≤≥§` + measured `⇒∵·×∨∧¬←↔⇔≈∅⊆∪∩↑↓`, `src/charset:R3`) · `box` (U+2500–U+257F, U+25A0–U+25FF) · `marks` (per fidelity, `src/rules:V29`: `✓✗⚠→` @`text`, `✅❌⚠➡` @`emoji`) · `math` (`×÷±°²³≤≥≠≈∞µ√∑π`) · `legal` (`©®™`) · `emoji` (single code points: U+1F300–U+1FAFF, U+2600–U+27BF; sequences compress, `src/fix:V31`) · letters ∀ CLDR locale (`pl`, `de`, `fr`, `ja`, …, `src/charset:V30`) · `cr` (`\r`) · `typography` ? (`src/fix:V26`) · coarse blocks `latin1` (U+0080–U+00FF), `latin-ext` (U+0100–U+017F), `cyrillic`, `greek`, `arabic` · `hazard` (`src/lint:V34`, ∉ `any`) · `any`. custom sets via `.characterminator-sets`.
- lib: `characterminator::{scan, fix, resolve}` — pure fn over `&str`.
- exit: 0 ok · 1 violation | drift · 2 usage.

## §V INVARIANTS

V13: dogfood: `characterminator check` gates own tree in `hk.pkl`; `.characterminator` grants `SPEC.md` `ascii+caveman`, rest `ascii`.
V14: SPEC.md form gated by `mth fmt --check SPEC.md` & `mth check --records .spec-records SPEC.md`. `mth` absent → gate FAILS hard, ⊥ skip. ∀ node SPEC.md, ⊥ root only.
V15: SPEC.md capped from commit one: `.context-limits` row gated by `itok check`; ceiling ~12% over measured.
V16: `sherd check`, `sherd budget` & `sherd sync --check` gate; federation live since the split ∴ ⊥ deferred.

## §T TASKS

id|status|task|cites
T1|.|scaffold crate: `Cargo.toml` edition 2024, MSRV 1.95, lints, lib+bin, `rustfmt.toml`, `clippy.toml`|-
T2|.|`flake.nix` dev shell: `nixpkgs-lock`, `nix-hk`, `microlith`, `itok`, `sherd` inputs, ∀ following `nixpkgs-lock`|V14,V15,V16
T3|.|`hk.pkl` gate: fmt, clippy `-D warnings`, test, `mth`, `mth-check`, `itok check`, `sherd check`|V14,V15,V16
T4|.|`.context-limits` per node; `.spec-records` baseline|V15
T13|.|dogfood: `.characterminator` for own tree, `check` step in `hk.pkl`|V13
T14|.|`sherd check` + `sherd budget` + `sherd sync --check` in `hk.pkl`|V16
T16|.|README, AGENTS.md, `docs/LLM-DISCLAIMER.md`, `docs/THIRD-PARTY-NOTICES.md` per fleet|-
T17|.|release: `release.toml` (`cargo-release`), crates.io publish ?|-
T27|.|dogfood wave 1: `check` + `stats` over sibling Rust repos (`src/charset:R1`); record anonymized savings in §R; fixes land via each repo's own review|`src/tokens:V10`,`src/cli:V7`
T28|.|dogfood wave 2: extend to rest of fleet (`src/charset:R4`) once wave 1 confirms presets|`src/charset:V23`

## §B BUGS

id|date|cause|fix
