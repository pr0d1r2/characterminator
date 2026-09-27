# SPEC ARCHIVE

Task rows moved out of `SPEC.md` by `mth archive`. An id is never reused
(`V12`), so a citation to an archived row still resolves -- here.

This is a SINK, not a spec. The citations inside these rows point into
`SPEC.md`, so `mth check` on this file reports every one of them as dangling,
correctly and uselessly. The verb that reads it is `mth tasks`.

## §T TASKS

T1|x|scaffold crate: `Cargo.toml` edition 2024, MSRV 1.95, lints, lib+bin, `rustfmt.toml`, `clippy.toml`|-
T2|x|`flake.nix` dev shell: `nixpkgs-lock`, `nix-hk`, `microlith`, `itok`, `sherd` inputs, ∀ following `nixpkgs-lock`|V14,V15,V16
T3|x|`hk.pkl` gate: fmt, clippy `-D warnings`, test, `mth` & `mth-check` ∀ node spec, `sherd check`, `sherd sync --check`; schema vendored `pkl/Config.pkl`|V14,V16
T4|x|`.context-limits` per node; `.spec-records` baseline; then `itok check` & `mth check --records` steps in `hk.pkl`|V15,V14
T13|x|dogfood: `.ctrm` for own tree, `check` step in `hk.pkl`. BLOCKED til T22/T23 ship sets|V13
T16|x|README, AGENTS.md, `docs/LLM-DISCLAIMER.md`, `docs/THIRD-PARTY-NOTICES.md` per fleet|-
T40|x|seam: public type vocabulary per node (`src/*/mod.rs`), ⊥ logic ∴ ∀ node buildable in parallel|`src:V38`,`src:V39`
