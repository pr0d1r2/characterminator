# SPEC

## §G GOAL

`characterminator` (bin `ctrm`) — Rust toolkit: find & eliminate chars outside allowed set, per file type & per file, so text costs fewer tokens. Default = pure ASCII; extended sets (locales, i18n data) granted only where declared.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI|repo gate, dogfood waves, release|-

## §N NAV

rel|path|lens
up|-|-
self|.|-

## §C CONSTRAINTS

- Rust **edition 2024**, stable, MSRV **1.95** = fleet pin (`nixpkgs-lock` → nixos-26.05). One crate: lib + bin. MIT. `unsafe_code` FORBID.
- crate `characterminator`, bin `ctrm` (`rg`/`mth` shape: crate carries meaning, bin carries muscle memory). `ctr` REJECTED: containerd ships `cmd/ctr`. `ctrm` free on crates.io, ⊥ found on PATH.
- CPU only, offline, deterministic. ⊥ network, ⊥ model.
- SPEC.md FORMAT = cavekit **4.1.0** as vendored by `microlith` (upstream rev `c322f0b`) + its `FORMAT-EXTENSIONS.md` (`§F`/`§N`). Form gated by `mth`, ⊥ restated here (V14).
- token counting = `itok` lib dep (crates.io 0.3, `default-features = false`, `features = ["bpe"]`). ⊥ own tokenizer, ⊥ own bytes/4.
- fs walk = `itok::walk::tracked`. `itok::glob::matches` & `itok::estimate::select_paths` = `pub(crate)` ∴ OWN glob matcher (zero-dep, `src/rules:T42`) & explicit-path selection lives in `src/tokens`.
- federation = `sherd`: dir = node = Rust module (dir + `mod.rs`), ⊥ 2018 `foo.rs`+`foo/`.
- gate runner = `hk` from `nix-hk`; ops in `hk.pkl`; schema vendored `pkl/Config.pkl` ∴ gate runs w/ ⊥ network.
- dev shell = `flake.nix`; ∀ inputs follow `nixpkgs-lock`; pins: `microlith` `v0.7.3` ?, `itok` `v0.3.1`, `sherd` `v0.5.0`.
- source ASCII-only (dogfood, V13).
- deps minimal; each direct dep justified in `docs/THIRD-PARTY-NOTICES.md`.
- ⊥ non-public repo named in spec, source, or commit message. private repos ? scanned & improved; their data cited as anonymous counts only. named only once verified public; unknown → private (fail closed).

## §I INTERFACES

- cmd: `ctrm check [paths]` → violations `path:line:col U+XXXX <set>`, 1 per line. 0 clean / 1 violation / 2 usage.
- cmd: `ctrm fix [--check] [paths]` → rewrite disallowed chars via transliteration map. `--check` reports & writes ⊥, exit 1 on drift (`rustfmt` grammar).
- cmd: `ctrm stats [--bpe] [paths]` → per file: chars outside set, bytes, tokens now vs after `fix`, via `itok`. report-only.
- cmd: `ctrm explain [<path>] [--as args|lines|prompt]` → effective set + winning rule + its config line; `--as` renders effective config as flags, data-file lines, or agent prompt (`src/cli:V32`).
- cmd: `ctrm sets` → builtin sets & members.
- cmd: `ctrm guard` → hook adapter: harness hook JSON stdin → decision JSON stdout; fuse on hazard (`src/cli:V35`).
- flag: `--format human|json` ∀ verbs · `-C <dir>`.
- file grammars live w/ their parser: `.ctrm` → `src/rules` §I · `.ctrm-map` → `src/fix` §I · `.ctrm-sets` & builtin presets → `src/charset` §I.
- flag (file twins, repeatable, `src/rules:V18`): `--rule <line>` · `--map <line>` · `--set <line>` · `--rules-file <f>` · `--map-file <f>` · `--sets-file <f>` · `--no-files` (skip discovered dotfiles) · `--no-builtin-map` · `--no-builtin-sets` · `--fidelity <family>` (`src/rules:V29`) · `--strict` (`src/lint:V36`) · `--pedantic` (`src/lint:V37`).
- lib: `characterminator::{scan, fix, resolve}` — pure fn over `&str`.
- exit: 0 ok · 1 violation | drift · 2 usage.

## §V INVARIANTS

V13: dogfood: `ctrm check` gates own tree in `hk.pkl`; `.ctrm` grants `SPEC.md` `ascii+caveman`, rest `ascii`.
V14: SPEC.md form gated by `mth fmt --check SPEC.md` & `mth check --records .spec-records SPEC.md`. `mth` absent → gate FAILS hard, ⊥ skip. ∀ node SPEC.md, ⊥ root only.
V15: SPEC.md capped from commit one: `.context-limits` row gated by `itok check`; ceiling ~12% over measured.
V16: `sherd check`, `sherd budget` & `sherd sync --check` gate; federation live since the split ∴ ⊥ deferred.
V40: dev shell installs a hook ONLY into its OWN repo (crate-name marker @ worktree root) & ONLY into an UNTRACKED hooks dir ∵ `core.hooksPath` ? be tracked. refusal LOUD.

## §T TASKS

id|status|task|cites
T1|x|scaffold crate: `Cargo.toml` edition 2024, MSRV 1.95, lints, lib+bin, `rustfmt.toml`, `clippy.toml`|-
T2|x|`flake.nix` dev shell: `nixpkgs-lock`, `nix-hk`, `microlith`, `itok`, `sherd` inputs, ∀ following `nixpkgs-lock`|V14,V15,V16
T3|x|`hk.pkl` gate: fmt, clippy `-D warnings`, test, `mth` & `mth-check` ∀ node spec, `sherd check`, `sherd sync --check`; schema vendored `pkl/Config.pkl`|V14,V16
T4|x|`.context-limits` per node; `.spec-records` baseline; then `itok check` & `mth check --records` steps in `hk.pkl`|V15,V14
T13|.|dogfood: `.ctrm` for own tree, `check` step in `hk.pkl`. BLOCKED til T22/T23 ship sets|V13
T14|.|`sherd check` + `sherd budget` + `sherd sync --check` in `hk.pkl`|V16
T16|.|README, AGENTS.md, `docs/LLM-DISCLAIMER.md`, `docs/THIRD-PARTY-NOTICES.md` per fleet|-
T17|.|release: `release.toml` (`cargo-release`), crates.io publish ?|-
T27|.|dogfood wave 1: `check` + `stats` over sibling Rust repos (`src/charset:R1`); record anonymized savings in §R; fixes land via each repo's own review|`src/tokens:V10`,`src/cli:V7`
T28|.|dogfood wave 2: extend to rest of fleet (`src/charset:R4`) once wave 1 confirms presets|`src/charset:V23`
T40|x|seam: public type vocabulary per node (`src/*/mod.rs`), ⊥ logic ∴ ∀ node buildable in parallel|`src:V38`,`src:V39`
T45|.|gate runs UNATTENDED: CI workflow + hooks REFUSE ⊥ skip|V14,V16

## §B BUGS

id|date|cause|fix
B1|2026-09-18|dev shell installed hooks at `git rev-parse --git-path hooks` w/ ⊥ repo check ∴ entered from a sibling, overwrote its TRACKED hooks & downgraded that gate refuse → skip|V40
