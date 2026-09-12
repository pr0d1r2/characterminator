# SPEC

## §G GOAL

`characterminator` — Rust toolkit: find & eliminate chars outside allowed set, per file type & per file, so text costs fewer tokens. Default = pure ASCII; extended sets (locales, i18n data) granted only where declared.

## §C CONSTRAINTS

- Rust **edition 2024**, stable, MSRV **1.95** = fleet pin (`nixpkgs-lock` → nixos-26.05). One crate: lib + bin. MIT. `unsafe_code` FORBID.
- SPEC.md FORMAT = cavekit **4.1.0** as vendored by `microlith` (upstream rev `c322f0b`) + its `FORMAT-EXTENSIONS.md` (`§F`/`§N`). Form gated by `mth`, ⊥ restated here (V14).
- deps minimal; each direct dep justified in `docs/THIRD-PARTY-NOTICES.md`.

## §V INVARIANTS

V14: SPEC.md form gated by `mth fmt --check SPEC.md` & `mth check --records .spec-records SPEC.md`. `mth` absent → gate FAILS hard, ⊥ skip.

## §T TASKS

id|status|task|cites
T1|.|scaffold crate: `Cargo.toml` edition 2024, MSRV 1.95, lints, lib+bin, `rustfmt.toml`, `clippy.toml`|-

## §B BUGS

id|date|cause|fix
