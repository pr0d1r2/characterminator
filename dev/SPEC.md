# SPEC

## §G GOAL

`characterminator-dev`: tooling that maintains THIS repo's generated docs & ships to nobody. README badges, the notices closure, & their drift checks.

## §N NAV

rel|path|lens
up|.|-
self|dev|`characterminator-dev`, `publish = false`: README badges, notices closure & their drift checks
sib|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
sib|docs|human reference: memory, LLM disclaimer, third-party notices
sib|tests|integration tests that drive the `ctrm` binary

## §C CONSTRAINTS

- ⊥ published: workspace member `dev/`, `publish = false`, `release = false`. ⊥ 2nd `[[bin]]` in the root package ∵ it would ship & land on a consumer's PATH.
- lints = root's, via `[workspace.lints]` ∴ dev code meets the same denials (`.:AGENTS.md` clippy rules), ⊥ a laxer copy.
- zero deps. ⊥ path dep on `characterminator` until a block renders product data.
- `lib.rs` owns logic; `main.rs` = shim. `run(args, root, external, err)` injectable ∴ ∀ verb testable w/ ⊥ the real tree. ∀ parser = pure fn over `&str`.
- e2e tests run in temp fixture repos, ⊥ the real README.

## §I INTERFACES

- cmd: `characterminator-dev readme|notices [--check]` · `characterminator-dev --check|--fix [<path>...]` (every job, scoped by paths).
- markers: `<!-- BEGIN <name> -->` … `<!-- END <name> -->`, 1 whole line each. blocks: README `badges` · notices `closure`.
- env: `CTRM_DEV_CARGO_TREE=<file>` replaces the `cargo tree` call w/ a recorded answer (tests).
- hk: `dev-generated` (fast, `{{files}}`) · `dev-generated-full` (all, unscoped).

## §V INVARIANTS

V131: ∀ generated value read from its OWNER: `Cargo.toml` (name, license, edition, rust-version, `[dependencies]` keys counted where a key STARTS ∴ a wrapped table counts once, `unsafe_code`), `.coverage` (`lines`), `cargo tree` (closure). absent → exit 1 naming the owner, ⊥ default ∵ a badge from a fallback claims a number nothing checks.
V132: closure = `cargo tree -e normal --target all --locked --offline`, self excluded, deduped. host-only REJECTED ∵ the closure differs by platform (macOS 26 vs 29 all-target) & CI gates 3 OSes ∴ only the all-target set is a number every runner reproduces.
V133: markers match a WHOLE line & the exact name (⊥ prefix: `x` ⊥ matches `xy`). pair missing, duplicated or END before BEGIN → exit 1 naming it, ⊥ rewrite.
V134: render idempotent: splice(splice(x)) == splice(x) ∴ `--check` = equality, writes ⊥. stale → exit 1 + up to 3 `want:`/`have:` lines per block.
V135: a percentage is TRUNCATED to 1 decimal as text, ⊥ float math ∵ platform drift (`.:V46`) & truncation never overstates.
V136: ∀ block declares its INPUTS; `--check <paths>` checks blocks whose inputs a path matches, ⊥ paths → all. 2 layers: scoped @ pre-commit, unscoped @ pre-push & CI ∵ too narrow a selection passes a stale block. ∀ declared input ! match the `dev-generated` glob; a test reads `hk.pkl` & fails on a gap.
V137: exits 0 clean · 1 stale, owner or marker error · 2 usage. a tool that cannot spawn → "MISSING TOOL, not a finding" + exit 1, ⊥ silent pass.

## §T TASKS

id|status|task|cites
T69|x|workspace + `dev/` crate; `readme` renders the `badges` block from owners; hk `dev-generated`/`-full` replace `coverage-badge`|V131,V133,V134,V135,V136,V137
T70|x|`notices` renders the `closure` block (count, table, licence tally) in `docs/THIRD-PARTY-NOTICES.md`; README closure badge from the same set|V131,V132,V134
T71|x|coverage over the workspace: `cargo llvm-cov --workspace`, `.coverage` key over `src dev` ∴ dev code sits under the floor|`.:V46`

## §B BUGS

id|date|cause|fix
