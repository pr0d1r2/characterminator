# SPEC

## §G GOAL

`characterminator` — Rust toolkit: find & eliminate chars outside allowed set, per file type & per file, so text costs fewer tokens. Default = pure ASCII; extended sets (locales, i18n data) granted only where declared.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|charset model, rule resolution, scan, fix, verbs, lib facade|consumer locale data, tokenizer & fs walk (`itok`'s)|-

## §N NAV

rel|path|lens
up|-|-
self|.|-

## §C CONSTRAINTS

- Rust **edition 2024**, stable, MSRV **1.95** = fleet pin (`nixpkgs-lock` → nixos-26.05). One crate: lib + bin. MIT. `unsafe_code` FORBID.
- SPEC.md FORMAT = cavekit **4.1.0** as vendored by `microlith` (upstream rev `c322f0b`) + its `FORMAT-EXTENSIONS.md` (`§F`/`§N`). Form gated by `mth`, ⊥ restated here (V14).
- federation = `sherd`: dir = node = Rust module (dir + `mod.rs`), ⊥ 2018 `foo.rs`+`foo/`.
- deps minimal; each direct dep justified in `docs/THIRD-PARTY-NOTICES.md`.

## §V INVARIANTS

V14: SPEC.md form gated by `mth fmt --check SPEC.md` & `mth check --records .spec-records SPEC.md`. `mth` absent → gate FAILS hard, ⊥ skip.
V15: SPEC.md capped from commit one: `.context-limits` row gated by `itok check`; ceiling ~12% over measured.
V16: `sherd check` & `sherd budget` gate once ≥1 child node exists.

## §T TASKS

id|status|task|cites
T1|.|scaffold crate: `Cargo.toml` edition 2024, MSRV 1.95, lints, lib+bin, `rustfmt.toml`, `clippy.toml`|-
T4|.|`.context-limits` SPEC.md ceiling; `.spec-records` baseline|V15
T14|.|`sherd check` + `sherd budget` in `hk.pkl` when first child node lands|V16

## §B BUGS

id|date|cause|fix
