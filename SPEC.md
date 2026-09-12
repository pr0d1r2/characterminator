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
- bin name `characterminator` ? short alias (`rg`/`mth` shape) TBD.
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
- cmd: `characterminator explain <path>` → effective set + winning rule + its config line.
- cmd: `characterminator sets` → builtin sets & members.
- flag: `--format human|json` ∀ verbs · `-C <dir>`.
- file: `.characterminator` ? — line-based, `<glob|path> <set>[+<set>...]`, `#` comment. zero-dep parse (`.context-limits` shape).
- file: `.characterminator-map` ? — transliteration, `<from> <to>`: `from` = literal char | `U+XXXX`; `to` = replacement, empty = explicit delete. `to` ∉ target file's set → char counts unmapped (V4).
- file: `.characterminator-sets` ? — custom sets, `<name> <member>...`: member = literal char | `U+XXXX` | `U+XXXX-U+YYYY`.
- flag (file twins, repeatable, V18): `--rule <line>` · `--map <line>` · `--set <line>` · `--rules-file <f>` · `--map-file <f>` · `--sets-file <f>` · `--no-files` (skip discovered dotfiles) · `--no-builtin-map` · `--no-builtin-sets`. names ?.
- sets (builtin function presets, V23): `ascii` intrinsic (U+0020–U+007E + `\t` `\n`, V21) · `caveman` (FORMAT.md symbols `→∴∀∃⊥≠∈∉≤≥§`) · `box` (U+2500–U+257F, U+25A0–U+25FF) · `marks` (`✓✔✗✘✅❌⚠➜` + VS16) · `math` (`×÷±°²³≤≥≠≈∞µ√∑π`) · `legal` (`©®™`) · `emoji` (U+1F300–U+1FAFF, U+2600–U+27BF, VS16, ZWJ) · letters `pl` `de`, more ? · `cr` (`\r`) · coarse blocks `latin1` (U+0080–U+00FF), `latin-ext` (U+0100–U+017F), `cyrillic`, `greek`, `arabic` · `any`. custom sets via `.characterminator-sets`.
- lib: `characterminator::{scan, fix, resolve}` — pure fn over `&str`.
- exit: 0 ok · 1 violation | drift · 2 usage.

## §R RESEARCH

id|topic|finding|src
R1|Rust corpus need|9 Rust repos (sibling dirs w/ `Cargo.toml`), 702 git-tracked text files: 65.1% pure ASCII (+ `\t` `\n`); `.rs` 77.7%, `.md` 15.0%. non-ASCII files (245): 44.5% need 1 extra char, 71.0% ≤3, 93.5% ≤10|scan 2026-09-12, repos anonymous
R2|Rust preset coverage|of 245 non-ASCII files: 12.7% fully fixed by typography map alone; + `ascii`+1 preset → 90.2%; +2 → 95.1%. `caveman` = 180 of the 1-preset fits|same scan
R3|caveman usage|Rust: 180 fit extended `caveman`, 123 fit FORMAT.md list ∴ misses 32%. fleet: 833 vs 521 ∴ 37%. extras, fleet files containing: `⇒` 222, `·` 168, `∵` 77|R1 scan + R4 scan
R4|fleet extension|227 repos, 73,114 text files: 95.1% pure ASCII. of 3,604 non-ASCII: typography map alone 45.1%; +1 preset → 84.9%; +2 → 89.2%. excl `.nix` (60% of files, 1 repo = 89% of its non-ASCII): 84.1% / 90.0%|fleet scan 2026-09-12, snapshots excluded, repos anonymous
R5|uncovered & pairs|Rust: `Σ` `≡` `⟺` `≪` `⊇` `⋃`, superscripts, CJK (wenyan-style caveman). fleet: `©` `®` `™`, Cyrillic, Arabic, `−` U+2212. top pair both corpora: `caveman`+`box` (Rust 8, fleet 42)|R1 scan + R4 scan

## §V INVARIANTS

V1: path w/ no matching rule → `ascii`. strict default; extended set = explicit grant. ≠ `itok`'s opt-in `.context-limits`: here an unguarded char IS the cost.
V2: rule resolution: later matching line wins (gitignore semantics); per-type glob & per-file path share one grammar ∴ per-file line placed after per-type line overrides it. `explain` ! print winner.
V3: sets compose by union only (`ascii+latin-ext`). ⊥ subtraction. effective set = union of winning rule's sets.
V4: `fix` replaces only via declared transliteration map. char w/o mapping → kept & reported, exit 1. ⊥ silent drop.
V5: `fix` idempotent: `fix(fix(x)) == fix(x)`, property-tested.
V6: `fix` touches ⊥ allowed char: bytes outside violations ! identical pre/post, asserted before write.
V7: only `check` & `fix --check` gate. bare `fix` rewrites only on explicit call. `stats`/`explain`/`sets` report-only, exit 0.
V8: invalid UTF-8 → error naming path & byte offset, exit 1; ⊥ lossy decode. binary file (NUL byte ?) → skipped & named in report, ⊥ silent.
V9: default fileset = git-tracked (`itok::walk::tracked`); explicit paths reach untracked.
V10: token figure self-describes unit & method, per `itok`: `~` = bytes/4 estimate, `(o200k)` = `--bpe`. ⊥ claim measurement it cannot make.
V11: `--format json` = stable contract; human output cosmetic.
V12: violation position = 1-based line + col (chars) + byte offset + `U+XXXX`. output sorted by path, then offset.
V13: dogfood: `characterminator check` gates own tree in `hk.pkl`; `.characterminator` grants `SPEC.md` `ascii+caveman`, rest `ascii`.
V14: SPEC.md form gated by `mth fmt --check SPEC.md` & `mth check --records .spec-records SPEC.md`. `mth` absent → gate FAILS hard, ⊥ skip.
V15: SPEC.md capped from commit one: `.context-limits` row gated by `itok check`; ceiling ~12% over measured.
V16: `sherd check` & `sherd budget` gate once ≥1 child node exists.
V17: locale separation: extended sets granted to data paths (`locales/**`, `*.po`, `config/locales/*.yml`); code stays `ascii`. non-ASCII string literal in code file → hint: move to locale file ?.
V18: ∀ data-file line kind → flag twin: `--rule` ≡ rules line, `--map` ≡ map line, `--set` ≡ sets line. flag value = exactly 1 line, same parser ∴ ∀ file F: `--no-files` + 1 flag per line of F ≡ F, property-tested.
V19: precedence, low → high: builtin → discovered dotfiles → `--*-file` (argv order) → inline flags (argv order). later wins: rule per V2, map entry per char, set per name.
V20: ∀ effective rule, map entry, set → origin (`<file>:<line>` | `argv[<n>]` | `builtin:<line>`). `explain` ! print it. `explain --as-args` ? → effective config as flags (round trip).
V21: zero-file run: `--no-files --no-builtin-map --no-builtin-sets` → config from argv only. `ascii` intrinsic (code, ⊥ data) ∴ V1 holds w/ ⊥ file.
V22: builtin map & sets ship as data files in same grammar as user files, compiled in via `include_str!`, written in `U+XXXX` form only ∴ ASCII, V13 holds w/ ⊥ grant. path follows node layout ?.
V23: builtin sets = small function presets, sized from R2 & R4: typical file = `ascii` + 1 preset after map. coarse blocks (`latin1`, `latin-ext`, scripts) only for multi-language data (locales), ⊥ recommended for code | docs.

## §T TASKS

id|status|task|cites
T1|.|scaffold crate: `Cargo.toml` edition 2024, MSRV 1.95, lints, lib+bin, `rustfmt.toml`, `clippy.toml`|-
T2|.|`flake.nix` dev shell: `nixpkgs-lock`, `nix-hk`, `microlith`, `itok`, `sherd` inputs, ∀ following `nixpkgs-lock`|V14,V15,V16
T3|.|`hk.pkl` gate: fmt, clippy `-D warnings`, test, `mth`, `mth-check`, `itok check`; vendor `pkl/Config.pkl`|V14,V15
T4|.|`.context-limits` SPEC.md ceiling; `.spec-records` baseline|V15
T5|.|charset model: builtin sets, union compose, custom ranges|V3,I.file
T6|.|`.characterminator` parse & rule resolution, last match wins|V1,V2
T7|.|scan core over `&str`: positions, UTF-8 errors, binary skip|V8,V12
T8|.|`check` verb + fileset via `itok::walk::tracked`|V7,V9,I.cmd
T9|.|`--format json` ∀ verbs|V11
T10|.|transliteration map + `fix` & `fix --check`; property tests: idempotency, untouched bytes|V4,V5,V6
T11|.|`stats` verb w/ `itok` counts now vs after fix|V10
T12|.|`explain` & `sets` verbs|V2,V7
T13|.|dogfood: `.characterminator` for own tree, `check` step in `hk.pkl`|V13
T14|.|`sherd check` + `sherd budget` in `hk.pkl` when first child node lands|V16
T15|.|locale hint on non-ASCII literal in code file ?|V17
T16|.|README, AGENTS.md, `docs/LLM-DISCLAIMER.md`, `docs/THIRD-PARTY-NOTICES.md` per fleet|-
T17|.|release: `release.toml` (`cargo-release`), crates.io publish ?|-
T18|.|one line parser per kind (rules, map, sets); flag twins feed same parser|V18,I.flag
T19|.|property test: file ≡ `--no-files` + flag sequence, ∀ kinds|V18
T20|.|config assembly: precedence chain, origin per entry, `explain` prints origin|V19,V20
T21|.|zero-file mode; `ascii` as intrinsic constant|V21,V1
T22|.|builtin map & sets as `U+XXXX` data files via `include_str!`|V22,V13
T23|.|preset data files per V23, contents from R2 & R4|V22,V23

## §B BUGS

id|date|cause|fix
