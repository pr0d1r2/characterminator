# SPEC

## §G GOAL

What a char set IS: builtin presets, membership, union, custom & composed sets.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- file: `.ctrm-sets` — custom sets, `<name> <member>...`: member = literal char | `U+XXXX` | `U+XXXX-U+YYYY` | set name (≥2 chars, V25) | `<family>:<member>` (V41).
- sets (builtin function presets, V23): `ascii` intrinsic (U+0020–U+007E + `\t` `\n`, `src/rules:V21`) · `caveman` (FORMAT.md symbols `→∴∀∃⊥≠∈∉≤≥§` + measured `⇒∵·×∨∧¬←↔⇔≈∅⊆∪∩↑↓`, R3) · `box` (U+2500–U+257F, U+25A0–U+25FF) · `marks` (labelled members, V41: `✓✗→` @`text`, `✅❌➡` @`emoji`, `⚠` unlabelled) · `math` (`×÷±°²³≤≥≠≈∞µ√∑π`) · `legal` (`©®™`) · `emoji` (single code points: U+1F300–U+1F3FA, U+1F400–U+1FAFF, U+2600–U+27BF; skin tone gap per V42; sequences compress, `src/fix:V31`) · letters per CLDR locale (`pl`, `de`, `fr`, …, + `<code>-aux`; 30 shipped, T32, V59) · `cr` (`\r`) · `typography` (`—–‘’“”…«»` + NBSP + `−`; grant ⇒ `src/fix:V26` map leaves them) · coarse blocks `latin1` (U+0080–U+00FF), `latin-ext` (U+0100–U+017F), `cyrillic`, `greek`, `arabic` · scripts w/ joiners `persian`, `hindi` (`src/lint:V57`) · `hazard` (`src/lint:V34`; ∈ `any` like any set — the FORBID level is what fires, ⊥ the set) · `any`. custom sets via `.ctrm-sets`.

## §R RESEARCH

id|topic|finding|src
R1|Rust corpus need|9 Rust repos (sibling dirs w/ `Cargo.toml`), 702 git-tracked text files: 65.1% pure ASCII (+ `\t` `\n`); `.rs` 77.7%, `.md` 15.0%. non-ASCII files (245): 44.5% need 1 extra char, 71.0% ≤3, 93.5% ≤10|scan 2026-09-12, repos anonymous
R2|Rust preset coverage|of 245 non-ASCII files: 12.7% fully fixed by typography map alone; + `ascii`+1 preset → 90.2%; +2 → 95.1%. `caveman` = 180 of the 1-preset fits|same scan
R3|caveman usage|Rust: 180 fit extended `caveman`, 123 fit FORMAT.md list ∴ misses 32%. fleet: 833 vs 521 ∴ 37%. extras, fleet files containing: `⇒` 222, `·` 168, `∵` 77|R1 scan + R4 scan
R4|fleet extension|227 repos, 73,114 text files: 95.1% pure ASCII. of 3,604 non-ASCII: typography map alone 45.1%; +1 preset → 84.9%; +2 → 89.2%. excl `.nix` (60% of files, 1 repo = 89% of its non-ASCII): 84.1% / 90.0%|fleet scan 2026-09-12, snapshots excluded, repos anonymous
R5|uncovered & pairs|Rust: `Σ` `≡` `⟺` `≪` `⊇` `⋃`, superscripts, CJK (wenyan-style caveman). fleet: `©` `®` `™`, Cyrillic, Arabic, `−` U+2212. top pair both corpora: `caveman`+`box` (Rust 8, fleet 42)|R1 scan + R4 scan
R6|CLDR letters|766 locales in `cldr-misc-full`. `exemplarCharacters` = letters in normal use (pl: 32, 9 non-ASCII); `auxiliary` = loan letters; `punctuation` incl. locale quotes (pl `„”`). Unicode License v3|github.com/unicode-org/cldr-json
R15|CLDR full cost|`locales.ctrm-sets` 12,876 → 176,760 B: 1,563 lines = 481 sets + 1,082 alias lines (766 locales + default-content codes). release binary 7,212,864 → 7,364,064 B (+151,200, +2.1%). `ctrm check` on this repo, 5 rounds × 30 runs: before 19.7–23.8 ms/run, after 20.4–23.2 ∴ delta within noise: lazy parse (V64) costs a run naming ⊥ locale nothing measurable|2026-10-02, macOS arm64, release build, loop timer (hyperfine ∉ dev shell)

## §V INVARIANTS

V3: sets compose by union only (`ascii+latin-ext`). ⊥ subtraction. effective set = union of winning rule's sets.
V22: builtin map & sets ship as data files in same grammar as user files, compiled in via `include_str!`, written in `U+XXXX` form only ∴ ASCII, `.:V13` holds w/ ⊥ grant.
V23: builtin sets = small function presets, sized from R2 & R4: typical file = `ascii` + 1 preset after map. coarse blocks (`latin1`, `latin-ext`, scripts) only for multi-language data (locales), ⊥ recommended for code | docs.
V25: set member ? names another set (name ≥2 chars; 1 char = literal) ∴ user presets = compositions (`spec caveman box marks`). cycle → error naming cycle, exit 2.
V30: language letter presets ∀ CLDR locale, named by locale code (`pl`, `pt-BR`): main `exemplarCharacters` − ASCII, + uppercase forms. data vendored (Unicode License v3, R6), generated into `U+XXXX` data file (V22), ⊥ fetched at runtime. `auxiliary` loan letters ship as `<code>-aux` from same vendoring ∴ 1 generator run, 2 presets.

V41: labelled set member `<family>:<member>`, notation ≡ `src/fix:V28`. unlabelled granted ∀ fidelity; labelled granted ⟺ label = resolved family (`src/rules:V29`). ⊥ family-tree walk here ∵ tree lives in the MAP (`src/fix:V27`) ∴ walking it = this node depending on `src/fix`. family = opaque NAME.
V42: preset range written so ⊥ code point another node WITHHOLDS falls inside (`src/fix:V31`: skin tone, VS, ZWJ, keycap, tag). V3 ⊥ subtraction ∴ gap ! be IN the range. §I = summary, data file SHIPS; test asserts ∀ exclusion.
V59: locale data (V30) = `locales.ctrm-sets`, generated by `cldr-letters.sh` from pinned cldr-json TAG + UCD `UnicodeData.txt` (simple uppercase); header records tag & ∀ input sha256; rerun over same inputs ≡ same bytes. `-aux` = aux + uppercase − main. multi-char exemplar (`{ch}`, base + combining mark) → its code points individually. ∀ locale set ∩ `hazard` = ∅ (subtracted at generation, test asserts). main empty after subtraction (all-ASCII, `en`) ⇒ alias line `<code> ascii` ∴ resolves, grants ASCII only; `-aux` empty ⇒ ⊥ set.
V61: FULL CLDR (T60) = ∀ locale in pinned `cldr-misc-full`, incl CJK, Indic, RTL, by the SAME generator (V59), extended: exemplar ranges (`[a-z]`, `[U+3041-U+3096]`) & `\uXXXX` escapes expanded; regional & script variants (`pt-BR`, `sr-Latn`, `zh-Hant`) follow the CLDR parent chain: emitted as a set ONLY when ≠ parent, else 1 ALIAS line naming the parent (`pt-BR pt`) ∴ ∀ CLDR code resolves, all-ASCII ones too (V59; test: `en` `en-US` `id` `ceb`) & ⊥ duplicate data. hazard ∩ = ∅ (V59). LAZY: locale file parsed only for the names a run's rules use ∴ a run naming ⊥ locale pays ⊥ parse; release binary size & `ctrm check` time on this repo, before vs after, recorded as an R row. per-locale `typography-<code>` stays `src/fix:V26`'s, ⊥ here.
V64: LAZY (V61) = `locales.ctrm-sets` ∉ eager builtin text (`SETS`), own compiled-in const. catalog MISS → line found by 1st token, ONLY it parsed, + names it composes in. alias's parent (`pt-BR pt`, `en ascii`) read from compiled-in data & inlined, ⊥ from catalog ∴ user set `pt` ⊥ stands in for `pt-BR`'s parent; ⊥ adds the parent's name. declared set of same name wins (only a miss looked up). rule-named locales adopted at catalog build ⟺ builtin sets on (`--no-builtin-sets` ⇒ ⊥ locale). `ctrm sets` lists ∀ locale. CLDR default-content codes (`pt-BR`, ⊥ file) = alias lines. test: run naming ⊥ locale adds ⊥ name.

## §T TASKS

id|status|task|cites
T5|x|ARCHIVED to SPEC-ARCHIVE.md|V3,`.:I.file`
T22|x|ARCHIVED to SPEC-ARCHIVE.md|V22,`.:V13`
T23|x|ARCHIVED to SPEC-ARCHIVE.md|V22,V23
T25|x|ARCHIVED to SPEC-ARCHIVE.md|V25
T32|x|ARCHIVED to SPEC-ARCHIVE.md|V30,V22,V59
T46|x|ARCHIVED to SPEC-ARCHIVE.md|V41,`src/rules:V29`
T60|x|ARCHIVED to SPEC-ARCHIVE.md|V61,V59,V30

## §B BUGS

id|date|cause|fix
B2|2026-09-20|§I stated `emoji` unbroken U+1F300–U+1FAFF; skin tone U+1F3FB–U+1F3FF sit INSIDE & `src/fix:V31` withholds them ∴ data file from §I granted them|V42
B32|2026-10-02|V59 dropped a locale w/ ⊥ non-ASCII letter ∴ 205 locales + 63 default-content codes (`en`, `en-US`, `id`, `ceb`) → "no set named", against V61|V59
B33|2026-10-02|alias line resolved its parent through the run's catalog ∴ `--set 'pt U+0161' --rule '* ascii+pt-BR'` granted U+0161 & ⊥ U+00E7 under `pt-BR`|V64
