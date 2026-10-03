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
- crate `characterminator`, bin `ctrm` (V92).
- CPU only, offline, deterministic. ⊥ network, ⊥ model.
- SPEC.md FORMAT = cavekit **4.1.0** as vendored by `microlith` (upstream rev `c322f0b`) + its `FORMAT-EXTENSIONS.md` (`§F`/`§N`). Form gated by `mth`, ⊥ restated here (V14).
- token counting = `itok` lib dep (crates.io 0.3, `default-features = false`, `features = ["bpe"]`). ⊥ own tokenizer, ⊥ own bytes/4.
- fs walk = `itok::walk::tracked`. `itok::glob::matches` & `itok::estimate::select_paths` = `pub(crate)` ∴ OWN glob matcher (zero-dep, `src/rules:T42`) & explicit-path selection lives in `src/tokens`.
- federation = `sherd`: dir = node = Rust module (dir + `mod.rs`), ⊥ 2018 `foo.rs`+`foo/`.
- gate runner = `hk` from `nix-hk`; ops in `hk.pkl`; schema vendored `pkl/Config.pkl` ∴ gate runs w/ ⊥ network.
- dev shell = `flake.nix`; ∀ inputs follow `nixpkgs-lock`; pins: `microlith` `v0.7.3`, `itok` `v0.3.1`, `sherd` `v0.5.2`. pin policy = V44.
- source ASCII-only (dogfood, V13).
- deps minimal; each direct dep justified in `docs/THIRD-PARTY-NOTICES.md`. direct: `itok` · `unicode-normalization` & `unicode-security` (0.1, `default-features = false`; UCD tables for pedantic, `src/lint:V58`).
- ⊥ non-public repo named in spec, source, or commit message. private repos ? scanned & improved; their data cited as anonymous counts only. named only once verified public; unknown → private (fail closed).

## §I INTERFACES

- cmd: `ctrm check | fix | stats | explain | sets | guard`; each verb's contract → `src/cli` §I. exit codes below.
- adopt: `.pre-commit-hooks.yaml` (ids `ctrm-check`, `ctrm-fix`) · `action.yml` (composite; inputs `args`, `output`, `working-directory`). both run the SAME `ctrm` binary (V52).
- file grammars live w/ their parser: `.ctrm` → `src/rules` §I · `.ctrm-map` → `src/fix` §I · `.ctrm-sets` & builtin presets → `src/charset` §I.
- flag: full list, per-verb `--format` & refusal rules → `src/cli` §I.
- lib: `characterminator::scan::scan_str`, `fix::fix`, `rules::resolve` — pure fns, ⊥ I/O. API rework pending; this line = the paths that exist today.
- exit: 0 ok · 1 violation | drift · 2 = ⊥ verdict reached: usage, config (`src/cli:V74`), write error (`src/cli:V47`), dir w/ ⊥ tracked file (`src/tokens:V43`), ⊥ git work tree (`src/tokens:V69`) · `guard`: 0 decided | 1 adapter failure, 2 ⊥ EVER (`src/cli/guard:V53`).

## §V INVARIANTS

V13: dogfood: `ctrm check` gates own tree (`hk.pkl` step `ctrm`). `.ctrm` grants, last match wins: `*` `ascii` · `*.md` `ascii+spec` (custom set, `.ctrm-sets`) ∵ specs DOCUMENT the chars they govern · `README.md` `AGENTS.md` `LICENSE` `docs/**/*.md` back to `ascii` ∵ the front page argues plain text · `.context-limits` `ascii+caveman` ∵ it cites `§` · `pkl/Config.pkl` `any` ∵ vendored upstream schema, ⊥ ours to rewrite.
V14: SPEC.md form gated by `mth fmt --check SPEC.md` & `mth check --records .spec-records SPEC.md`. `mth` absent → gate FAILS hard, ⊥ skip. ∀ node SPEC.md, ⊥ root only.
V15: SPEC.md capped from commit one: `.context-limits` row gated by `itok check`; ceiling ~12% over measured.
V16: `sherd check`, `sherd budget` & `sherd sync --check` gate; federation live since the split ∴ ⊥ deferred.
V40: dev shell installs a hook ONLY into its OWN repo (crate-name marker @ worktree root) & ONLY into an UNTRACKED hooks dir ∵ `core.hooksPath` ? be tracked. refusal LOUD. runner: hk step `hook-guard` (text: both guards precede every `install`).

V44: ∀ flake input pins a TAG, bumped in its OWN reviewed commit. FOLLOWING a branch REJECTED ∵ what the gate enforces ? then change w/ ⊥ diff to read, & a gate whose rules move unreviewed gates ⊥. `nixpkgs-lock` & `nix-hk` = the fleet authorities, pinned by their own lock ∴ exempt.

V46: line coverage GATED, ⊥ reported. FLOOR = hard min; `.coverage` = CEILING, the figure the badge CLAIMS. measured < floor → FAIL. measured < claim → FAIL ∵ badge OVERSTATES. measured − claim > 0.5 → FAIL ∵ badge stale. `cargo llvm-cov` needs llvm tools matching `rustc`s LLVM ∴ gate asserts the majors agree: a mismatch reads as a crash, ⊥ as a pin.

V52: adoption surfaces (pre-commit hook, GitHub Action) WRAP the `ctrm` binary, built from the consumer-pinned rev; ⊥ own rule logic ∵ a wrapper that judged files itself drifts from the tool it is named after. action: inputs reach the shell via `env` ONLY, ⊥ `${{ }}` in `run:` ∵ template injection; exit status preserved when `output` redirects. CI runs the action via `uses: ./` ∵ a wrapper nothing exercises ? rot unseen.

V92: crate `characterminator`, bin `ctrm` (`rg`/`mth` shape: crate carries meaning, bin carries muscle memory). `ctr` REJECTED ∵ containerd ships `cmd/ctr`. `ctrm` free on crates.io, ⊥ found on PATH. runner: hk step `ctrm` = `cargo run --bin ctrm` ∴ a rename fails the gate; the record → `.spec-records`.

## §T TASKS

id|status|task|cites
T1|x|ARCHIVED to SPEC-ARCHIVE.md|-
T2|x|ARCHIVED to SPEC-ARCHIVE.md|V14,V15,V16
T3|x|ARCHIVED to SPEC-ARCHIVE.md|V14,V16
T4|x|ARCHIVED to SPEC-ARCHIVE.md|V15,V14
T13|x|ARCHIVED to SPEC-ARCHIVE.md|V13
T14|x|ARCHIVED to SPEC-ARCHIVE.md|V16
T16|x|ARCHIVED to SPEC-ARCHIVE.md|-
T17|~|release: `release.toml` (`cargo-release`) + crates.io publish, ∵ T13 & T16 land|-
T27|x|ARCHIVED to SPEC-ARCHIVE.md|`src/tokens:V10`,`src/cli:V7`
T28|.|dogfood wave 2: extend to rest of fleet (`src/charset:R4`) once wave 1 confirms presets|`src/charset:V23`
T40|x|ARCHIVED to SPEC-ARCHIVE.md|`src:V38`,`src:V39`
T45|x|ARCHIVED to SPEC-ARCHIVE.md|V14,V16
T48|x|ARCHIVED to SPEC-ARCHIVE.md|V44
T51|x|ARCHIVED to SPEC-ARCHIVE.md|V46
T54|x|ARCHIVED to SPEC-ARCHIVE.md|V52

## §B BUGS

id|date|cause|fix
B1|2026-09-18|dev shell installed hooks at `git rev-parse --git-path hooks` w/ ⊥ repo check ∴ entered from a sibling, overwrote its TRACKED hooks & downgraded that gate refuse → skip|V40
